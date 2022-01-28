use serde::{Deserialize, Serialize};
use tide::{http::Url, Body, Error, Redirect, Request, Response, Result};

use super::{cookie, sec::Claims, State};

#[cfg(debug_assertions)]
const COOKIE_SET_FLAGS: &'static str = "Max-Age=600; Path=/; SameSite=Strict; HttpOnly";
#[cfg(not(debug_assertions))]
const COOKIE_SET_FLAGS: &'static str = "Max-Age=600; Path=/; SameSite=Strict; HttpOnly; Secure";

#[derive(Debug, Serialize)]
struct AuthIdentifyResponse {
  ok: bool,
  timestamp: chrono::DateTime<chrono::Utc>,
  user: Option<UserInfo>,
}

impl Default for AuthIdentifyResponse {
  fn default() -> Self {
    Self {
      ok: false,
      timestamp: chrono::Utc::now(),
      user: None,
    }
  }
}

#[derive(Debug, Deserialize)]
struct AuthCodeResponse {
  access_token: String,
}

#[derive(Debug, Serialize)]
struct AuthCodeRequest {
  grant_type: String,
  client_id: String,
  client_secret: String,
  redirect_uri: String,
  code: String,
}

impl Default for AuthCodeRequest {
  fn default() -> Self {
    let client_id = std::env::var("AUTH_0_CLIENT_ID").ok().unwrap_or_else(|| {
      log::warn!("missing auth 0 client id");
      String::default()
    });
    let redirect_uri = std::env::var("AUTH_0_REDIRECT_URI").ok().unwrap_or_else(|| {
      log::warn!("missing auth 0 redirect uri");
      String::default()
    });
    let client_secret = std::env::var("AUTH_0_CLIENT_SECRET").ok().unwrap_or_else(|| {
      log::warn!("missing auth 0 client secret");
      String::default()
    });

    AuthCodeRequest {
      client_id,
      client_secret,
      redirect_uri,
      code: "".into(),
      grant_type: "authorization_code".into(),
    }
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

pub async fn fetch_user<T>(token: T) -> Option<UserInfo>
where
  T: std::fmt::Display,
{
  let uri = std::env::var("AUTH_0_USERINFO_URI").unwrap_or_else(|error| {
    log::warn!("missing auth0 userinfo uri in environment - {}", error);
    "".into()
  });

  let mut res = surf::get(&uri)
    .header("Authorization", format!("Bearer {}", token))
    .await
    .ok()?;

  if res.status() != surf::StatusCode::Ok {
    log::warn!("bad response status - '{:?}'", res.status());
    return None;
  }

  log::debug!("loaded info with status '{}', attempting to parse", res.status());
  res.body_json::<UserInfo>().await.ok()
}

async fn token_from_response(response: &mut surf::Response) -> Option<String> {
  let status = response.status();

  match status {
    surf::StatusCode::Ok => log::debug!("good response from auth provider token api"),
    other => {
      log::warn!("bad status code from token response - '{:?}'", other);
      return None;
    }
  };

  response
    .body_json::<AuthCodeResponse>()
    .await
    .ok()
    .map(|body| body.access_token)
}

pub async fn identify(request: Request<State>) -> Result {
  let claims = cookie(&request).and_then(|cook| {
    log::info!("found cookie - {:?}", cook.value());
    Claims::decode(&cook.value()).ok()
  });

  log::debug!("attempting to identify user from claims - {:?}", claims);
  let mut res = AuthIdentifyResponse::default();

  if let Some(claims) = claims {
    let user = fetch_user("hello").await;
    res.ok = user.is_some();
    res.user = user;
  }

  Body::from_json(&res).map(|bod| Response::builder(200).body(bod).build())
}

pub async fn complete(request: Request<State>) -> Result {
  let code = request
    .url()
    .query_pairs()
    .find_map(|(k, v)| if k == "code" { Some(v) } else { None })
    .ok_or(Error::from_str(404, "no-code"))?;

  let oauth = request.state().oauth();

  // Attempt top exchange our code with the oAuth provider for a token.
  let payload = oauth.auth_token_payload(&String::from(code))?;
  log::info!("requesting token - {:?}", payload);
  let destination = oauth.token_uri().map_err(|error| {
    log::warn!("missing auth 0 token url environment - {}", error);
    Error::from_str(500, "bad-oauth")
  })?;

  let mut response = surf::post(&destination).body_json(&payload)?.await?;
  let token = token_from_response(&mut response).await.ok_or_else(|| {
    log::warn!("unable to parse token from response");
    Error::from_str(404, "token-exchange")
  })?;

  let user = fetch_user(&token).await.ok_or(Error::from_str(404, "user-not-found"))?;

  if user.email_verified.unwrap_or(false) != true {
    log::warn!("user email not verified for sub '{}'", user.sub);
    return Err(Error::from_str(404, "user-not-found"));
  }

  let jwt = Claims::for_sub(&user.sub).encode()?;

  let cookie = format!("{}={}; {}", "_obs", jwt, COOKIE_SET_FLAGS);

  let destination = std::env::var("CLIENT_AUTH_COMPLETE_URI").ok().unwrap_or_else(|| {
    log::warn!("missing client auth completion uri");
    "/auth/identify".into()
  });

  // TODO - determine where to send the user. Once the web UI is created, we will send the user to some login page
  // where an attempt will be made to fetch identity information using the newly-set cookie.
  let response = Response::builder(302)
    .header("Set-Cookie", cookie)
    .header("Location", destination.as_str())
    .build();

  Ok(response)
}

pub async fn start(request: Request<State>) -> Result {
  log::info!("initializing oauth redirect");
  let destination = request.state().oauth().redirect_uri().map_err(|error| {
    log::warn!("{}", error);
    Error::from_str(500, "bad-oauth")
  })?;
  Ok(Redirect::temporary(destination).into())
}
