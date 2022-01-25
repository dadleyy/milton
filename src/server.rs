use std::io::{Error, ErrorKind, Result};

use async_std::channel::Sender;
use serde::{Deserialize, Serialize};
use tide::{Request, Response};

use crate::heartbeat::HeartControl;

#[derive(Default)]
pub struct StateBuilder {
  sender: Option<Sender<blinkrs::Message>>,
  heart: Option<Sender<HeartControl>>,
}

impl StateBuilder {
  pub fn heart(mut self, chan: Sender<HeartControl>) -> Self {
    self.heart = Some(chan);
    self
  }

  pub fn sender(mut self, chan: Sender<blinkrs::Message>) -> Self {
    self.sender = Some(chan);
    self
  }

  pub fn build(self) -> Result<State> {
    let sender = self.sender.ok_or(Error::new(ErrorKind::Other, "missing sender"))?;
    let heart = self.heart.ok_or(Error::new(ErrorKind::Other, "missing heart"))?;
    Ok(State { sender, heart })
  }
}

#[derive(Clone)]
pub struct State {
  sender: Sender<blinkrs::Message>,
  heart: Sender<HeartControl>,
}

impl State {
  pub fn builder() -> StateBuilder {
    StateBuilder::default()
  }
}

#[derive(Debug, Serialize)]
struct ControlResponse {
  ok: bool,
  timestamp: chrono::DateTime<chrono::Utc>,
}

impl Default for ControlResponse {
  fn default() -> Self {
    Self {
      ok: true,
      timestamp: chrono::Utc::now(),
    }
  }
}

#[derive(Default, Debug, Deserialize)]
struct ControlQuery {
  mode: String,
  code: String,
  pattern: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
struct HookProgressPayload {
  print_time_left: Option<u64>,
  print_time: Option<u64>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
struct HookStatePayload {
  text: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
struct HookPayload {
  device_identifier: Option<String>,
  topic: Option<String>,
  message: Option<String>,
  state: Option<HookStatePayload>,
  progress: Option<HookProgressPayload>,
}

impl HookPayload {
  pub fn qualified(&mut self) -> Option<QualifiedPayload> {
    let top = self.topic.take();
    let dev = self.device_identifier.take();
    let mes = self.message.take();

    top.zip(dev).zip(mes).map(|((top, dev), mes)| QualifiedPayload {
      topic: top,
      device_identifier: dev,
      message: mes,
    })
  }
}

#[derive(Debug, Clone)]
struct QualifiedPayload {
  topic: String,
  device_identifier: String,
  message: String,
}

pub async fn control(mut req: Request<State>) -> tide::Result {
  let query = req.body_json::<ControlQuery>().await.map_err(|error| {
    log::warn!("uanble to parse control payload - {}", error);
    tide::Error::from_str(422, "bad-payload")
  })?;

  let actual = std::env::var("HEARTBEAT_SECRET_CODE").unwrap_or_default();

  if actual != query.code {
    log::warn!("unauthorized attempt to set heartbeat");
    return Ok(tide::Response::builder(404).body("not-found").build());
  }

  let result = match query.mode.as_str() {
    "off" => req.state().heart.send(HeartControl::Stop).await.map(|_| ()),
    "on" => req.state().heart.send(HeartControl::Start).await.map(|_| ()),
    "load" => {
      let name = query.pattern.ok_or(Error::new(ErrorKind::Other, "missing name"))?;
      req.state().heart.send(HeartControl::Load(name)).await.map(|_| ())
    }
    unknown => {
      log::warn!("unrecognized control input from payload - '{}'", unknown);
      Ok(())
    }
  };

  if let Err(error) = result {
    log::warn!("unable to send control message to heartbeat - {}", error);
  }

  tide::Body::from_json(&ControlResponse::default()).map(|bod| tide::Response::builder(200).body(bod).build())
}

pub async fn webhook(mut req: Request<State>) -> tide::Result {
  let mut body = req
    .body_json::<HookPayload>()
    .await
    .map_err(|error| {
      log::warn!("unable to read request body into string - '{}'", error);
      error
    })
    .unwrap_or_default();

  let qualified = match body.qualified() {
    Some(q) => q,
    None => {
      log::info!("no valid state from payload, skipping");
      return Ok(Response::builder(200).build());
    }
  };

  log::info!("device[{}] - {}", qualified.device_identifier, qualified.message);

  let blink = match qualified.topic.as_str() {
    "Print Done" => blinkrs::Message::Immediate(blinkrs::Color::Green, None),
    "Print Started" => blinkrs::Message::Immediate(blinkrs::Color::Green, None),
    "Print Progress" => blinkrs::Message::Immediate(blinkrs::Color::Green, None),
    _ => blinkrs::Message::Immediate(blinkrs::Color::Three(255, 255, 100), None),
  };

  req.state().sender.send(blink).await.map_err(|error| {
    log::warn!("unable to send blink message to channel - {}", error);
    tide::Error::from_str(500, "blinkr-problem")
  })?;

  Ok("ok".into())
}

pub async fn missing(req: Request<State>) -> tide::Result {
  log::warn!("[warning] unknown request received - '{}'", req.url().path());
  Ok(Response::builder(404).build())
}
