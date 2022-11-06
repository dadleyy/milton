use async_std::channel;
use async_std::stream::StreamExt;
use serde::Deserialize;
use std::io::Result;

#[allow(clippy::missing_docs_in_private_items)]
#[derive(Deserialize, Debug, PartialEq, Eq, Clone)]
pub struct LightConfiguration {
  pub device: String,

  pub baud: u32,
}

#[allow(clippy::missing_docs_in_private_items)]
#[derive(Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BasicColor {
  Red,
  Green,
  Blue,
}

impl std::fmt::Display for BasicColor {
  fn fmt(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
    match self {
      BasicColor::Red => write!(formatter, "red"),
      BasicColor::Green => write!(formatter, "green"),
      BasicColor::Blue => write!(formatter, "blue"),
    }
  }
}

#[allow(clippy::missing_docs_in_private_items)]
#[derive(Debug, PartialEq, Eq)]
pub enum Command {
  Configure(LightConfiguration),
  On,
  BasicColor(BasicColor),
  Off,
}

#[allow(clippy::missing_docs_in_private_items)]
pub async fn run(receiver: channel::Receiver<Command>) -> Result<()> {
  log::debug!("starting light effect manager runtime");
  let mut timer = async_std::stream::interval(std::time::Duration::from_millis(100));
  let mut connection = None;
  let mut empty_reads = 0;
  let mut last_debug = std::time::Instant::now();
  let mut last_configuration: Option<LightConfiguration> = None;

  loop {
    match (connection.is_some(), last_configuration.as_ref()) {
      (false, Some(configuration)) => {
        log::info!("attempting to establish serial connection to light controller: {configuration:?}");

        connection = serialport::new(&configuration.device, configuration.baud)
          .open()
          .map_err(|error| {
            log::error!("unable to connect - {error}");
            error
          })
          .and_then(|mut port| {
            port
              .set_timeout(std::time::Duration::from_millis(10))
              .map_err(|error| std::io::Error::new(std::io::ErrorKind::Other, error))?;

            Ok(port)
          })
          .ok();
      }
      _ => log::trace!("no reconnection necessary"),
    }

    match receiver.try_recv() {
      Ok(Command::Configure(configuration)) => {
        log::info!("attempting to update connection via configuration - {configuration:?}");

        last_configuration = Some(configuration.clone());

        connection = serialport::new(&configuration.device, configuration.baud)
          .open()
          .map_err(|error| {
            log::error!("unable to connect - {error}");
            error
          })
          .and_then(|mut port| {
            port
              .set_timeout(std::time::Duration::from_millis(10))
              .map_err(|error| std::io::Error::new(std::io::ErrorKind::Other, error))?;

            Ok(port)
          })
          .ok();
      }

      Ok(Command::BasicColor(color @ BasicColor::Red))
      | Ok(Command::BasicColor(color @ BasicColor::Green))
      | Ok(Command::BasicColor(color @ BasicColor::Blue)) => {
        connection = match connection.take() {
          Some(mut con) => {
            log::debug!("turning lights off");

            if let Err(error) = writeln!(con, "{color}") {
              log::warn!("unable to write command - {error}");
            }

            Some(con)
          }

          None => {
            log::warn!("no serial connection ready yet");
            None
          }
        };
      }

      Ok(off @ Command::Off) | Ok(off @ Command::On) => {
        connection = match connection.take() {
          Some(mut con) => {
            log::debug!("turning lights off");

            if let Err(error) = writeln!(con, "{}", if off == Command::Off { "off" } else { "on" }) {
              log::warn!("unable to write command - {error}");
            }

            Some(con)
          }

          None => {
            log::warn!("no serial connection ready yet");
            None
          }
        };
      }

      Err(error) if error.is_closed() => {
        log::warn!("unable to read - {error}");
        break;
      }

      Err(_) => {
        log::trace!("no messages");
        empty_reads += 1;
      }
    }

    if let Some(ref mut con) = &mut connection {
      let mut buffer = [0u8; 255];

      match con.read(&mut buffer) {
        Err(error) if error.kind() == std::io::ErrorKind::TimedOut => log::trace!("nothing to read"),

        Err(error) => {
          log::warn!("failed reading connection - {error}");
          connection = None;
        }

        Ok(amount) => {
          let contents = std::str::from_utf8(&buffer[0..amount]);
          log::debug!("read {amount} bytes - {contents:?}");
        }
      };
    }

    if std::time::Instant::now().duration_since(last_debug).as_secs() > 5 {
      last_debug = std::time::Instant::now();
      log::debug!("empty reads since last debug: {empty_reads}");
      empty_reads = 0;
    }

    timer.next().await;
  }

  Ok(())
}
