use std::{
    future::{ready, Future, Ready},
    pin::Pin,
};

use actix_web::body::BoxBody;

use actix_web::{
    dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform},
    http, Error, HttpResponse,
};

/// Middleware that strips a base path (e.g. "/myapp") from incoming requests
pub struct StripPathPrefix {
    prefix: String,
}

impl StripPathPrefix {
    pub fn new(prefix: impl Into<String>) -> Self {
        let mut prefix = prefix.into();
        // Ensure leading slash, no trailing slash
        if !prefix.starts_with('/') {
            prefix.insert(0, '/');
        }
        if prefix.len() > 1 && prefix.ends_with('/') {
            prefix.pop();
        }
        Self { prefix }
    }
}

// `S` - type of the next service
// `B` - type of response's body
impl<S, B> Transform<S, ServiceRequest> for StripPathPrefix
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: actix_web::body::MessageBody,
    B: 'static,
{
    type Response = ServiceResponse<BoxBody>;
    type Error = Error;
    type InitError = ();
    type Transform = StripPathPrefixMiddleware<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(StripPathPrefixMiddleware {
            service,
            prefix: self.prefix.clone(),
        }))
    }
}

pub struct StripPathPrefixMiddleware<S> {
    /// The next service to call
    service: S,
    prefix: String,
}

// This future doesn't have the requirement of being `Send`.
// See: futures_util::future::LocalBoxFuture
type LocalBoxFuture<T> = Pin<Box<dyn Future<Output = T> + 'static>>;

// `S`: type of the wrapped service
// `B`: type of the body - try to be generic over the body where possible
impl<S, B> Service<ServiceRequest> for StripPathPrefixMiddleware<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static,
    B: actix_web::body::MessageBody,
{
    type Response = ServiceResponse<BoxBody>;
    //type Response = ServiceResponse<B>;
    type Error = Error;
    type Future = LocalBoxFuture<Result<Self::Response, Self::Error>>;

    // This service is ready when its next service is ready
    forward_ready!(service);

    fn call(&self, mut req: ServiceRequest) -> Self::Future {
        let prefix = self.prefix.clone();

        // modify the request to remove the prefix
        if let Some(path) = req.uri().path_and_query() {
            let old_uri = req.uri();
            if let Some(stripped) = path.as_str().strip_prefix(&prefix) {
                // Make sure empty → "/" so it routes correctly
                let new_path = if stripped.is_empty() { "/" } else { stripped };

                // Rebuild URI with the new path
                let mut builder = http::Uri::builder();

                // copy over the old bits to the new URI
                if let Some(auth) = old_uri.authority() {
                    builder = builder.authority(auth.clone());
                }
                if let Some(s) = old_uri.scheme() {
                    builder = builder.scheme(s.clone());
                }
                builder = builder.path_and_query(new_path);
                let new_uri = builder.build().unwrap();

                let head = req.head_mut();
                head.uri = new_uri.clone();
                // re-match on the new URI
                req.match_info_mut().get_mut().update(&new_uri);

                let fut = self.service.call(req);
                return Box::pin(async move {
                    let res = fut.await?;
                    Ok(res.map_into_boxed_body())
                });
            }
        }

        // No match, return 404
        Box::pin(async move {
            let (req, _pl) = req.into_parts();
            let res = HttpResponse::NotFound().finish();
            Ok(ServiceResponse::new(req, res))
        })
    }
}
