#![allow(clippy::missing_docs_in_private_items)]

//! These types represent the schema of misc. octoprint related json responses.

use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct OctoprintJobFile {
  pub name: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct OctoprintJob {
  pub file: Option<OctoprintJobFile>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct OctoprintJobProgress {
  pub completion: Option<f64>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct OctoprintJobResponse {
  pub job: Option<OctoprintJob>,
  pub progress: Option<OctoprintJobProgress>,
  pub state: Option<String>,
}
