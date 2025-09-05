pub mod errors;
pub mod javascript;
pub mod view;

#[cfg(feature = "sessions")]
pub mod session;

#[cfg(feature = "sessions")]
pub use session::Session;

#[cfg(feature = "turbo-streams")]
pub mod turbo;

#[cfg(feature = "middleware")]
pub mod middleware;

static SITE_ROOT: std::sync::Mutex<Option<String>> = std::sync::Mutex::new(None);

/// Sets the ROOT URL to use when building links to assets
pub fn set_app_root(root: impl Into<String>) {
    let root: String = root.into();
    let mut lock = SITE_ROOT.lock().unwrap();
    *lock = Some(root);
}

/// The root of the site used when building link to assets
pub fn app_root() -> String {
    let lock = SITE_ROOT.lock().unwrap();
    match lock.as_deref() {
        Some(r) => r.to_owned(),
        None => "/".to_owned(),
    }
}
