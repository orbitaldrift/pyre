use std::{
    sync::Arc,
    task::{Context, Poll},
};

use futures_util::future::BoxFuture;
use http::{Method, Request, Response};
use tower_cookies::Cookies;
use tower_layer::Layer;
use tower_service::Service;

use crate::{error::Error, token::validate_token, Config};

#[derive(Clone, Default)]
pub struct Guard;

impl<S> Layer<S> for Guard {
    type Service = GuardService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        GuardService { inner }
    }
}

#[derive(Clone)]
pub struct GuardService<S> {
    inner: S,
}

impl<S> GuardService<S> {
    pub(crate) fn new(inner: S) -> Self {
        Self { inner }
    }
}

impl<S, Q, R> Service<Request<Q>> for GuardService<S>
where
    S: Service<Request<Q>, Response = Response<R>> + Send + 'static,
    S::Future: Send + 'static,
    Q: Send + 'static,
    R: Default + Send,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, request: Request<Q>) -> Self::Future {
        if ![Method::POST, Method::PUT, Method::PATCH, Method::DELETE].contains(request.method()) {
            return Box::pin(self.inner.call(request));
        }

        // TODO: if no cookies, and no session, use random UUID
        // if cookies, session ext, then use session id
        // this will remove need for calling .set manually.

        let config = request.extensions().get::<Arc<Config>>().cloned();
        let cookies = request.extensions().get::<Cookies>().cloned();
        let header_value = config
            .as_ref()
            .and_then(|c| {
                request
                    .headers()
                    .get(&c.header_name)
                    .and_then(|h| h.to_str().ok())
            })
            .map(std::string::ToString::to_string);

        let future = self.inner.call(request);

        Box::pin(async move {
            let response = future.await?;

            let config = match config.ok_or(Error::ExtensionNotFound("Config".into())) {
                Ok(config) => config,
                Err(err) => return Ok(Error::make_layer_error(err)),
            };

            let cookies = match cookies.ok_or(Error::ExtensionNotFound("Cookies".into())) {
                Ok(cookies) => cookies,
                Err(err) => return Ok(Error::make_layer_error(err)),
            };

            let Some(cookie_value) = cookies
                .get(&config.cookie_name())
                .map(|c| c.value().to_owned())
            else {
                return Ok(Error::make_layer_forbidden());
            };

            let Some(header_value) = header_value else {
                return Ok(Error::make_layer_forbidden());
            };

            match validate_token(&config.secret, &cookie_value, &header_value) {
                Ok(valid) => {
                    if valid {
                        Ok(response)
                    } else {
                        Ok(Error::make_layer_forbidden())
                    }
                }
                Err(err) => Ok(Error::make_layer_error(err)),
            }
        })
    }
}
