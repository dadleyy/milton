use async_std::{channel, channel::Receiver, channel::Sender};
use serde::Deserialize;
use std::io::Result;
use tide::{Request, Response};

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

async fn receive(mut req: Request<State>) -> tide::Result {
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
    "Print Started" => 1,
    _ => 0,
  };

  req.state().sender.send(blink).await.map_err(|error| {
    log::warn!("unable to send blink message to channel - {}", error);
    tide::Error::from_str(500, "blinkr-problem")
  })?;

  log::info!("request received '{:?}'", qualified);
  Ok("yay".into())
}

async fn missing(mut _req: Request<State>) -> tide::Result {
  log::info!("[warning] unknown request received");
  Ok(Response::builder(404).build())
}

async fn worker(receiver: Receiver<u8>) -> Result<()> {
  log::info!("worker thread spawned");

  let blinker = blinkrs::Blinkers::new().map_err(|error| {
    log::warn!("unable to initialize blink(1) usb library - {}", error);
    std::io::Error::new(std::io::ErrorKind::Other, error)
  })?;

  log::info!(
    "found {} devices",
    blinker.device_count().map_err(|error| {
      log::warn!("unable to count devices - {}", error);
      std::io::Error::new(std::io::ErrorKind::Other, error)
    })?,
  );

  while let Ok(i) = receiver.recv().await {
    log::info!("[worker] received message {}", i);

    let attempt = match i {
      0 => blinker.send(blinkrs::Message::Immediate(blinkrs::Color::Red)),
      _ => blinker.send(blinkrs::Message::Off),
    };

    if let Err(error) = attempt {
      log::warn!("unable to send blinkrs message - '{}'", error);
    }
  }

  Ok(())
}

#[derive(Clone)]
struct State {
  sender: Sender<u8>,
}

async fn serve() -> Result<()> {
  log::info!("thread running, opening blinkrs");
  let (s, r) = channel::bounded(1);

  let handle = async_std::task::spawn(worker(r));
  let addr = std::env::var("WEBHOOK_LISTENER_ADDR").unwrap_or("0.0.0.0:8081".into());

  log::info!("preparing web thread on addr '{}'", addr);
  let mut app = tide::with_state::<State>(State { sender: s });
  app.at("/incoming-webhook").post(receive);
  app.at("/*").all(missing);
  app.listen(&addr).await?;
  handle.await?;
  Ok(())
}

fn main() -> Result<()> {
  dotenv::dotenv().map_err(|error| std::io::Error::new(std::io::ErrorKind::Other, error))?;
  env_logger::init();

  log::info!("starting async thread");
  async_std::task::block_on(serve())?;
  Ok(())
}
