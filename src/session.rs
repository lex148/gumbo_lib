use actix_web::error::ErrorUnauthorized;
use actix_web::http::Method;
use actix_web::FromRequest;
use aes_gcm::{
    aead::{Aead, AeadCore, KeyInit, OsRng},
    Aes256Gcm, Key, Nonce,
};
use base64::prelude::*;
use rand::distr::Alphanumeric;
use rand::Rng;
use rkyv::{deserialize, Archive, Deserialize, Serialize};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use yew::html;
use yew::virtual_dom::vnode::VNode;

/// An Active Users Session
/// If you want to store info about this user You should go make a user table/model
/// The Sub can be used to uniquely Identity them.

#[derive(Archive, Deserialize, Serialize, Debug, PartialEq)]
#[rkyv(
    // This will generate a PartialEq impl between our unarchived
    // and archived types
    compare(PartialEq),
    // Derives can be passed through to the generated type:
    derive(Debug),
)]
pub struct Session {
    // A unique identifier for the given user
    sub: String,
    // unix timestamp (sec) when this session will expire
    exp: i64,
    // The expected csrf_token for this given session
    csrf_token: String,
}

/// An Active Users Session that does NOT verify a csrf-token
pub struct SessionUnsafe(Session);
impl SessionUnsafe {
    pub fn into_inner(self) -> Session {
        self.0
    }
}

impl Session {
    pub fn sub(&self) -> &str {
        &self.sub
    }

    /// This is called when a user is logged in
    pub fn build(sub: impl Into<String>) -> Session {
        let csrf_token: String = rand::rng()
            .sample_iter(&Alphanumeric)
            .take(32)
            .map(char::from)
            .collect();
        Session {
            sub: sub.into(),
            csrf_token,
            exp: next_exp_time(),
        }
    }

    pub fn as_encrypted(&self) -> String {
        let key_bytes = auth_key();
        let key = Key::<Aes256Gcm>::from_slice(&key_bytes);
        let cipher = Aes256Gcm::new(key);
        // Generate a random nonce

        let nonce = Aes256Gcm::generate_nonce(&mut OsRng);
        let noncebytes: &[u8] = nonce.as_slice();

        // generate an encrypt string of this struct
        let serialized =
            rkyv::to_bytes::<rkyv::rancor::Error>(self).expect("Session Serialization Failed");

        let cipherbytes: Vec<u8> = cipher
            .encrypt(&nonce, serialized.as_ref())
            .expect("Serialization failed");

        let allbytes: Vec<u8> = noncebytes
            .iter()
            .chain(cipherbytes.iter())
            .cloned()
            .collect();

        BASE64_STANDARD.encode(&allbytes)
    }

    fn from_encrypted(encrypted_bytes: &[u8]) -> Result<Session, actix_web::Error> {
        if encrypted_bytes.len() <= 12 {
            return Err(ErrorUnauthorized(""));
        }
        let (noncebytes, contents) = encrypted_bytes.split_at(12);
        let nonce = Nonce::from_slice(noncebytes);
        let key_bytes = auth_key();
        let key = Key::<Aes256Gcm>::from_slice(&key_bytes);
        let cipher = Aes256Gcm::new(key);

        let bytes = cipher
            .decrypt(nonce, contents)
            .or(Err(ErrorUnauthorized("")))?;

        let archived = rkyv::access::<ArchivedSession, rkyv::rancor::Error>(&bytes)
            .or(Err(ErrorUnauthorized("")))?;

        let session =
            deserialize::<Session, rkyv::rancor::Error>(archived).or(Err(ErrorUnauthorized("")))?;

        Ok(session)
    }

    /// Add this to the top of your html page.
    pub fn meta_csrf_token(&self) -> VNode {
        html! {
            <meta name="csrf-token" content={ self.csrf_token.clone() } />
        }
    }
}

/// Panics if the AUTH_SECRET is not set or is invalid.
/// used at boot to make sure the app is setup
pub fn verify_auth_key() {
    let _ = auth_key();
}

