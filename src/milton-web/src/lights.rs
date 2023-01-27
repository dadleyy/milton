use async_std::channel;
use async_std::stream::StreamExt;
use serde::Deserialize;
use std::io::{self, Result};

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

impl std::fmt::Display for Command {
  fn fmt(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
    match self {
      Self::On => write!(formatter, "on:"),
      Self::Off => write!(formatter, "off:"),
      Self::BasicColor(color) => write!(formatter, "{color}:"),
      _ => Ok(()),
    }
  }
}

/// Helper method that will attempt to pull a message off our channel and handle returning an err
/// based on the correct conditions when that should occur.
fn next(channel: &mut channel::Receiver<Command>) -> Result<Option<Command>> {
  if channel.is_closed() {
    return Err(io::Error::new(io::ErrorKind::Other, "message channel has been closed"));
  }

  match channel.try_recv() {
    Err(error) if error.is_empty() => Ok(None),
    Err(other) => Err(io::Error::new(io::ErrorKind::Other, format!("{other}"))),
    Ok(cmd) => Ok(Some(cmd)),
  }
}

#[allow(clippy::missing_docs_in_private_items)]
pub async fn run(mut receiver: channel::Receiver<Command>) -> Result<()> {
  log::debug!("starting light effect manager runtime");
  let mut timer = async_std::stream::interval(std::time::Duration::from_millis(10));

  // The connection will hold our serialport ttyport.
  let mut connection = None;

  let mut empty_reads = 0;
  let mut last_debug = std::time::Instant::now();
  let mut last_configuration: Option<LightConfiguration> = None;
  let mut force_reconnect = false;

  // A bit of a hack, we could use an `Option<Instant>` instead. The goal here is to allow the
  // first configuration message to kick in immediately, while forcing others to wait a short
  // period.
  let mut last_connection_attempt = std::ops::Sub::sub(std::time::Instant::now(), std::time::Duration::from_secs(10));

  loop {
    connection = match (connection, last_configuration.as_ref(), force_reconnect) {
      (None, Some(configuration), _) | (_, Some(configuration), true) => {
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
      (Some(con), _, _) => Some(con),
      (None, None, _) => None,
    };

    if force_reconnect {
      force_reconnect = false;
    }

    let bytes_to_send = match next(&mut receiver)? {
      Some(command @ Command::Off) | Some(command @ Command::On) | Some(command @ Command::BasicColor(_)) => {
        Some(format!("{command}"))
      }
      Some(Command::Configure(config)) => {
        log::info!("received updated light-controller serial configuration to apply");
        last_configuration = Some(config);
        continue;
      }
      None => None,
    };

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

      if let Some(message) = bytes_to_send {
        if let Err(error) = write!(con, "{message}") {
          log::warn!("unable to write message - {error}");
          force_reconnect = true;
        }
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
