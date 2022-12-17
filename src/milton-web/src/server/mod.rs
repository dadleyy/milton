//! This module contains the http json api server code. It is still a work-in-progress; things like
//! where the video streaming task and final route layout are still being figured out.

use std::io::{Error, ErrorKind, Result};

use async_std::channel::Sender;
use serde::{Deserialize, Serialize};
use tide::{http::Cookie, Request, Response};
#[cfg(feature = "camera")]
use v4l::io::traits::CaptureStream;
#[cfg(feature = "camera")]
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

#[cfg(feature = "camera")]
/// TODO: move this to a more general video module.
mod huffman;

/// An authenticated user will have varying levels of authority. Currently the only distinction
/// we're making is an admin, to which all functionality is available.
pub(crate) enum Authority {
  /// Unlimited access.
  Admin,
  /// Unlimited access, implies machine.
  AutomatedAdmin,
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

  /// The key with our redis instance where we will store tokens.
  token_store: String,

  /// The domain we're hosting from; used for cookies.
  domain: String,

  #[cfg(feature = "camera")]
  /// The kernel managed device path compatible with v4l.
  video_device: Option<String>,

  /// A special token to be used by octoprint for our mjpg stream endpoint. This should be a
  /// short-lived feature and replaced with a more robust application auth token system.
  octoprint_stream_token: Option<String>,
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

      video: VideoState {
        data: async_std::sync::Arc::new(async_std::sync::RwLock::new((None, Vec::with_capacity(0)))),
        semaphores: None,
      },
    })
  }
}

/// The video state is a shared structure representing our underlying data and channels used to
/// communicate frame readiness.
#[derive(Clone)]
struct VideoState {
  /// The underlying timestamp and data of our last video frame.
  data: async_std::sync::Arc<async_std::sync::RwLock<(Option<std::time::Instant>, Vec<u8>)>>,

  /// The sender of per-request semaphare channels whose receiver will be polled in our video frame
  /// loop and updated.
  semaphores: Option<async_std::channel::Sender<async_std::channel::Sender<()>>>,
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
  video: VideoState,
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

// TODO: There is a bit of awkward design between the shared state and these "floating" functions.
// Since these functions deal with the request itself, it felt a bit odd if there were functions on
// the `State` type itself, e.g:
//
// ```
// fn authority(&self, request: Request<Self>) -> Option<...>;
// ```

/// Returns the cookie responsible for holding our session from the request http header.
fn cookie(request: &Request<State>) -> Option<Cookie<'static>> {
  request.cookie(auth::COOKIE_NAME)
}

/// Returns the decoded JWT claims based on the cookie provided by an http request.
fn claims(request: &Request<State>) -> Option<sec::Claims> {
  let cook = cookie(request)?;
  sec::Claims::decode(&cook.value(), &request.state().config.jwt_secret).ok()
}

/// The minimal url query structure we need to deserialize into for automated admin authorization
/// status.
#[derive(Deserialize)]
struct QueryWithAdminToken {
  /// A special token lookup.
  _admin_token: String,
}

/// Given a request, this method will do what it can to figure out with what authority we are
/// working with.
pub(crate) async fn authority(request: &Request<State>) -> Option<Authority> {
  let state = request.state();

  if let Some(cookie_claims) = claims(request) {
    return state.authority(&cookie_claims.oid).await;
  }

  let admin_query = request.query::<QueryWithAdminToken>().ok()?;

  // Start by getting the list of our current tokens
  let get_command = kramer::Command::Hashes::<&str, &str>(kramer::HashCommand::Get(
    &state.config.token_store,
    Some(kramer::Arity::One("_admin")),
  ));
  let response = state.command(get_command).await.ok()?;

  if let kramer::Response::Item(kramer::ResponseValue::String(content)) = response {
    let parsed = serde_json::from_str::<Vec<String>>(&content)
      .map_err(|error| {
        log::warn!("unable to parse admin token store from redis - {error}");
        error
      })
      .ok()?;

    if parsed.contains(&admin_query._admin_token) {
      log::warn!("authorized as an automated admin via token");
      return Some(Authority::AutomatedAdmin);
    }

    log::error!(
      "dangerous attempt to authorized as admin using token '{}'",
      admin_query._admin_token
    );
  }

  None
}

