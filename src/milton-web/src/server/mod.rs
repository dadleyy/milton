use std::io::{Error, ErrorKind, Result};

use async_std::channel::Sender;
use serde::Deserialize;
use tide::{http::Cookie, Request, Response};

use crate::oauth;

mod sec;

pub mod auth;
pub mod control;
pub mod effects;
pub mod webhook;

pub enum Authority {
  Admin,
}

#[derive(Deserialize, Clone, Debug)]
pub struct MiltonUIConfiguration {
  octoprint_api_url: String,
  octoprint_api_key: String,
  octoprint_snapshot_url: String,
  auth_complete_uri: String,
  jwt_secret: String,
}

#[derive(Default, Clone)]
pub struct StateBuilder {
  sender: Option<Sender<effects::Effects>>,
  oauth: Option<oauth::AuthZeroConfig>,
  ui_config: Option<MiltonUIConfiguration>,
}

impl StateBuilder {
  pub fn oauth(mut self, conf: oauth::AuthZeroConfig) -> Self {
    self.oauth = Some(conf);
    self
  }

  pub fn ui_config(mut self, ui_config: MiltonUIConfiguration) -> Self {
    self.ui_config = Some(ui_config);
    self
  }

  pub fn sender(mut self, chan: Sender<effects::Effects>) -> Self {
    self.sender = Some(chan);
    self
  }

  pub fn build(self) -> Result<State> {
    let sender = self
      .sender
      .ok_or_else(|| Error::new(ErrorKind::Other, "missing sender"))?;
    let oauth = self
      .oauth
      .ok_or_else(|| Error::new(ErrorKind::Other, "missing oauth config"))?;
    let ui_config = self
      .ui_config
      .ok_or_else(|| Error::new(ErrorKind::NotFound, "no ui config found"))?;
    Ok(State {
      sender,
      oauth,
      ui_config,
    })
  }
}

#[derive(Clone)]
pub struct State {
  sender: Sender<effects::Effects>,
  oauth: oauth::AuthZeroConfig,
  ui_config: MiltonUIConfiguration,
}

impl State {
  pub fn builder() -> StateBuilder {
    StateBuilder::default()
  }

  pub async fn authority<T>(&self, id: T) -> Option<Authority>
  where
    T: std::fmt::Display,
  {
    let oauth = self.oauth();
    let roles = oauth.fetch_user_roles(id).await.ok()?;
    if roles.into_iter().any(|role| role.is_admin()) {
      return Some(Authority::Admin);
    }
    None
  }

  pub async fn send(&self, effect: effects::Effects) -> Result<()> {
    self
      .sender
      .send(effect)
      .await
      .map_err(|error| Error::new(ErrorKind::Other, error))
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

pub fn claims(request: &Request<State>) -> Option<sec::Claims> {
  let cook = cookie(request)?;
  sec::Claims::decode(&cook.value(), &request.state().ui_config.jwt_secret).ok()
}

pub async fn missing(req: Request<State>) -> tide::Result {
  log::warn!("[warning] unknown request received - '{}'", req.url().path());
  Ok(Response::builder(404).build())
}

pub async fn listen<S>(state: State, addr: S) -> std::io::Result<()>
where
  S: std::convert::AsRef<str>,
{
  let mut app = tide::with_state(state);
  app.at("/incoming-webhook").post(webhook::hook);

  app.at("/control").post(control::command);
  app.at("/control").get(control::query);
  app.at("/control/snapshot").get(control::snapshot);

  app.at("/auth/start").get(auth::start);
  app.at("/auth/end").get(auth::end);
  app.at("/auth/complete").get(auth::complete);
  app.at("/auth/identify").get(auth::identify);

  app.at("/*").all(missing);
  app.listen(addr.as_ref()).await
}
