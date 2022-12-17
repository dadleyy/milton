//! todo: this is very much a work-in-progress. The goal was to implement something as quickly as
//! possible to get a functioning alexa -> milton-web integration up and running as quickly as
//! possible.

use serde::Serialize;
use std::io;

/// Abstractions to provide deserialization of messages sent from alexa.
mod requests;

/// Abstractions to provide serialization of state to be sent to alexa.
mod responses;

/// Shared state across thread/web-request boundaries.
#[derive(Clone)]
struct Runtime {
  /// Our original, immutable configuration.
  config: async_std::sync::Arc<crate::config::Config>,

  /// The last state we figured.
  state: async_std::sync::Arc<async_std::sync::Mutex<bool>>,
}

/// The most basic, on/off control request.
#[derive(Debug, Serialize)]
struct StateControlQuery {
  /// Whether or not the lights should be on.
  on: bool,
}

/// This type is used to represent the various json payloads supported by the "direct" control api
/// route.
#[derive(Debug, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum ControlQuery {
  /// Will control on/off.
  State(StateControlQuery),
}

/// The `setup.xml` handler.
async fn setup(request: tide::Request<Runtime>) -> tide::Result {
  log::info!("returning setup xml document");
  let body = tide::Body::from_file(&request.state().config.setup_file).await?;

  Ok(tide::Response::builder(200).content_type("text/xml").body(body).build())
}

/// A generic 404 handler with some logging so we can see what alexa is poking around at.
async fn not_found<S>(request: tide::Request<S>) -> tide::Result {
  log::warn!("404 - {:?} @ {:?}", request.method(), request.url());
  Ok(tide::Response::builder(404).build())
}

/// This is the main event handler that actually receives the requests from alexa at a user's
/// request.
async fn basicevent(mut request: tide::Request<Runtime>) -> tide::Result {
  let body = request.body_string().await.map_err(|error| {
    log::warn!("unable to read basic event payload as string - {error}");
    error
  })?;
  let operation = body.parse::<requests::StateOperation>().map_err(|error| {
    log::warn!("failed basicevent payload parsing - {error}");
    error
  })?;
  let state = request.state();

  log::debug!("found operation - {operation:?}");

  let xml_body = match operation {
    requests::StateOperation::SetState(Some(new_value)) => {
      let mut unlocked_state = request.state().state.lock().await;
      match new_value.as_str() {
        value @ "0" | value @ "1" => *unlocked_state = value == "1",
        _ => (),
      }

      let url = format!(
        "{}/control?_admin_token={}",
        state.config.milton_addr, state.config.milton_token
      );
      let request = surf::post(&url).body_json(&ControlQuery::State(StateControlQuery { on: *unlocked_state }));

      match request {
        Ok(req) => match req.await {
          Err(error) => log::warn!("unable to send - {error}"),
          Ok(_) => log::info!("sent request to milton-web"),
        },
        Err(error) => {
          log::warn!("unable to serialize request {error}");
        }
      }

      format!("{}", responses::EventResponse::SetState(*unlocked_state))
    }

    requests::StateOperation::GetState(Some(_)) => {
      let unlocked_state = request.state().state.lock().await;
      format!("{}", responses::EventResponse::GetState(*unlocked_state))
    }

    requests::StateOperation::GetState(None) | requests::StateOperation::SetState(None) => {
      log::warn!("basic event payload parsing did not finish");
      return Ok(tide::Response::builder(422).build());
    }
  };

  log::debug!("sending - {xml_body}");

  Ok(
    tide::Response::builder(200)
      .content_type("text/xml")
      .body(xml_body)
      .build(),
  )
}

/// The main entrypoint for our "runtime" side of the service. This task is concerned with setting
/// up our TCP listener and dealing with requests once alexa has found us.
pub async fn application(config: &crate::config::Config) -> io::Result<()> {
  log::info!("application task started");
  let mut app = tide::with_state(Runtime {
    config: async_std::sync::Arc::new(config.clone()),
    state: async_std::sync::Arc::new(async_std::sync::Mutex::new(true)),
  });

  app.at("/setup.xml").get(setup);
  app.at("/upnp/control/basicevent1").post(basicevent);

  app.at("*").all(not_found);
  if let Err(error) = app.listen("0.0.0.0:12340").await {
    log::warn!("failed application task - {error}");
    return Err(error);
  }
  Ok(())
}
