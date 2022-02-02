use std::collections::HashMap;
use std::io::{Error, ErrorKind, Result};
use std::path::{Path, PathBuf};

use async_std::channel::{Receiver, Sender};

#[derive(Debug, Clone)]
pub enum HeartControl {
  Stop,
  Start,
  Load(String),
}

#[derive(Default)]
pub struct HeartBuilder {
  sender: Option<Sender<blinkrs::Message>>,
  receiver: Option<Receiver<HeartControl>>,
  delay: Option<std::time::Duration>,
  ledr: Option<(u8, u8)>,
  patterns: Option<PathBuf>,
}

impl HeartBuilder {
  pub fn receiver(mut self, chan: Receiver<HeartControl>) -> Self {
    self.receiver = Some(chan);
    self
  }

  pub fn sender(mut self, chan: Sender<blinkrs::Message>) -> Self {
    self.sender = Some(chan);
    self
  }

  pub fn patterns(mut self, dir: PathBuf) -> Self {
    self.patterns = Some(dir);
    self
  }

  pub fn delay(mut self, del: std::time::Duration) -> Self {
    self.delay = Some(del);
    self
  }

  pub fn ledr(mut self, start: u8, end: u8) -> Self {
    self.ledr = Some((start, end));
    log::debug!("heartbeat led range - [{} -> {}]", start, end);
    self
  }

  pub fn build(self) -> Result<Heart> {
    let Self {
      sender,
      receiver,
      patterns,
      delay,
      ledr,
    } = self;
    let su = sender.ok_or(Error::new(ErrorKind::Other, "heart missing message output channel."))?;
    let ru = receiver.ok_or(Error::new(ErrorKind::Other, "missing command receiver channel."))?;
    let dir = patterns.ok_or(Error::new(ErrorKind::Other, "missing pattern directory."))?;

    if dir.is_dir() != true {
      let warning = format!("'{:?}' is not a valid directory for pattern storage", dir);
      return Err(Error::new(ErrorKind::Other, warning));
    }

    Ok(Heart {
      sender: su,
      receiver: ru,
      delay: delay.unwrap_or(std::time::Duration::from_millis(100)),
      patterns: dir,
      ledr: ledr.unwrap_or((crate::constants::DEFAULT_LEDN_START, crate::constants::DEFAULT_LEDN_END)),
    })
  }
}

#[derive(Clone)]
pub struct Heart {
  sender: Sender<blinkrs::Message>,
  receiver: Receiver<HeartControl>,
  delay: std::time::Duration,
  patterns: PathBuf,
  ledr: (u8, u8),
}

impl Heart {
  pub fn builder() -> HeartBuilder {
    HeartBuilder::default()
  }

  async fn read<P>(&self, input: P) -> Result<HashMap<u8, HashMap<u8, blinkrs::Color>>>
  where
    P: AsRef<Path>,
  {
    let mut target = self.patterns.clone();
    target.push(input);
    let bytes = async_std::fs::read(target.clone()).await?;
    let source = String::from_utf8(bytes).map_err(|error| {
      log::warn!("unable to read '{:?}' as utf-8 string - {}", target, error);
      Error::new(ErrorKind::Other, "invalid utf-8 source")
    })?;

    patternize(&source).map(|p| {
      p.into_iter()
        .map(|(frame, mut map)| {
          for light in self.ledr.0..(self.ledr.1 + 1) {
            let color = map.remove(&light).unwrap_or(blinkrs::Color::Three(0, 0, 0));
            map.insert(light, color);
          }

          (frame, map)
        })
        .collect()
    })
  }
}

fn rl(
  store: Result<HashMap<u8, HashMap<u8, blinkrs::Color>>>,
  line: &str,
) -> Result<HashMap<u8, HashMap<u8, blinkrs::Color>>> {
  let mut store = match store {
    Err(error) => return Err(error),
    Ok(store) => store,
  };

  if line.trim().len() == 0 {
    log::debug!("skipping empty line");
    return Ok(store);
  }

  let mut bits = line.split(" ").into_iter();
  let leader = bits.next();

  if let Some("#") = leader {
    log::debug!("comment - '{}'", line);
    return Ok(store);
  }

  let frame = leader
    .ok_or(Error::new(ErrorKind::Other, "malformed line"))
    .and_then(|f| {
      let mut chars = f.chars();
      if let Some('F') = chars.next() {
        return chars.collect::<String>().parse::<u8>().map_err(|error| {
          log::warn!("uanble to parse frame number - {}", error);
          Error::new(ErrorKind::Other, format!("{}", error))
        });
      }
      return Err(Error::new(ErrorKind::Other, "malformed-line"));
    })?;

  let ledn = bits
    .next()
    .ok_or(Error::new(ErrorKind::Other, "malformed line"))
    .and_then(|f| {
      let mut chars = f.chars();
      if let Some('L') = chars.next() {
        return chars.collect::<String>().parse::<u8>().map_err(|error| {
          log::warn!("unable to parse led number - {}", error);
          Error::new(ErrorKind::Other, format!("{}", error))
        });
      }
      return Err(Error::new(ErrorKind::Other, "malformed-line"));
    })?;

  let colors = bits
    .into_iter()
    .map(|s| {
      s.parse::<u8>().map_err(|error| {
        log::warn!("unable to parse color - '{}'", error);
        Error::new(ErrorKind::Other, "malformed-line")
      })
    })
    .collect::<Result<Vec<u8>>>()?;

  let red = colors.get(0);
  let green = colors.get(1);
  let blue = colors.get(2);
  let combined = red
    .zip(green)
    .zip(blue)
    .map(|((r, g), b)| (r, g, b))
    .ok_or(Error::new(ErrorKind::Other, "malformed-color"))?;

  let mut existing = store.remove(&frame).unwrap_or_else(|| HashMap::with_capacity(10));
  existing.insert(ledn, blinkrs::Color::Three(*combined.0, *combined.1, *combined.2));
  store.insert(frame, existing);

  Ok(store)
}

