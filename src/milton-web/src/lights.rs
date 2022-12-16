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
  let mut last_connection_attempt = std::time::Instant::now();

  loop {
    connection = match (connection, last_configuration.as_ref()) {
      (None, Some(configuration)) => {
        if std::time::Instant::now()
          .duration_since(last_connection_attempt)
          .as_secs()
          > 5
        {
          log::info!("attempting to establish serial connection to light controller: {configuration:?}");
          last_connection_attempt = std::time::Instant::now();

          let connection = serialport::new(&configuration.device, configuration.baud)
            .open()
            .map_err(|error| {
              log::warn!("unable to connect - {error}");
              error
            })
            .ok();

          if connection.is_some() {
            log::info!("serial connection to light controller suceeded");
          }

          connection
        } else {
          None
        }
      }
      (Some(con), _) => Some(con),
      (None, None) => None,
    };

    match receiver.try_recv() {
      Ok(Command::Configure(configuration)) => {
        log::info!("attempting to update connection via configuration - {configuration:?}");
        // Update our configuration and let the next loop take care of reconnection
        last_configuration = Some(configuration.clone());
        continue;
      }

      Ok(Command::BasicColor(color @ BasicColor::Red))
      | Ok(Command::BasicColor(color @ BasicColor::Green))
      | Ok(Command::BasicColor(color @ BasicColor::Blue)) => {
        connection = match connection.take() {
          Some(mut con) => {
            log::debug!("sending color command to lights - {color}");

            if let Err(error) = write!(con, "{color}:") {
              log::warn!("unable to write command - {error}");
              None
            } else {
              Some(con)
            }
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

            if let Err(error) = write!(con, "{}:", if off == Command::Off { "off" } else { "on" }) {
              log::warn!("unable to write command - {error}");
              None
            } else {
              Some(con)
            }
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
      let mut buffer = Vec::new();
      let has_bytes = con.bytes_to_read().map(|amount| amount > 0).unwrap_or_default();

      if has_bytes {
        let read_result = con.read_to_end(&mut buffer);

        if let Err(error) = read_result {
          if error.kind() != std::io::ErrorKind::TimedOut {
            log::warn!("failed - {error}");
            break;
          } else {
            log::warn!("timeout after buffer read - {error} ({} bytes)", buffer.len());
          }
        }

        let contents = String::from_utf8(buffer);
        log::debug!("read bytes - {contents:?}");
      }
    }

    if std::time::Instant::now().duration_since(last_debug).as_secs() > 5 {
      last_debug = std::time::Instant::now();
      log::debug!("empty effect channel reads since last debug: {empty_reads}");
      empty_reads = 0;
    }

    timer.next().await;
  }

  Ok(())
}
