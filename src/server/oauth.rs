use std::io::{Error, ErrorKind, Result};

use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct AuthCodeRequest {
  grant_type: String,
  client_id: String,
  client_secret: String,
  redirect_uri: Option<String>,
  code: Option<String>,
}

#[derive(Debug, Clone)]
pub struct AuthZeroConfig {
  auth_client_id: String,
  auth_client_secret: String,
  management_client_id: String,
  management_client_secret: String,
  redirect_uri: String,
  domain: String,
}

impl AuthZeroConfig {
  pub fn token_uri(&self) -> Result<String> {
    let base = format!("{}/oauth/token", self.domain);
    Ok(base)
  }

  pub fn auth_token_payload(&self, code: &String) -> Result<AuthCodeRequest> {
    Ok(AuthCodeRequest {
      client_id: self.auth_client_id.clone(),
      client_secret: self.auth_client_secret.clone(),
      code: Some(code.clone()),
      redirect_uri: Some(self.redirect_uri.clone()),
      grant_type: "authorization_code".into(),
    })
  }

  pub fn redirect_uri(&self) -> Result<String> {
    let base = format!("{}/authorize", self.domain);
    tide::http::Url::parse_with_params(
      &base,
      &[
        ("client_id", self.auth_client_id.as_str()),
        ("redirect_uri", self.redirect_uri.as_str()),
        ("response_type", &"code"),
        ("scope", &"openid profile email"),
      ],
    )
    .map_err(|error| {
      log::warn!("unable to build redirect uri - {}", error);
      Error::new(ErrorKind::Other, "bad-oauth-redirect-uri")
    })
    .map(|url| url.to_string())
  }
}

impl AuthZeroConfig {
  pub fn from_env() -> Result<Self> {
    let auth_client_id = std::env::var("AUTH_0_CLIENT_ID").map_err(|error| {
      log::warn!("unable to find auth 0 client id - {}", error);
      Error::new(ErrorKind::Other, "missing-client-id")
    })?;
    let auth_client_secret = std::env::var("AUTH_0_CLIENT_SECRET").map_err(|error| {
      log::warn!("unable to find auth 0 client secret - {}", error);
      Error::new(ErrorKind::Other, "missing-client-secret")
    })?;
    let management_client_id = std::env::var("AUTH_0_MANAGEMENT_CLIENT_ID").map_err(|error| {
      log::warn!("unable to find auth 0 management client id - {}", error);
      Error::new(ErrorKind::Other, "missing-management-client-secret")
    })?;
    let management_client_secret = std::env::var("AUTH_0_MANAGEMENT_CLIENT_SECRET").map_err(|error| {
      log::warn!("unable to find auth 0 management client secret - {}", error);
      Error::new(ErrorKind::Other, "missing-management-client-secret")
    })?;
    let domain = std::env::var("AUTH_0_DOMAIN").map_err(|error| {
      log::warn!("unable to find auth 0 domain - {}", error);
      Error::new(ErrorKind::Other, "missing-domain")
    })?;
    let redirect_uri = std::env::var("AUTH_0_REDIRECT_URI").map_err(|error| {
      log::warn!("unable to find auth 0 redirect uri - {}", error);
      Error::new(ErrorKind::Other, "missing-redirect-uri")
    })?;

    Ok(Self {
      domain,
      auth_client_id,
      auth_client_secret,
      management_client_id,
      management_client_secret,
      redirect_uri,
    })
  }
}
