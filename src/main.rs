use async_std::{channel, channel::Receiver, channel::Sender};
use serde::Deserialize;
use std::io::Result;
use tide::{Request, Response};

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct HookProgressPayload {
  print_time_left: Option<u64>,
  print_time: Option<u64>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct HookStatePayload {
  text: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct HookPayload {
  device_identifier: Option<String>,
  topic: Option<String>,
  message: Option<String>,
  state: Option<HookStatePayload>,
  progress: Option<HookProgressPayload>,
}

async fn receive(mut req: Request<State>) -> tide::Result {
  let body = req
    .body_json::<HookPayload>()
    .await
    .map_err(|error| {
      log::warn!("unable to read request body into string - {}", error);
      error
    })
    .unwrap_or_default();

  log::info!("request received '{:?}'", body);
  Ok("yay".into())
}

async fn missing(mut _req: Request<State>) -> tide::Result {
  log::info!("[warning] unknown request received");
  Ok(Response::builder(404).build())
}

async fn worker(receiver: Receiver<u8>) -> Result<()> {
  log::info!("worker thread spawned");

  let blinkrs = blinkrs::Blinkers::new().map_err(|error| {
    log::warn!("unable to initialize blink(1) usb library - {}", error);
    std::io::Error::new(std::io::ErrorKind::Other, error)
  })?;

  log::info!(
    "found {} devices",
    blinkrs.device_count().map_err(|error| {
      log::warn!("unable to count devices - {}", error);
      std::io::Error::new(std::io::ErrorKind::Other, error)
    })?,
  );

  while let Ok(i) = receiver.recv().await {
    log::info!("received message {}", i);
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

  log::info!("preparing web thread");
  let mut app = tide::with_state::<State>(State { sender: s });
  app.at("/incoming-webhook").post(receive);
  app.at("/*").all(missing);
  app.listen("0.0.0.0:8080").await?;
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
