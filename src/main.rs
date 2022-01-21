use std::io::Result;
use std::time::Duration;

use async_std::{channel, channel::Receiver, channel::Sender};
use serde::Deserialize;
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

#[derive(Default, Debug, Deserialize)]
struct ControlQuery {
	mode: String,
}

async fn control(req: Request<State>) -> tide::Result {
	let query = req.query::<ControlQuery>().unwrap_or_default();
  log::debug!("received control request - {:?}", query);

	let result = match query.mode.as_str() {
		"off" => req.state().heart.send(false).await.map(|_| ()),
		"on" => req.state().heart.send(true).await.map(|_| ()),
		_ => Ok(()),
	};

	if let Err(error) = result {
		log::warn!("unable to send control message to heartbeat - {}", error);
	}

  Ok("".into())
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

async fn missing(mut _req: Request<State>) -> tide::Result {
  log::debug!("[warning] unknown request received");
  Ok(Response::builder(404).build())
}

async fn worker(receiver: Receiver<blinkrs::Message>) -> Result<()> {
  log::nifo!("worker thread spawned");

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

async fn heartbeat(heart: Heart) {
  log::debug!("heartbeat thread started");
  let mut step = 0u8;
  let mut working = true;

  loop {
    log::debug!("beating heart, checking for kill message first");

    if let Ok(pulse) = heart.receiver.try_recv() {
      if pulse == false && working == true {
        log::debug!("received kill signal while working, no longer applying light changes");

        if let Err(error) = heart.sender.send(blinkrs::Message::Off).await {
          log::warn!("unable to kill lights on pulse terminal - {}", error);
        }
        working = false;
      }
      if pulse == true && working == false {
        log::debug!("restarting heart into working state");
        working = true;
      }
    }

    if working == true {
      let color = match step {
        0 => blinkrs::Color::Three(255, 0, 0),
        1 => blinkrs::Color::Three(0, 255, 0),
        2 => blinkrs::Color::Three(0, 0, 255),
        _ => blinkrs::Color::Three(0, 255, 255),
      };

      step = step + 1;

      if step > 2 {
        step = 0;
      }

			let mut index = 0;

			while index < 4 {
				if let Err(error) = heart.sender.send(blinkrs::Message::Immediate(color, Some(3 + index))).await {
					log::warn!("unable to beat heart - {}", error);
				}

				index += 1;
			}
    }

    async_std::task::sleep(Duration::from_millis(2000)).await;
  }
}

#[derive(Clone)]
struct Heart {
  sender: Sender<blinkrs::Message>,
  receiver: Receiver<bool>,
}

#[derive(Clone)]
struct State {
  sender: Sender<blinkrs::Message>,
  heart: Sender<bool>,
}

async fn serve() -> Result<()> {
  log::info!("thread running, opening blinkrs");
  let (sender, receiver) = channel::bounded(1);
  let arteries = channel::bounded(1);
  let heart = Heart {
    sender: sender.clone(),
    receiver: arteries.1,
  };

  let wh = async_std::task::spawn(worker(receiver));
  let hbh = async_std::task::spawn(heartbeat(heart));

  let addr = std::env::var("WEBHOOK_LISTENER_ADDR").unwrap_or("0.0.0.0:8081".into());
  log::info!("preparing web thread on addr '{}'", addr);

  let mut app = tide::with_state::<State>(State {
    sender: sender.clone(),
    heart: arteries.0,
  });
  app.at("/incoming-webhook").post(receive);
  app.at("/heartbeat").get(control);
  app.at("/*").all(missing);
  app.listen(&addr).await?;

  wh.await?;
  hbh.await;
  Ok(())
}

fn main() -> Result<()> {
  dotenv::dotenv().map_err(|error| std::io::Error::new(std::io::ErrorKind::Other, error))?;
  env_logger::init();

  log::info!("starting async thread");
  async_std::task::block_on(serve())?;
  Ok(())
}
