use clap::Parser;
use serde::Deserialize;
use std::io::Result;

use async_std::channel;
use async_std::prelude::FutureExt;
use async_std::stream::StreamExt;

#[derive(Deserialize, Debug)]
struct RuntimeConfiguration {
  #[allow(unused)]
  lights: milton::lights::LightConfiguration,
  oauth: milton::oauth::AuthZeroConfig,
  server: milton::server::Configuration,
}

#[derive(Deserialize, clap::Parser)]
struct CommandLineOptions {
  config: String,
  device: Option<String>,
}

async fn manage_effects(
  server_effects: channel::Receiver<milton::server::effects::Effects>,
  light_commands: channel::Sender<milton::lights::Command>,
) -> Result<()> {
  log::debug!("managing effects");
  let mut interval = async_std::stream::interval(std::time::Duration::from_millis(100));

  loop {
    let server_result: Result<Option<&str>> = match server_effects.try_recv() {
      Ok(milton::server::effects::Effects::Lights(command)) => {
        if let Err(error) = light_commands.send(command).await {
          log::warn!("unable to propagate command - {error}");
        }

        Ok(Some(""))
      }
      Err(error) if error.is_closed() => {
        log::warn!("effect loop closed");
        break;
      }
      _ => Ok(None),
    };

    if let Err(error) = server_result {
      log::warn!("problem with result - {error}");
    }

    interval.next().await;
  }

  Err(std::io::Error::new(std::io::ErrorKind::Other, "closed effect loop"))
}

async fn serve(config: RuntimeConfiguration) -> Result<()> {
  log::info!("thread running, preparing channels");
  let server_effects = channel::bounded(1);
  let light_effects = channel::bounded(10);

  log::info!("initializing server...");
  let server = milton::server::State::builder()
    .oauth(config.oauth)
    .version(option_env!("MILTON_VERSION").unwrap_or_else(|| "dev").to_string())
    .config(config.server)
    .sender(server_effects.0.clone())
    .build()?;

  light_effects
    .0
    .send(milton::lights::Command::Configure(config.lights))
    .await
    .map_err(|error| {
      log::error!("unable to populate initial light effect manager initial config - {error}");
      std::io::Error::new(std::io::ErrorKind::Other, error)
    })?;

  log::info!("spawing effect management thread");
  let effect_thread = async_std::task::spawn(manage_effects(server_effects.1, light_effects.0));

  log::info!("spawing blinker channel worker thread");
  let light_thread = async_std::task::spawn(milton::lights::run(light_effects.1));

  let addr = std::env::var("WEBHOOK_LISTENER_ADDR").unwrap_or_else(|_| "0.0.0.0:8081".into());
  log::info!("preparing web thread on addr '{}'", addr);

  milton::server::listen(server, addr)
    .race(light_thread)
    .race(effect_thread)
    .await?;

  Ok(())
}

fn main() -> Result<()> {
  if dotenv::dotenv().is_err() {
    eprintln!("warning: no '.env' file detected'");
  }

  env_logger::init();
  let args = CommandLineOptions::parse();

  log::info!("loading config from '{}'", args.config);
  let contents = std::fs::read_to_string(args.config)?;
  log::info!("loaded config from '{contents}'");
  let mut parsed = toml::from_str::<RuntimeConfiguration>(&contents)?;

  if let Some(device) = args.device {
    parsed.lights.device = device;
  }

  log::info!("starting async main thread");
  async_std::task::block_on(serve(parsed))?;
  Ok(())
}
