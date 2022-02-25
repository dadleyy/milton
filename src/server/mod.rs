use std::io::{Error, ErrorKind, Result};

use async_std::channel::Sender;
use tide::{http::Cookie, Request};

use crate::{auth_zero, heartbeat::HeartControl};

pub mod routes;
pub(crate) mod sec;

pub enum Authority {
  Admin,
}

#[derive(Default, Clone)]
pub struct StateBuilder {
  sender: Option<Sender<blinkrs::Message>>,
  heart: Option<Sender<HeartControl>>,
  oauth: Option<auth_zero::AuthZeroConfig>,
  redis: Option<crate::redis::RedisConfig>,
}

impl StateBuilder {
  pub fn redis(mut self, config: crate::redis::RedisConfig) -> Self {
    self.redis = Some(config);
    self
  }

  pub fn heart(mut self, chan: Sender<HeartControl>) -> Self {
    self.heart = Some(chan);
    self
  }

  pub fn oauth(mut self, conf: auth_zero::AuthZeroConfig) -> Self {
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
    Ok(State {
      sender,
      heart,
      oauth,
      redis: self.redis.ok_or(Error::new(ErrorKind::Other, "missing-redis"))?,
    })
  }
}

#[derive(Clone)]
pub struct State {
  sender: Sender<blinkrs::Message>,
  heart: Sender<HeartControl>,
  oauth: auth_zero::AuthZeroConfig,
  redis: crate::redis::RedisConfig,
}

impl State {
  pub fn builder() -> StateBuilder {
    StateBuilder::default()
  }

  pub async fn command<T, V>(&self, cmd: &kramer::Command<T, V>) -> Result<()>
  where
    T: std::fmt::Display,
    V: std::fmt::Display,
  {
    self.redis.send(&cmd).await.map(|_| ())
  }

  pub async fn authority<T>(&self, id: T) -> Option<Authority>
  where
    T: std::fmt::Display,
  {
    let oauth = self.oauth();
    let roles = oauth.fetch_user_roles(id).await.ok()?;
    roles.into_iter().find(|role| role.is_admin()).map(|_| Authority::Admin)
  }

  #[inline]
  pub fn oauth(&self) -> auth_zero::AuthZeroConfig {
    self.oauth.clone()
  }
}

#[inline]
pub fn cookie(request: &Request<State>) -> Option<Cookie<'static>> {
  request
    .header("Cookie")
    .and_then(|list| list.get(0))
    .map(|value| value.to_string())
    .and_then(|cook| Cookie::parse(cook).ok())
}
