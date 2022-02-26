use super::State;
use tide::{http::Cookie, Request};

#[inline]
pub fn cookie(request: &Request<State>) -> Option<Cookie<'static>> {
  request
    .header("Cookie")
    .and_then(|list| list.get(0))
    .map(|value| value.to_string())
    .and_then(|cook| Cookie::parse(cook).ok())
}
