use serde::Deserialize;

/// Deserializable configuration data.
#[derive(Deserialize, Clone)]
pub struct Config {
  /// This is the location that will be used in the discovery udp socket. It should be the url of
  /// where alexa can find the `setup.xml` document. For example:
  /// `http://192.168.1.39:12340/setup.xml`
  pub(crate) setup_location: String,

  /// The location of a `setup.xml` file that will be provided to alexa.
  pub(crate) setup_file: String,

  /// "Unique Service Name". This will be included in the headers returned from the discovery udp
  /// socket when alexa requests them. This value should match the `<UDN>` value in the `setup.xml`
  /// file.
  pub(crate) usn: String,

  /// A unique id that will be used in the headers sent back from our discovery/udp socket handler.
  pub(crate) device_id: String,

  /// A friendly name; this is included in the headers sent back from our discovery/udp socket
  /// handler.
  pub(crate) server: String,

  /// The location where we can find the main `milton-web` application running.
  pub(crate) milton_addr: String,

  /// The admin token we'll use to send requests to milton.
  pub(crate) milton_token: String,
}
