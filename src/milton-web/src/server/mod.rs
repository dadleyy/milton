use std::io::{Error, ErrorKind, Result};

use async_std::channel::Sender;
use serde::Deserialize;
use tide::{http::Cookie, Request, Response};
use v4l::io::traits::CaptureStream;
use v4l::video::Capture;

use crate::oauth;

/// The `sec` module holds our security/authenticated jwt-based types.
mod sec;

/// Routes and types related to authentication.
pub mod auth;
/// Routes and types related to system control.
pub mod control;

/// General type definition for side effects.
pub mod effects;

/// An authenticated user will have varying levels of authority. Currently the only distinction
/// we're making is an admin, to which all functionality is available.
pub(crate) enum Authority {
  /// Unlimited access.
  Admin,
}

/// This is a hodgepodge of config.
#[derive(Deserialize, Clone, Debug)]
pub struct Configuration {
  /// API root for octoprint. (e.g http://192.168.2.27:5000/api)
  octoprint_api_url: String,

  /// API key for octoprint. (e.g abcdef)
  octoprint_api_key: String,

  /// The location to send users _back_ to after successful oauth exchanges.
  auth_complete_uri: String,

  /// The secret that will be used to sign jwt tokens.
  jwt_secret: String,

  /// The redis host that we will use for session storage.
  redis_host: String,

  /// The redis port that we will use for session storage.
  redis_port: u32,

  /// The domain we're hosting from; used for cookies.
  domain: String,

  /// The kernel managed device path compatible with v4l.
  video_device: Option<String>,
}

/// The builder-pattern impl for our shared `State` type.
#[derive(Default, Clone)]
pub struct StateBuilder {
  /// Outbound channel for side effects.
  sender: Option<Sender<effects::Effects>>,

  /// Auth0 config.
  oauth: Option<oauth::AuthZeroConfig>,

  /// General, misc config. Needs cleaning.
  config: Option<Configuration>,

  /// The `version` field is expected to be populated from the `MILTON_VERSION` value at compile
  /// time.
  version: Option<String>,
}

impl StateBuilder {
  /// Populates the oauth config.
  pub fn oauth(mut self, conf: oauth::AuthZeroConfig) -> Self {
    self.oauth = Some(conf);
    self
  }

  /// Populates the ui config.
  pub fn config(mut self, config: Configuration) -> Self {
    self.config = Some(config);
    self
  }

  /// Populates the side effect channel.
  pub fn sender(mut self, chan: Sender<effects::Effects>) -> Self {
    self.sender = Some(chan);
    self
  }

  /// Populates the version value.
  pub fn version(mut self, version: String) -> Self {
    self.version = Some(version);
    self
  }

  /// Validates and returns a `State` instance.
  pub fn build(self) -> Result<State> {
    let sender = self
      .sender
      .ok_or_else(|| Error::new(ErrorKind::Other, "missing sender"))?;
    let oauth = self
      .oauth
      .ok_or_else(|| Error::new(ErrorKind::Other, "missing oauth config"))?;
    let config = self
      .config
      .ok_or_else(|| Error::new(ErrorKind::NotFound, "no ui config found"))?;

    Ok(State {
      sender,
      oauth,
      config,

      version: self
        .version
        .ok_or_else(|| Error::new(ErrorKind::NotFound, "no version provided"))?,

      redis: async_std::sync::Arc::new(async_std::sync::Mutex::new(None)),

      video_data: async_std::sync::Arc::new(async_std::sync::RwLock::new((None, Vec::with_capacity(0)))),
    })
  }
}

/// The `State` here represents all shared types that are used across web requests. Requires that
/// this is `clone`-able.
#[derive(Clone)]
pub struct State {
  /// The outbound effect channel that will be used to send side effects from web requests to a
  /// central effect manager.
  sender: Sender<effects::Effects>,

  /// General configuration. Should probably be cleaned up.
  pub(crate) config: Configuration,

  /// Auth0 credentials (client ids, secrets, etc...)
  pub(crate) oauth: oauth::AuthZeroConfig,

  /// Compiler time version value.
  pub(crate) version: String,

  /// A shared tcp connection to our redis connection. This eventually should be expanded into a
  /// pool of available tcp connections.
  redis: async_std::sync::Arc<async_std::sync::Mutex<Option<async_std::net::TcpStream>>>,

  /// A shared reference to our video data, should a request need one.
  video_data: async_std::sync::Arc<async_std::sync::RwLock<(Option<std::time::Instant>, Vec<u8>)>>,
}

impl State {
  /// Helper method to return the builder pattern for this struct.
  pub fn builder() -> StateBuilder {
    StateBuilder::default()
  }

  /// Executes a redis command against our shared, mutex locked redis "pool".
  async fn command<S, V>(&self, command: kramer::Command<S, V>) -> Result<kramer::Response>
  where
    S: std::fmt::Display,
    V: std::fmt::Display,
  {
    let mut redis = self.redis.lock().await;

    let mut pulled_connection = match redis.take() {
      Some(inner) => inner,
      None => {
        let connection_addr = format!("{}:{}", self.config.redis_host, self.config.redis_port);
        async_std::net::TcpStream::connect(&connection_addr)
          .await
          .map_err(|error| {
            log::error!("failed establishing new connection to redis - {error}");
            error
          })?
      }
    };

    let output = kramer::execute(&mut pulled_connection, &command)
      .await
      .map_err(|error| {
        log::error!("unable to execute redis command - {error}");
        error
      })?;

    *redis = Some(pulled_connection);

    Ok(output)
  }