#[derive(Serialize)]
struct Heartbeat<'a> {
  time: chrono::DateTime<chrono::Utc>,
  version: &'a String,
}

/// The heartbeat url.
async fn heartbeat(req: Request<State>) -> tide::Result {
  let body = tide::Body::from_json(&Heartbeat {
    time: chrono::Utc::now(),
    version: &req.state().version,
  })?;
  Ok(Response::builder(200).body(body).build())
}

/// The catchall 404 handling route.
async fn missing(req: Request<State>) -> tide::Result {
  log::warn!("[warning] unknown request received - '{}'", req.url().path());
  Ok(Response::builder(404).build())
}

#[allow(unused_mut)]
/// This is the main entry point for the http server responsible for setting up routes and binding
/// our shared state to the tcp listener.
pub async fn listen<S>(mut state: State, addr: S) -> std::io::Result<()>
where
  S: std::convert::AsRef<str>,
{
  #[cfg(feature = "camera")]
  if let Some(path) = &state.config.video_device {
    let dev = v4l::Device::with_path(path)?;
    let mut has_support = false;

    'outer: for format in dev.enum_formats()? {
      for framesize in dev.enum_framesizes(format.fourcc)? {
        for discrete in framesize.size.to_discrete() {
          if format.fourcc == v4l::format::FourCC::new(b"MJPG") {
            log::info!("found mjpg compatible format on {path}");
            dev.set_format(&v4l::Format::new(
              discrete.width,
              discrete.height,
              v4l::format::FourCC::new(b"MJPG"),
            ))?;
            has_support = true;
            break 'outer;
          }
        }
      }
    }

    if has_support {
      let clone_ref = state.video.clone();
      let mut stream = v4l::prelude::MmapStream::with_buffers(&dev, v4l::buffer::Type::VideoCapture, 4)?;

      let (sema_sender, sema_receiver) = async_std::channel::unbounded();
      state.video.semaphores = Some(sema_sender);

      async_std::task::spawn(async move {
        log::info!("video data read thread active");
        let mut last_debug = std::time::Instant::now();
        let mut current_frames = 0;
        let mut listeners = vec![];

        loop {
          let before = std::time::Instant::now();

          match stream.next() {
            Ok((buffer, meta)) => {
              let after = std::time::Instant::now();
              let seconds_since = before.duration_since(last_debug).as_secs();
              current_frames += 1;

              let mut normalized = Vec::with_capacity(buffer.len());
              let mut i = 0;

              while i < 2048 {
                if buffer[i] == 0xff && buffer[i + 1] == 0xC4 {
                  log::info!("found huffman in raw camera payload");
                  break;
                }

                // If we're at the start of "start of frame" marker, toss our default huffman table into
                // the buffer.
                if buffer[i] == 0xff && buffer[i + 1] == 0xC0 {
                  normalized.extend_from_slice(&huffman::HUFFMAN);
                  break;
                }

                normalized.push(buffer[i]);
                i += 1;
              }

              // Copy the remainder of our buffer into the normalized data.
              normalized.extend_from_slice(&buffer[i..meta.bytesused as usize]);

              let mut writable_reference = clone_ref.data.write().await;
              *writable_reference = (Some(std::time::Instant::now()), normalized);
              drop(writable_reference);

              // See if we have any new web connections waiting to register their semaphore receivers.
              if let Ok(lisener) = sema_receiver.try_recv() {
                listeners.push(lisener);
              }

              // Iterate over any listener, sending our semaphore alone.
              if !listeners.is_empty() {
                let mut next = vec![];

                for listener in listeners.drain(0..) {
                  if listener.is_closed() {
                    continue;
                  }

                  // Keep this semaphore channel around if we were able to send.
                  if listener.send(()).await.is_ok() {
                    next.push(listener);
                  }
                }

                listeners = next;
              }

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

  app.at("/status").get(heartbeat);

  app.at("/control").post(control::command);
  app.at("/control").get(control::query);
  app.at("/control/video-stream").get(control::stream);
  app.at("/control/video-snapshot").get(control::snapshot);

  app.at("/auth/start").get(auth::start);
  app.at("/auth/end").get(auth::end);
  app.at("/auth/complete").get(auth::complete);
  app.at("/auth/identify").get(auth::identify);

  app.at("/*").all(missing);
  app.listen(addr.as_ref()).await
}
