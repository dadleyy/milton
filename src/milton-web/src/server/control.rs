use serde::{Deserialize, Serialize};
use tide::{Request, Response, Result};

use crate::{octoprint::OctoprintJobResponse, server::State};

/// The stream endpoint will use this as the http multi-part boundary for its mjpg stream.
const MJPG_BOUNDARY: &str = "mjpg-boundary-do-not-cross";

/// Requests to the control api will receive this type serialized as json.
#[derive(Debug, Serialize)]
struct ControlResponse {
  /// Was the command sent successfully.
  ok: bool,

  /// The current time.
  timestamp: chrono::DateTime<chrono::Utc>,
}

impl Default for ControlResponse {
  fn default() -> Self {
    Self {
      ok: true,
      timestamp: chrono::Utc::now(),
    }
  }
}

/// The most basic, on/off control request.
#[derive(Debug, Deserialize)]
struct StateControlQuery {
  /// Whether or not the lights should be on.
  on: bool,
}

/// Wraps the `lights` module supported colors in a type easily deserialized from json.
#[derive(Debug, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
struct ColorControlQuery {
  /// The color to set.
  color: crate::lights::BasicColor,
}

/// This type is used to represent the various json payloads supported by the "direct" control api
/// route.
#[derive(Debug, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum ControlQuery {
  /// Will control on/off.
  State(StateControlQuery),

  /// Will control basic color.
  BasicColor(ColorControlQuery),
}

/// Accessing the snapshot endpoint is something that we'd like octoprint to be able to do, in
/// addition to users authorized through the ui via http cookies.
#[derive(Deserialize, Debug)]
struct SnapshotUrlQuery {
  /// The optional authorization token we're using to access the control mjpg stream.
  token: Option<String>,
}

// TODO: this will be useful once we're able to control specific colors. blocked by firmware.
// fn parse_hex(input: &String) -> Option<(u8, u8, u8)> {
//   let mut results = (1..input.len())
//     .step_by(2)
//     .map(|i| u8::from_str_radix(&input[i..i + 2], 16).ok())
//     .flatten()
//     .collect::<Vec<u8>>();
//
//   results
//     .pop()
//     .zip(results.pop())
//     .zip(results.pop())
//     .map(|((r, g), b)| (r, g, b))
// }

/// ROUTE: return jpeg snapshot
pub async fn snapshot(request: Request<State>) -> Result {
  // TODO: replace this with a more robust application auth token storage + validation system.
  match request.query::<SnapshotUrlQuery>() {
    Ok(query) if query.token.is_some() && query.token == request.state().config.octoprint_stream_token => {
      log::info!("authorizing stream as octoprint");
    }
    _ => {
      let claims = super::claims(&request).ok_or_else(|| {
        log::warn!("unauthorized attempt to access camera control");
        tide::Error::from_str(404, "not-found")
      })?;

      if request.state().authority(&claims.oid).await.is_none() {
        return Ok(tide::Response::new(404));
      }
    }
  }

  let frame_reader = request.state().video.data.read().await;
  let buffer = frame_reader.1.clone();
  drop(frame_reader);

  // Prepare the response with the correct header
  Ok(
    tide::Response::builder(200)
      .header("Access-Control-Allow-Origin", "*")
      .content_type("image/jpeg")
      .body(buffer)
      .build(),
  )
}

