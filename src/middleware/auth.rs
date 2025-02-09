use crate::util::{
    auth::{decode_jwt, PrivateClaim},
    errors::ApiError,
};
use actix_identity::RequestIdentity;
use actix_service::{Service, Transform};
use actix_web::{
    dev::{ServiceRequest, ServiceResponse},
    Error, HttpResponse,
};
use futures::{
    future::{ok, Ready},
    Future,
};
use std::pin::Pin;
use std::task::{Context, Poll};

pub struct Auth;

impl<S: Service<Req, Response = ServiceResponse, Error=Error>, Req> Transform<S, Req> for Auth
where
    S::Future: 'static,
{
    // type Request = ServiceRequest;
    type Response = S::Response;//ServiceResponse<B>;
    type Error = Error;
    type InitError = ();
    type Transform = AuthMiddleware<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ok(AuthMiddleware { service })
    }
}
pub struct AuthMiddleware<S> {
    service: S,
}

impl<S: Service<Req, Response = ServiceResponse, Error=Error>, Req> Service<Req> for AuthMiddleware<S>
where
    S::Future: 'static,
{
    type Response = S::Response;
    type Error = Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>>>>;

    fn poll_ready(&mut self, cx: &mut Context) -> Poll<Result<(), Self::Error>> {
        self.service.poll_ready(cx)
    }

    fn call(&mut self, req: ServiceRequest) -> Self::Future {
        let identity = RequestIdentity::get_identity(&req).unwrap_or("".into());
        let private_claim: Result<PrivateClaim, ApiError> = decode_jwt(&identity);
        let is_logged_in = private_claim.is_ok();
        let unauthorized = !is_logged_in && !req.path().ends_with("/login");

        if unauthorized {
            return Box::pin(async move {
                Ok(req.into_response(
                    HttpResponse::Unauthorized()
                        .body("Unauthorized - you must login!")
                        .into_body(),
                ))
            });
        }

        let fut = self.service.call(req);

        Box::pin(async move {
            let res = fut.await?;
            Ok(res)
        })
    }
}
