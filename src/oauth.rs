use std::io::{Error, ErrorKind, Result};

use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct UserRole {
  id: String,
  name: String,
}

impl UserRole {
  pub fn is_admin(&self) -> bool {
    self.name.split(":").any(|part| part.starts_with("admin"))
  }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct UserInfo {
  pub sub: String,
  pub nickname: String,
  pub email: String,
  pub picture: String,
  pub email_verified: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct AuthCodeResponse {
  access_token: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ManagementTokenResponse {
  access_token: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ManagementUserInfoResponse {
  name: Option<String>,
  user_id: String,
  picture: Option<String>,
  email: Option<String>,
  nickname: Option<String>,
  email_verified: bool,
}

#[derive(Debug, Serialize)]
pub struct AuthCodeRequest {
  grant_type: String,
  client_id: String,
  client_secret: String,
  redirect_uri: Option<String>,
  code: Option<String>,
  audience: Option<String>,
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
  #[inline]
  pub fn token_uri(&self) -> Result<String> {
    let base = format!("{}/oauth/token", self.domain);
    Ok(base)
  }

  pub async fn fetch_initial_user_info<T>(&self, code: T) -> Result<UserInfo>
  where
    T: std::fmt::Display,
  {
    let mut response = surf::post(&self.token_uri()?)
      .body_json(&self.auth_token_payload(code)?)
      .map_err(|error| {
        log::warn!("unable to serialize auth token payload - {}", error);
        Error::new(ErrorKind::Other, "bad-token-serialize")
      })?
      .await
      .map_err(|error| {
        log::warn!("unable to request token for code - {}", error);
        Error::new(ErrorKind::Other, "bad-code-exchange-request")
      })?;

    let tok = response
      .body_json::<AuthCodeResponse>()
      .await
      .map_err(|error| {
        log::warn!("unable to parse token exchange response - {}", error);
        Error::new(ErrorKind::Other, "bad-code-exchange-request")
      })
      .map(|body| body.access_token)?;

    let mut res = surf::get(format!("{}/userinfo", self.domain))
      .header("Authorization", format!("Bearer {}", tok))
      .await
      .map_err(|error| {
        log::warn!("unable to parse token exchange response - {}", error);
        Error::new(ErrorKind::Other, "bad-code-exchange-request")
      })?;

    res.body_json::<UserInfo>().await.map_err(|error| {
      log::warn!("unable to parse token exchange response - {}", error);
      Error::new(ErrorKind::Other, "bad-code-exchange-request")
    })
  }

  pub async fn fetch_detailed_user_info<T>(&self, id: T) -> Result<ManagementUserInfoResponse>
  where
    T: std::fmt::Display,
  {
    let token = self.get_new_management_token().await?;
    let mut response = surf::get(format!("{}/api/v2/users/{}", self.domain, id))
      .header("Authorization", format!("Bearer {}", token))
      .await
      .map_err(|error| {
        log::warn!("unable to parse user info response - {}", error);
        Error::new(ErrorKind::Other, format!("{}", error))
      })?;

    if response.status() != surf::StatusCode::Ok {
      return Err(Error::new(ErrorKind::Other, "not-ok-response"));
    }

    response
      .body_json::<ManagementUserInfoResponse>()
      .await
      .map_err(|error| {
        log::warn!("unable to parse response - {}", error);
        Error::new(ErrorKind::Other, format!("{}", error))
      })
  }

  pub async fn fetch_user_roles<T>(&self, id: T) -> Result<Vec<UserRole>>
  where
    T: std::fmt::Display,
  {
    let token = self.get_new_management_token().await?;
    let mut response = surf::get(format!("{}/api/v2/users/{}/roles", self.domain, id))
      .header("Authorization", format!("Bearer {}", token))
      .await
      .map_err(|error| {
        log::warn!("unable to parse user info response - {}", error);
        Error::new(ErrorKind::Other, format!("{}", error))
      })?;

    log::debug!("request for roles completed - {}", response.status());

    response.body_json::<Vec<UserRole>>().await.map_err(|error| {
      log::warn!("unable to parse user role response - {}", error);
      Error::new(ErrorKind::Other, format!("{}", error))
    })
  }

  async fn get_new_management_token(&self) -> Result<String> {
    let mut response = surf::post(&self.token_uri()?)
      .body_json(&self.manage_token_payload()?)
      .map_err(|error| {
        log::warn!("failed serializing management token payload - {}", error);
        Error::new(ErrorKind::Other, "bad-management-payload")
      })?
      .await
      .map_err(|error| {
        log::warn!("failed management token response - {}", error);
        Error::new(ErrorKind::Other, "bad-management-response")
      })?;

    if response.status() != surf::StatusCode::Ok {
      return Err(Error::new(ErrorKind::Other, "not-ok-response"));
    }

    response
      .body_json::<ManagementTokenResponse>()
      .await
      .map_err(|error| {
        log::warn!("unable to parse response - {}", error);
        Error::new(ErrorKind::Other, format!("{}", error))
      })
      .map(|b| b.access_token)
  }

  fn manage_token_payload(&self) -> Result<AuthCodeRequest> {
    Ok(AuthCodeRequest {
      client_id: self.management_client_id.clone(),
      client_secret: self.management_client_secret.clone(),
      code: None,
      redirect_uri: None,
      grant_type: "client_credentials".into(),
      audience: Some(format!("{}/api/v2/", self.domain)),
    })
  }

  fn auth_token_payload<T>(&self, code: T) -> Result<AuthCodeRequest>
  where
    T: std::fmt::Display,
  {
    Ok(AuthCodeRequest {
      client_id: self.auth_client_id.clone(),
      client_secret: self.auth_client_secret.clone(),
      code: Some(format!("{}", code)),
      audience: None,
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