/// ROUTE: mjpeg stream
pub async fn stream(request: Request<State>) -> Result {
  // TODO: replace this with a more robust application auth token storage + validation system.
  match request.query::<SnapshotUrlQuery>() {
    Ok(query) if query.token.is_some() && query.token == request.state().config.octoprint_stream_token => {
      log::info!("authorizing stream as octoprint");
    }
    _ => {
      let claims = super::claims(&request).ok_or_else(|| {
        log::warn!("unauthorized attempt to access camera control");
        tide::Error::from_str(404, "not-found")
      })?;

      if request.state().authority(&claims.oid).await.is_none() {
        return Ok(tide::Response::new(404));
      }
    }
  }

  let semaphores = match &request.state().video.semaphores {
    None => return Ok(tide::Response::new(404)),
    Some(sema) => sema,
  };

  // Create our semaphore channel and send it back out to our main server thread.
  let (ss, sr) = async_std::channel::unbounded();
  semaphores.send(ss).await?;

  // Create the channel whose receiver will be used as a async reader.
  let (writer, drain) = async_std::channel::bounded(2);
  let buf_drain = futures::stream::TryStreamExt::into_async_read(drain);

  // Prepare the response with the correct header
  let response = tide::Response::builder(200)
    .header("Access-Control-Allow-Origin", "*")
    .content_type(format!("multipart/x-mixed-replace;boundary={MJPG_BOUNDARY}").as_str())
    .body(tide::Body::from_reader(buf_drain, None))
    .build();

  // In a separate task, continously check our shared buffer's timestamp. If that value differs
  // from the timestamp of the last message sent on our end, send a new multipart chunk.
  async_std::task::spawn(async move {
    log::debug!("spawned client mjpeg stream handler for {:?}", request.remote());
    let mut last_debug = std::time::Instant::now();
    let mut last_frame = None;
    let mut sent_frames = 0u32;

    loop {
      if let Err(error) = sr.recv().await {
        log::warn!("unable to receive on semaphore channel - {error}");
        break;
      }

      // Unlock our data mutex and read.
      let frame_reader = request.state().video.data.read().await;
      if last_frame.is_some() && last_frame == frame_reader.0 {
        log::warn!(
          "stale frame sent past semaphore ({last_frame:?} vs {:?}",
          frame_reader.0
        );
        drop(frame_reader);
        continue;
      }

      // Start the buffer that we'll send using the boundary and some multi-part http header
      // context.
      let mut buffer = format!(
        "--{MJPG_BOUNDARY}\r\nContent-Type: image/jpeg\r\nContent-Length: {}\r\n\r\n",
        frame_reader.1.len(),
      )
      .into_bytes();

      // Actually push the JPEG data into our buffer.
      buffer.extend_from_slice(frame_reader.1.as_slice());

      // Update this thread's reference to the last frame sent.
      last_frame = (*frame_reader).0;
      drop(frame_reader);

      let now = std::time::Instant::now();
      sent_frames += 1;

      if now.duration_since(last_debug).as_secs() > 3 {
        log::info!("{sent_frames} frames in 3 seconds");
        last_debug = now;
        sent_frames = 0;
      }

      if let Err(error) = writer.send(Ok(buffer)).await {
        log::warn!("unable to send received data - {error}");
        break;
      }
    }
  });

  Ok(response)
}

/// ROUTE: fetches current job information from octoprint api
pub async fn query(req: Request<State>) -> Result {
  super::claims(&req).ok_or_else(|| {
    log::warn!("unauthorized attempt to query state");
    tide::Error::from_str(404, "not-found")
  })?;

  let mut res = surf::get(format!("{}/api/job", &req.state().config.octoprint_api_url))
    .header("X-Api-Key", &req.state().config.octoprint_api_key)
    .await
    .map_err(|error| {
      log::warn!("unable to issue request to octoprint - {}", error);
      tide::Error::from_str(500, "bad-config")
    })?;

  if res.status() != surf::StatusCode::Ok {
    log::warn!("bad octoprint response status - '{:?}'", res.status());
    return Err(tide::Error::from_str(500, "bad-config"));
  }

  let infos = res.body_json::<OctoprintJobResponse>().await.map_err(|error| {
    log::warn!("invalid response from octoprint - {}", error);
    tide::Error::from_str(500, "bad-config")
  })?;

  log::info!("requested octoprint current job info - {:?}", infos);
  tide::Body::from_json(&infos).map(|bod| Response::builder(200).body(bod).build())
}

/// ROUTE: sends command to heartbeat/light controls.
pub async fn command(mut req: Request<State>) -> Result {
  let mut timer = std::time::Instant::now();
  let claims = super::claims(&req).ok_or_else(|| {
    log::warn!("unauthorized attempt to commit command");
    tide::Error::from_str(404, "not-found")
  })?;

  req.state().authority(&claims.oid).await.ok_or_else(|| {
    log::warn!("unauthorized attempt to commit command");
    tide::Error::from_str(404, "not-found")
  })?;

  log::debug!(
    "loaded session authority in {} millis",
    std::time::Instant::now().duration_since(timer).as_millis()
  );

  timer = std::time::Instant::now();

  let query = req.body_json::<ControlQuery>().await.map_err(|error| {
    log::warn!("unable to parse control payload - {}", error);
    tide::Error::from_str(422, "bad-payload")
  })?;

  log::debug!("received control request - {:?}", query);

  let effect = match query {
    ControlQuery::BasicColor(ColorControlQuery { color }) => {
      super::effects::Effects::Lights(crate::lights::Command::BasicColor(color))
    }
    ControlQuery::State(target_state) => {
      if target_state.on {
        super::effects::Effects::Lights(crate::lights::Command::On)
      } else {
        super::effects::Effects::Lights(crate::lights::Command::Off)
      }
    }
  };

  if let Err(error) = req.state().send(effect).await {
    log::warn!("unable to send control effect - {error}");
    return Ok(tide::Response::new(500));
  }

  log::debug!(
    "sent control effect in {} millis",
    std::time::Instant::now().duration_since(timer).as_millis()
  );

  tide::Body::from_json(&ControlResponse::default()).map(|bod| Response::builder(200).body(bod).build())
}
