use std::collections::HashMap;
use std::io::{Error, ErrorKind};

use serde::{Deserialize, Serialize};
use tide::{Request, Response, Result};

use crate::{heartbeat::HeartControl, octoprint::OctoprintJobResponse, server::sec::Claims, server::State};

#[derive(Debug, Serialize)]
struct ControlResponse {
  ok: bool,
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

#[derive(Debug, Serialize)]
struct ControlPatternResponse {
  ok: bool,
  timestamp: chrono::DateTime<chrono::Utc>,
  name: String,
}

impl Default for ControlPatternResponse {
  fn default() -> Self {
    Self {
      ok: true,
      timestamp: chrono::Utc::now(),
      name: String::new(),
    }
  }
}

#[derive(Default, Debug, Deserialize)]
struct ControlPatternWriteColorAssignmentPayload {
  hex: String,
  ledn: u8,
}

#[derive(Default, Debug, Deserialize)]
struct ControlPatternWriteFramePayload {
  colors: Vec<ControlPatternWriteColorAssignmentPayload>,
}

#[derive(Default, Debug, Deserialize)]
struct ControlPatternWritePayload {
  frames: Vec<ControlPatternWriteFramePayload>,
}

#[derive(Default, Debug, Deserialize)]
struct ControlQuery {
  mode: String,
  pattern: Option<String>,
}

#[inline]
fn parse_hex(input: &String) -> Option<(u8, u8, u8)> {
  let mut it = input
    .chars()
    .step_by(2)
    .skip(1)
    .enumerate()
    .map(|(indx, _)| input[(indx * 2) + 1..(indx * 2) + 3].to_string())
    .map(|val| u8::from_str_radix(&val, 16).ok())
    .flatten();

  it.next().zip(it.next()).zip(it.next()).map(|((r, g), b)| (r, g, b))
}

// ROUTE: attempt to create a new pattern.
pub async fn write_pattern(mut request: Request<State>) -> Result {
  let oid = super::cookie(&request)
    .and_then(|cook| Claims::decode(&cook.value()).ok())
    .map(|claims| claims.oid)
    .ok_or_else(|| {
      log::warn!("unauthorized attempt to query state");
      tide::Error::from_str(404, "not-found")
    })?;

  request.state().authority(&oid).await.ok_or_else(|| {
    log::warn!("unauthorized attempt to commit command");
    tide::Error::from_str(404, "not-found")
  })?;

  let payload = request
    .body_json::<ControlPatternWritePayload>()
    .await
    .map_err(|error| {
      log::warn!("invalid pattern write payload - {}", error);
      tide::Error::from_str(422, "bad-payload")
    })?;

  let name = uuid::Uuid::new_v4().to_string();
  log::info!("user '{}' attempting to write a pattern - {:?}", oid, name);

  let pattern = payload
    .frames
    .into_iter()
    .enumerate()
    .fold(HashMap::new(), |mut acc, (indx, frame)| {
      let current = frame.colors.into_iter().fold(HashMap::new(), |mut acc, col| {
        let color = parse_hex(&col.hex);
        if let Some(real) = color {
          acc.insert(col.ledn, blinkrs::Color::Three(real.0, real.1, real.2));
        } else {
          log::warn!("unable to parse hex string '{}'", col.hex);
        }
        acc
      });

      if indx < (u8::MAX as usize) {
        acc.insert(indx as u8, current);
        acc
      } else {
        log::warn!("too many frames (max {})", u8::MAX);
        acc
      }
    });

  request
    .state()
    .heart
    .send(HeartControl::Save(name.clone(), pattern))
    .await
    .map_err(|error| {
      log::warn!("unable to issue heartbeat save command - {}", error);
      tide::Error::from_str(500, "bad-heart")
    })?;

  request
    .state()
    .heart
    .send(HeartControl::Load(name.clone()))
    .await
    .map_err(|error| {
      log::warn!("unable to issue heartbeat load command - {}", error);
      tide::Error::from_str(500, "bad-heart")
    })?;

  let res = ControlPatternResponse {
    name: name,
    ..Default::default()
  };
  tide::Body::from_json(&res).map(|bod| Response::builder(200).body(bod).build())
}

// ROUTE: proxy to octoprint (mjpg-streamer) snapshot url
pub async fn snapshot(request: Request<State>) -> Result {
  let claims = super::cookie(&request)
    .and_then(|cook| Claims::decode(&cook.value()).ok())
    .ok_or_else(|| {
      log::warn!("unauthorized attempt to query state");
      tide::Error::from_str(404, "not-found")
    })?;

  log::info!("fetching snapshot for user {}", claims.oid);

  let response = surf::get(std::env::var("OCTOPRINT_SNAPSHOT_URL").map_err(|error| {
    log::warn!("unable to find OCTOPRINT_SNAPSHOT_URL in env - {}", error);
    tide::Error::from_str(404, "not-found")
  })?)
  .await
  .map_err(|error| {
    log::warn!("unable to request snapshot - {}", error);
    tide::Error::from_str(404, "not-found")
  })?;

  let size = response.len();
  Ok(
    tide::Response::builder(200)
      .content_type(tide::http::mime::JPEG)
      .body(tide::Body::from_reader(response, size))
      .build(),
  )
}

// ROUTE: fetches current job information from octoprint api
pub async fn query(req: Request<State>) -> Result {
  super::cookie(&req)
    .and_then(|cook| Claims::decode(&cook.value()).ok())
    .ok_or_else(|| {
      log::warn!("unauthorized attempt to query state");
      tide::Error::from_str(404, "not-found")
    })?;

  let mut res = surf::get(format!(
    "{}/api/job",
    std::env::var("OCTOPRINT_API_URL").map_err(|error| {
      log::warn!("unable to find OCTOPRINT_API_URL in environment - {}", error);
      tide::Error::from_str(500, "bad-config")
    })?
  ))
  .header(
    "X-Api-Key",
    std::env::var("OCTOPRINT_API_KEY").map_err(|error| {
      log::warn!("unable to find OCTOPRINT_API_KEY in environment - {}", error);
      tide::Error::from_str(500, "bad-config")
    })?,
  )
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

// ROUTE: sends command to heartbeat/light controls.
pub async fn command(mut req: Request<State>) -> Result {
  let claims = super::cookie(&req)
    .and_then(|cook| Claims::decode(&cook.value()).ok())
    .ok_or_else(|| {
      log::warn!("unauthorized attempt to commit command");
      tide::Error::from_str(404, "not-found")
    })?;

  req.state().authority(&claims.oid).await.ok_or_else(|| {
    log::warn!("unauthorized attempt to commit command");
    tide::Error::from_str(404, "not-found")
  })?;

  let query = req.body_json::<ControlQuery>().await.map_err(|error| {
    log::warn!("uanble to parse control payload - {}", error);
    tide::Error::from_str(422, "bad-payload")
  })?;

  log::debug!("received control request - {:?}", query);

  let result = match query.mode.as_str() {
    "off" => req.state().heart.send(HeartControl::Stop).await.map(|_| ()),
    "on" => req.state().heart.send(HeartControl::Start).await.map(|_| ()),
    "load" => {
      let name = query.pattern.ok_or(Error::new(ErrorKind::Other, "missing name"))?;
      req.state().heart.send(HeartControl::Load(name)).await.map(|_| ())
    }
    unknown => {
      log::warn!("unrecognized control input from payload - '{}'", unknown);
      Ok(())
    }
  };

  if let Err(error) = result {
    log::warn!("unable to send control message to heartbeat - {}", error);
  }

  tide::Body::from_json(&ControlResponse::default()).map(|bod| Response::builder(200).body(bod).build())
}

#[cfg(test)]
mod tests {
  use super::parse_hex;

  #[test]
  fn test_parse_hex_bad_empty() {
    assert_eq!(parse_hex(&"".to_string()), None);
  }

  #[test]
  fn test_parse_hex_bad_odd() {
    assert_eq!(parse_hex(&"zzz".to_string()), None);
  }

  #[test]
  fn test_parse_hex_bad_not_hex() {
    assert_eq!(parse_hex(&"zzzzzz".to_string()), None);
  }

  #[test]
  fn test_parse_hex_red() {
    assert_eq!(parse_hex(&"#ff0000".to_string()), Some((255, 0, 0)));
  }

  #[test]
  fn test_parse_hex_green() {
    assert_eq!(parse_hex(&"#00ff00".to_string()), Some((0, 255, 0)));
  }

  #[test]
  fn test_parse_hex_blue() {
    assert_eq!(parse_hex(&"#0000ff".to_string()), Some((0, 0, 255)));
  }
}
