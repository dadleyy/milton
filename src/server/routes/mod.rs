use tide::{Request, Response, Result};

use super::State;

pub mod auth;
pub mod control;
pub mod webhook;

pub async fn missing(req: Request<State>) -> Result {
  log::warn!("[warning] unknown request received - '{}'", req.url().path());
  Ok(Response::builder(404).build())
}