fn patternize<P>(source: P) -> Result<HashMap<u8, HashMap<u8, blinkrs::Color>>>
where
  P: AsRef<str>,
{
  source.as_ref().lines().fold(Ok(HashMap::with_capacity(100)), rl)
}

struct Cursor(u8, bool, HashMap<u8, HashMap<u8, blinkrs::Color>>);

impl std::fmt::Display for Cursor {
  fn fmt(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
    write!(formatter, "f[{}] r[{}] p[{}]", self.0, self.1, self.2.len())
  }
}

impl Default for Cursor {
  fn default() -> Self {
    Cursor(0, true, HashMap::new())
  }
}

impl Cursor {
  fn start(&mut self) {
    if self.1 == false {
      self.0 = 0;
    }

    self.1 = true;
  }

  fn stop(&mut self) {
    if self.1 == true {
      self.0 = 0;
    }

    self.1 = false;
  }

  // If we are not running and we're on the 0th frame, we will send a off-all blinkrs message.
  // To avoid sending this over and over again, we're using the frame will not running to indicate
  // whether or not we have already instructed the blinker to turn off.
  fn inc(&mut self) {
    let Cursor(frame, running, _) = self;

    if *running != true && *frame == 0u8 {
      self.0 = 1;
      return;
    }

    if *running {
      self.0 = frame.overflowing_add(1).0;
    }
  }

  fn seek(&mut self, pattern: HashMap<u8, HashMap<u8, blinkrs::Color>>) {
    self.0 = 0;
    self.1 = true;
    self.2 = pattern;
  }

  fn messages(&self) -> Vec<blinkrs::Message> {
    let Cursor(frame, running, pattern) = self;

    // If we're not running and are on the 0th frame, we know that this is the first time we have
    // been asked for our message list while being off: so we send the off message.
    if !running && *frame == 0 {
      log::info!("cursor off and on 0th frame, sending kill to blinker");
      return vec![blinkrs::Message::Off];
    }

    if !running {
      return vec![];
    }

    let index = frame % pattern.len() as u8;
    pattern
      .get(&index)
      .map(|inner| {
        inner
          .iter()
          .fold(Vec::with_capacity(inner.len()), |mut acc, (ledn, color)| {
            acc.push(blinkrs::Message::Immediate(color.clone(), Some(*ledn)));
            acc
          })
      })
      .unwrap_or_default()
  }
}

pub async fn beat(heart: Heart) -> Result<()> {
  log::info!("beating heart on {:?} delay (@ {:?})", heart.delay, heart.patterns);
  let mut cursor = Cursor::default();

  if let Ok(start) = heart.read("init.txt").await.map_err(|error| {
    log::warn!("unable to set initial pattern - {}", error);
    error
  }) {
    log::info!("initial pattern loaded successfully (frames {})", start.len());
    cursor.seek(start);
  }

  loop {
    if let Ok(control) = heart.receiver.try_recv() {
      match control {
        HeartControl::Stop => {
          log::info!("received stop command, stopping cursor");
          cursor.stop();
        }
        HeartControl::Load(name) => {
          log::info!("attempting to load pattern '{}'", name);

          if let Ok(pat) = heart.read(&name).await.map_err(|error| {
            log::warn!("unable to load pattern '{}' - {}", name, error);
            error
          }) {
            log::info!("loaded new pattern '{}'", name);
            cursor.seek(pat);
          }
        }
        HeartControl::Start => {
          log::info!("received start command");
          cursor.start();
        }
      }
    }

    log::debug!("entering frame '#{}'", cursor);

    for message in cursor.messages().into_iter() {
      if let Err(error) = heart.sender.send(message).await {
        log::warn!("unable to send message - {}", error);
        cursor.stop();
        break;
      }
    }

    cursor.inc();
    async_std::task::sleep(heart.delay).await;
  }
}
