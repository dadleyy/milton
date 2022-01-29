use std::io::{Error, ErrorKind, Result};

use async_std::channel::Sender;
use tide::{http::Cookie, Request, Response};

use crate::{heartbeat::HeartControl, oauth};

pub mod auth;
pub mod control;
pub mod sec;
pub mod webhook;

#[derive(Default, Clone)]
pub struct StateBuilder {
  sender: Option<Sender<blinkrs::Message>>,
  heart: Option<Sender<HeartControl>>,
  oauth: Option<oauth::AuthZeroConfig>,
}

impl StateBuilder {
  pub fn heart(mut self, chan: Sender<HeartControl>) -> Self {
    self.heart = Some(chan);
    self
  }

  pub fn oauth(mut self, conf: oauth::AuthZeroConfig) -> Self {
    self.oauth = Some(conf);
    self
  }

  pub fn sender(mut self, chan: Sender<blinkrs::Message>) -> Self {
    self.sender = Some(chan);
    self
  }

  pub fn build(self) -> Result<State> {
    let sender = self.sender.ok_or(Error::new(ErrorKind::Other, "missing sender"))?;
    let heart = self.heart.ok_or(Error::new(ErrorKind::Other, "missing heart"))?;
    let oauth = self.oauth.ok_or(Error::new(ErrorKind::Other, "missing oauth config"))?;
    Ok(State { sender, heart, oauth })
  }
}

#[derive(Clone)]
pub struct State {
  sender: Sender<blinkrs::Message>,
  heart: Sender<HeartControl>,
  oauth: oauth::AuthZeroConfig,
}

impl State {
  pub fn builder() -> StateBuilder {
    StateBuilder::default()
  }

  pub fn oauth(&self) -> oauth::AuthZeroConfig {
    self.oauth.clone()
  }
}

pub fn cookie(request: &Request<State>) -> Option<Cookie<'static>> {
  request
    .header("Cookie")
    .and_then(|list| list.get(0))
    .map(|value| value.to_string())
    .and_then(|cook| Cookie::parse(cook).ok())
}

pub async fn missing(req: Request<State>) -> tide::Result {
  log::warn!("[warning] unknown request received - '{}'", req.url().path());
  Ok(Response::builder(404).build())
}
