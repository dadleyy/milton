#![warn(clippy::missing_docs_in_private_items)]
#![no_std]

//! This library provides the application-level abstractions that will be used by the main runtime
//! itself.

/// Enumerates the various kinds of state requests we can receive from our serial interrupt.
#[derive(Clone)]
pub enum StateRequest {
  /// Turn the LEDs on.
  On,

  /// Turn the LEDs off.
  Off,

  /// Turn the lights red.
  Red,

  /// Turn the lights green.
  Green,

  /// Turn the lights blue.
  Blue,
}

/// Enumerates the kinds of things that can go wrong when parsing requests.
pub enum StateRequestParseError {
  /// Whatever was present in the `str` that we attempted to parse as a request was invalid.
  Unrecognized,
}

impl core::str::FromStr for StateRequest {
  type Err = StateRequestParseError;

  fn from_str(input: &str) -> Result<Self, Self::Err> {
    match input {
      "on" | "On" | "ON" => Ok(Self::On),
      "off" | "Off" | "OFF" => Ok(Self::Off),
      "red" | "Red" | "RED" => Ok(Self::Red),
      "blue" | "Blue" | "BLUE" => Ok(Self::Blue),
      "green" | "Green" | "GREEN" => Ok(Self::Green),
      _ => Err(StateRequestParseError::Unrecognized),
    }
  }
}

impl StateRequest {
  /// Attempts to build the state request from a slice of bytes.
  pub fn from_bytes(input: &[u8]) -> Option<Self> {
    let utf8_representation = core::str::from_utf8(input).ok()?;
    utf8_representation.parse().ok()
  }

  /// Given a constant number of leds to fill, this method will return an _array_ of colors for
  /// each one corresponding to its matching value in the request.
  pub fn colors<const M: usize>(&self) -> smart_leds::Brightness<<[smart_leds::RGB8; M] as IntoIterator>::IntoIter> {
    let mut out = [smart_leds::RGB8::new(0, 0, 0); M];

    for item in out.iter_mut().take(M) {
      *item = match self {
        Self::On => smart_leds::RGB8::new(255, 255, 255),
        Self::Off => smart_leds::RGB8::new(0, 0, 0),
        Self::Red => smart_leds::RGB8::new(255, 0, 0),
        Self::Green => smart_leds::RGB8::new(0, 255, 0),
        Self::Blue => smart_leds::RGB8::new(0, 0, 255),
      }
    }

    smart_leds::brightness(out.into_iter(), 100)
  }
}

/// The response type implements `core::fmt::Display` and enumerates the possible strings that we
/// will send back over our serial connection to the client.
pub enum Response {
  /// The requested action succeeded.
  Roger,

  /// The requested action failed.
  Failed,
}

impl core::fmt::Display for Response {
  fn fmt(&self, formatter: &mut core::fmt::Formatter) -> core::fmt::Result {
    match self {
      Self::Roger => write!(formatter, "ok\r\n"),
      Self::Failed => write!(formatter, "failed\r\n"),
    }
  }
}
