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

#[derive(Default, Debug, Deserialize)]
struct ControlQuery {
  mode: String,
  code: String,
  pattern: Option<String>,
}

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

  Ok("".into())
}

pub async fn command(mut req: Request<State>) -> Result {
  super::cookie(&req)
    .and_then(|cook| Claims::decode(&cook.value()).ok())
    .ok_or_else(|| {
      log::warn!("unauthorized attempt to commit command");
      tide::Error::from_str(404, "not-found")
    })?;

  let query = req.body_json::<ControlQuery>().await.map_err(|error| {
    log::warn!("uanble to parse control payload - {}", error);
    tide::Error::from_str(422, "bad-payload")
  })?;

  let actual = std::env::var("HEARTBEAT_SECRET_CODE").unwrap_or_default();

  if actual != query.code {
    log::warn!("unauthorized attempt to set heartbeat");
    return Ok(Response::builder(404).body("not-found").build());
  }

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
