use serde::Deserialize;
use tide::{Request, Response, Result};

use super::State;

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

pub async fn hook(mut req: Request<State>) -> Result {
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
