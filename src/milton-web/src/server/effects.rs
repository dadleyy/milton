#[derive(Debug)]
pub enum Effects {
  Lights(crate::lights::Command),
}
