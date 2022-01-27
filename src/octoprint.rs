use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct OctoprintJobFile {
  pub name: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct OctoprintJob {
  pub file: Option<OctoprintJobFile>,
}

#[derive(Debug, Deserialize)]
pub struct OctoprintJobProgress {
  pub completion: Option<f64>,
}

#[derive(Debug, Deserialize)]
pub struct OctoprintJobResponse {
  pub job: Option<OctoprintJob>,
  pub progress: Option<OctoprintJobProgress>,
  pub state: Option<String>,
}
