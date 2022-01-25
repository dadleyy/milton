use std::collections::HashMap;
use std::io::{Error, ErrorKind, Result};
use std::path::{Path, PathBuf};

use async_std::channel::{Receiver, Sender};

#[derive(Debug, Clone)]
pub enum HeartControl {
  Stop,
  Start,
}

#[derive(Default)]
pub struct HeartBuilder {
  sender: Option<Sender<blinkrs::Message>>,
  receiver: Option<Receiver<HeartControl>>,
  delay: Option<std::time::Duration>,
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

  pub fn build(self) -> Result<Heart> {
    let Self {
      sender,
      receiver,
      patterns,
      delay,
    } = self;
    let su = sender.ok_or(Error::new(ErrorKind::Other, ""))?;
    let ru = receiver.ok_or(Error::new(ErrorKind::Other, ""))?;
    let dir = patterns.ok_or(Error::new(ErrorKind::Other, "missing pattern directory"))?;

    if dir.is_dir() != true {
      let warning = format!("{:?} is not a valid directory", dir);
      return Err(Error::new(ErrorKind::Other, warning));
    }

    Ok(Heart {
      sender: su,
      receiver: ru,
      delay: delay.unwrap_or(std::time::Duration::from_millis(100)),
      patterns: dir,
    })
  }
}

#[derive(Clone)]
pub struct Heart {
  sender: Sender<blinkrs::Message>,
  receiver: Receiver<HeartControl>,
  delay: std::time::Duration,
  patterns: PathBuf,
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
    parse_pattern(&source)
  }
}

fn parse_pattern(source: &str) -> Result<HashMap<u8, HashMap<u8, blinkrs::Color>>> {
  let out = source.lines().fold(HashMap::with_capacity(100), |mut store, line| {
    let mut bits = line.split(" ").into_iter();

    let frame = bits.next().and_then(|f| {
      let mut chars = f.chars();
      if let Some('F') = chars.next() {
        return chars.collect::<String>().parse::<u8>().ok();
      }
      return None;
    });

    let ledn = bits.next().and_then(|f| {
      let mut chars = f.chars();
      if let Some('L') = chars.next() {
        return chars.collect::<String>().parse::<u8>().ok();
      }
      return None;
    });

    let colors = bits
      .into_iter()
      .map(|s| s.parse::<u8>().ok())
      .flatten()
      .collect::<Vec<u8>>();

    let red = colors.get(0);
    let green = colors.get(1);
    let blue = colors.get(2);
    let combined = frame.zip(ledn).zip(red).zip(green).zip(blue);

    if let Some(((((frame, led), red), green), blue)) = combined {
      let mut existing = store.remove(&frame).unwrap_or_else(|| HashMap::with_capacity(10));
      existing.insert(led, blinkrs::Color::Three(*red, *green, *blue));
      log::debug!("pattern frame[{}] led[{}] {:?}", frame, led, colors);
      store.insert(frame, existing);
    }

    store
  });
  Ok(out)
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

  fn inc(&mut self) {
    if self.1 {
      self.0 = self.0.overflowing_add(1).0;
    }
  }

  fn seek(&mut self, pattern: HashMap<u8, HashMap<u8, blinkrs::Color>>) {
    self.2 = pattern;
  }

  fn messages(&self) -> Vec<blinkrs::Message> {
    let Cursor(frame, running, pattern) = self;

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
          cursor.stop();
          log::info!("received stop command");
        }
        HeartControl::Start => {
          cursor.start();
          log::info!("received start command");
        }
      }
    }

    log::info!("entering frame '#{}'", cursor);

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
