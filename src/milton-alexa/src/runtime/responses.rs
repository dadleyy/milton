//! Define a type that will help abstract away the generation of our response payloads so we dont
//! have as much clutter in our actual route handlers.

/// Abstracts over the response kinds we want to send back to alexa.
#[derive(Debug)]
pub(super) enum EventResponse {
  /// Generates the `SetBinaryStateResponse` payload when formatted.
  SetState(bool),

  /// Generates the `GetBinaryStateResponse` payload when formatted.
  GetState(bool),
}

// [todo] clean this up?
impl std::fmt::Display for EventResponse {
  fn fmt(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
    match self {
      Self::SetState(value) => write!(
        formatter,
        r#"<s:Envelope xmlns:s="http://schemas.xmlsoap.org/soap/envelope/" s:encodingStyle="http://schemas.xmlsoap.org/soap/encoding/">
        <s:Body>
            <u:SetBinaryStateResponse xmlns:u="urn:Belkin:service:basicevent:1">
                <BinaryState>{}</BinaryState>
            </u:SetBinaryStateResponse>
        </s:Body>
    </s:Envelope>"#,
        if *value { "1" } else { "0" }
      ),
      Self::GetState(value) => write!(
        formatter,
        r#"<s:Envelope xmlns:s="http://schemas.xmlsoap.org/soap/envelope/" s:encodingStyle="http://schemas.xmlsoap.org/soap/encoding/">
        <s:Body>
            <u:GetBinaryStateResponse xmlns:u="urn:Belkin:service:basicevent:1">
                <BinaryState>{}</BinaryState>
            </u:GetBinaryStateResponse>
        </s:Body>
    </s:Envelope>"#,
        if *value { "1" } else { "0" }
      ),
    }
  }
}
