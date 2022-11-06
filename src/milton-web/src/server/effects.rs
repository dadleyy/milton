/// This enumerated type represents all of the "outbound" effects that can be created from a web
/// request.
#[derive(Debug)]
pub enum Effects {
  /// `Lights` effects are used to control the led strip; sent to `pio-lights` firmware.
  Lights(crate::lights::Command),
}