/// Panics if the AUTH_SECRET is not set or is invalid.
/// used at boot to make sure the app is setup
fn auth_key() -> Vec<u8> {
    use base64::prelude::*;
    let key_base64 =
        std::env::var("AUTH_SECRET").expect("\n\nAUTH_SECRET env not set. expected a AES_256_KEY\nYou can generate an AUTH_SECRET for your gumbo project to use by running the command:\ngumbo generate env\n\n");
    let key_bytes = BASE64_STANDARD
        .decode(key_base64)
        .expect("\nFailed to read env AUTH_SECRET. expected a AES_256_KEY\nYou can generate an AUTH_SECRET for your gumbo project to use by running the command:\ngumbo generate env\n\n");
    assert_eq!(key_bytes.len(), 32, "Key must be 256 bits (32 bytes)");
    key_bytes
}

/// returns the time now
fn now_sec() -> i64 {
    let now = SystemTime::now();
    now.duration_since(UNIX_EPOCH)
        .expect("Time went backwards")
        .as_secs() as i64
}

/// return the time one hour from now
fn next_exp_time() -> i64 {
    let now = SystemTime::now();
    now.checked_add(Duration::new(60 * 60 * 24, 0)) //24 hours from now
        .expect("time overflowed")
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards")
        .as_secs() as i64
}

use actix_web::dev::Payload;
use actix_web::HttpRequest;
use futures::future::LocalBoxFuture;

/// Allows you to request a Session from an actix resource
impl FromRequest for Session {
    type Error = actix_web::Error;
    type Future = LocalBoxFuture<'static, std::result::Result<Self, Self::Error>>;
    fn from_request(req: &HttpRequest, _payload: &mut Payload) -> Self::Future {
        let req_clone = req.clone();
        Box::pin(async move { load_session(&req_clone).await })
    }
}

/// loads a session the AuthCookie.
async fn load_session(req: &HttpRequest) -> std::result::Result<Session, actix_web::Error> {
    log::debug!("load_session");
    let auth_cookie = req.cookie("_session").ok_or(ErrorUnauthorized(""))?;
    let encrypted_base64 = auth_cookie.value().to_string();
    let encrypted_bytes = BASE64_STANDARD
        .decode(&encrypted_base64)
        .or(Err(ErrorUnauthorized("")))?;
    let session = Session::from_encrypted(&encrypted_bytes).or(Err(ErrorUnauthorized("")))?;
    if session.exp < now_sec() {
        log::debug!("load_session::expected");
        return Err(ErrorUnauthorized(""));
    }

    // For Non-GETs, make sure the csrf_token matches what is expected
    if req.method() != Method::GET {
        log::debug!("load_session::verifying csrf-token");
        let headers = req.headers();
        let token = headers.get("X-CSRF-Token").ok_or(ErrorUnauthorized(""))?;
        let token = token.to_str().or(Err(ErrorUnauthorized("")))?;
        if token != session.csrf_token {
            log::debug!("load_session::token mismatch");
            return Err(ErrorUnauthorized(""));
        }
    }

    Ok(session)
}

/// Allows you to request a Session from an actix resource
impl FromRequest for SessionUnsafe {
    type Error = actix_web::Error;
    type Future = LocalBoxFuture<'static, std::result::Result<Self, Self::Error>>;
    fn from_request(req: &HttpRequest, _payload: &mut Payload) -> Self::Future {
        let req_clone = req.clone();
        Box::pin(async move { load_session_unsafe(&req_clone).await })
    }
}

/// loads a session the AuthCookie.
async fn load_session_unsafe(
    req: &HttpRequest,
) -> std::result::Result<SessionUnsafe, actix_web::Error> {
    log::debug!("load_session");
    let auth_cookie = req.cookie("_session").ok_or(ErrorUnauthorized(""))?;
    let encrypted_base64 = auth_cookie.value().to_string();
    let encrypted_bytes = BASE64_STANDARD
        .decode(&encrypted_base64)
        .or(Err(ErrorUnauthorized("")))?;
    let session = Session::from_encrypted(&encrypted_bytes).or(Err(ErrorUnauthorized("")))?;
    if session.exp < now_sec() {
        log::debug!("load_session::expected");
        return Err(ErrorUnauthorized(""));
    }
    // NOTE: not verifying the csrf_token
    Ok(SessionUnsafe(session))
}
