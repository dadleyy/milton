use std::io::Result;

use async_std::{channel, channel::Receiver};

async fn worker(receiver: Receiver<blinkrs::Message>) -> Result<()> {
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

  while let Ok(message) = receiver.recv().await {
    log::debug!("[worker] received message {:?}", message);
    let attempt = blinker.send(message);

    if let Err(error) = attempt {
      log::warn!("unable to send blinkrs message - '{}'", error);
    }
  }

  Ok(())
}

async fn serve() -> Result<()> {
  log::info!("thread running, preparing channels...");
  let messages = channel::bounded(1);
  let arteries = channel::bounded(1);

  log::info!("initializing redis configuration...");
  let redis = milton::redis::from_env()?;

  log::info!("initializing heart...");
  let heart = milton::heartbeat::Heart::builder()
    .sender(messages.0.clone())
    .receiver(arteries.1)
    .patterns(std::env::var("HEARTBEAT_PATTERN_DIR").ok().unwrap_or_default().into())
    .redis(redis.clone())
    .delay(
      std::env::var("HEARTBEAT_FRAME_DELAY")
        .ok()
        .and_then(|a| a.parse::<u64>().ok())
        .map(std::time::Duration::from_millis)
        .unwrap_or(std::time::Duration::from_millis(500u64)),
    )
    .ledr(
      std::env::var("HEARTBEAT_LEDN_START")
        .ok()
        .and_then(|num| num.parse::<u8>().ok())
        .unwrap_or_else(|| {
          log::warn!("HEARTBEAT_LEDN_START not valid, defaulting");
          milton::constants::DEFAULT_LEDN_START
        }),
      std::env::var("HEARTBEAT_LEDN_END")
        .ok()
        .and_then(|num| num.parse::<u8>().ok())
        .unwrap_or_else(|| {
          log::warn!("HEARTBEAT_LEDN_END not valid, defaulting");
          milton::constants::DEFAULT_LEDN_END
        }),
    )
    .build()?;

  log::info!("initializing server...");
  let server = milton::server::State::builder()
    .oauth(milton::auth_zero::AuthZeroConfig::from_env()?)
    .sender(messages.0.clone())
    .heart(arteries.0)
    .redis(redis.clone())
    .build()?;

  log::info!("spawing blinker channel worker thread");
  let blinker_worker = async_std::task::spawn(worker(messages.1));

  log::info!("spawing heartbeat channel worker thread");
  let heartbeat_worker = async_std::task::spawn(milton::heartbeat::beat(heart));

  let addr = std::env::var("WEBHOOK_LISTENER_ADDR").unwrap_or("0.0.0.0:8081".into());
  log::info!("entering main web thread on '{}'", addr);

  if let Err(error) = milton::server::run(server, &addr).await {
    log::warn!("web thread faile with error - {}", error);
  }

  blinker_worker.await?;
  heartbeat_worker.await?;
  Ok(())
}

fn main() -> Result<()> {
  dotenv::dotenv().map_err(|error| std::io::Error::new(std::io::ErrorKind::Other, error))?;
  env_logger::init();

  log::info!("starting async main thread");
  async_std::task::block_on(serve())?;
  Ok(())
}
