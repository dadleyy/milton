use std::io;

/// The various kinds of messages we will receive from alexa.
#[derive(Debug)]
pub enum StateOperation {
  /// Attempts to get state. Frequently sent by alexa.
  GetState(Option<String>),

  /// Attempts to set state.
  SetState(Option<String>),
}

impl std::str::FromStr for StateOperation {
  type Err = io::Error;

  fn from_str(input: &str) -> Result<Self, Self::Err> {
    let mut reader = quick_xml::Reader::from_str(input);
    reader.trim_text(true);

    let mut found_payload = 0u8;
    let mut inner_payload = None;

    loop {
      match reader.read_event() {
        Ok(quick_xml::events::Event::Eof) => break Err(io::Error::new(io::ErrorKind::Other, "missing-operation")),
        Ok(quick_xml::events::Event::Text(start)) if found_payload == 1u8 => {
          if let Ok(value) = start.unescape() {
            inner_payload = Some(value.into());
            found_payload = 2u8;
            log::trace!("found text - {start:?}");
            continue;
          }
          found_payload = 3u8;
          log::warn!("invalid text node after binary state node");
        }
        Ok(quick_xml::events::Event::Start(start)) => {
          let local_name = start.local_name();
          let as_string = std::str::from_utf8(local_name.as_ref());
          match as_string {
            Ok("BinaryState") => {
              found_payload = 1u8;
              continue;
            }
            _ => continue,
          }
        }
        Ok(quick_xml::events::Event::End(start)) => {
          let local_name = start.local_name();
          let as_string = std::str::from_utf8(local_name.as_ref());
          match as_string {
            Ok("GetBinaryState") => break Ok(StateOperation::GetState(inner_payload)),
            Ok("SetBinaryState") => break Ok(StateOperation::SetState(inner_payload)),
            _ => continue,
          }
        }
        Ok(_) => continue,
        Err(error) => {
          log::warn!("xml parse error - {error}");
          break Err(io::Error::new(io::ErrorKind::Other, "xml error - {error}"));
        }
      }
    }
  }
}