  /// This function is responsible for taking the unique id found in our session cookie and
  /// returning the user data that we have previously stored in redis.
  pub(crate) async fn user_from_session<T>(&self, id: T) -> Option<auth::AuthIdentifyResponseUserInfo>
  where
    T: std::fmt::Display,
  {
    // Look up our session by the uuid in our redis session store
    let serialized_id = format!("{id}");
    let command =
      kramer::Command::Strings::<&str, &str>(kramer::StringCommand::Get(kramer::Arity::One(&serialized_id)));

    let response = self
      .command(command)
      .await
      .map_err(|error| {
        log::error!("unable to fetch session info - {error}");
        error
      })
      .ok()?;

    // Attempt to deserialize as our user info structure.
    if let kramer::Response::Item(kramer::ResponseValue::String(inner)) = response {
      log::trace!("has session data - {inner:?}");
      return serde_json::from_str(&inner).ok();
    }

    None
  }

  /// Returns the authority level based on the session data provided by our cookie. This is
  /// verified against our external oauth (auth0) provider.
  pub(crate) async fn authority<T>(&self, id: T) -> Option<Authority>
  where
    T: std::fmt::Display,
  {
    let data = self.user_from_session(id).await?;

    if data.roles.into_iter().any(|role| role.is_admin()) {
      return Some(Authority::Admin);
    }

    None
  }

  /// Incoming web requests have the ability to create side effects that are handled elsewhere.
  /// This method wraps the inner `channel` send.
  pub(crate) async fn send(&self, effect: effects::Effects) -> Result<()> {
    self
      .sender
      .send(effect)
      .await
      .map_err(|error| Error::new(ErrorKind::Other, error))
  }
}

/// Returns the cookie responsible for holding our session from the request http header.
pub(crate) fn cookie(request: &Request<State>) -> Option<Cookie<'static>> {
  request.cookie(auth::COOKIE_NAME)
}

/// Returns the decoded jwd claims based on the cookie provided by an http request.
pub(crate) fn claims(request: &Request<State>) -> Option<sec::Claims> {
  let cook = cookie(request)?;
  sec::Claims::decode(&cook.value(), &request.state().config.jwt_secret).ok()
}

/// The catchall 404 handling route.
pub(crate) async fn missing(req: Request<State>) -> tide::Result {
  log::warn!("[warning] unknown request received - '{}'", req.url().path());
  Ok(Response::builder(404).build())
}

/// This is the main entry point for the http server responsible for setting up routes and binding
/// our shared state to the tcp listener.
pub async fn listen<S>(state: State, addr: S) -> std::io::Result<()>
where
  S: std::convert::AsRef<str>,
{
  if let Some(path) = &state.config.video_device {
    let dev = v4l::Device::with_path(&path)?;
    let mut has_support = false;

    'outer: for format in dev.enum_formats()? {
      for framesize in dev.enum_framesizes(format.fourcc)? {
        for discrete in framesize.size.to_discrete() {
          if format.fourcc == v4l::format::FourCC::new(b"MJPG") {
            log::info!("found mjpg compatible format on {path}");
            dev.set_format(&v4l::Format::new(
              discrete.width,
              discrete.width,
              v4l::format::FourCC::new(b"MJPG"),
            ))?;
            has_support = true;
            break 'outer;
          }
        }
      }
    }

    if has_support {
      let clone_ref = state.video_data.clone();
      let mut stream = v4l::prelude::MmapStream::with_buffers(&dev, v4l::buffer::Type::VideoCapture, 4)?;

      async_std::task::spawn(async move {
        log::info!("video data read thread active");
        let mut last_debug = std::time::Instant::now();

        let mut current_frames = 0;
        loop {
          let before = std::time::Instant::now();

          match stream.next() {
            Ok((buffer, meta)) => {
              let after = std::time::Instant::now();
              let seconds_since = before.duration_since(last_debug).as_secs();
              current_frames += 1;

              let mut writable_reference = clone_ref.write().await;
              // log::debug!("read video device meta - {}bytes", meta.bytesused);
              *writable_reference = (Some(std::time::Instant::now()), buffer.to_vec());
              drop(writable_reference);

              if seconds_since > 3 {
                let frame_read_time = after.duration_since(before).as_millis();
                log::info!(
                  "{current_frames}f ({seconds_since}s) {frame_read_time}ms per {}bytes",
                  meta.bytesused
                );
                last_debug = before;
                current_frames = 0;
              }
            }
            Err(error) => {
              log::error!("unable to read next stream from video device - {error}");
              async_std::task::sleep(std::time::Duration::from_millis(500)).await;
            }
          }
        }
      });
    }
  }

  let mut app = tide::with_state(state);

  app.at("/control").post(control::command);
  app.at("/control").get(control::query);
  app.at("/control/snapshot").get(control::snapshot);

  app.at("/auth/start").get(auth::start);
  app.at("/auth/end").get(auth::end);
  app.at("/auth/complete").get(auth::complete);
  app.at("/auth/identify").get(auth::identify);

  app.at("/*").all(missing);
  app.listen(addr.as_ref()).await
}
