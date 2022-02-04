use serde::Serialize;
use tide::{Body, Error, Redirect, Request, Response, Result};

use super::{cookie, sec::Claims, State};

#[cfg(debug_assertions)]
const COOKIE_SET_FLAGS: &'static str = "Max-Age=3600; Path=/; SameSite=Strict; HttpOnly";
#[cfg(not(debug_assertions))]
const COOKIE_SET_FLAGS: &'static str = "Max-Age=3600; Path=/; SameSite=Strict; HttpOnly; Secure";

#[derive(Debug, Serialize)]
struct AuthIdentifyResponseUserInfo {
  user: crate::auth_zero::ManagementUserInfoResponse,
  roles: Vec<crate::auth_zero::UserRole>,
}

#[derive(Debug, Serialize)]
struct AuthIdentifyResponse {
  ok: bool,
  timestamp: chrono::DateTime<chrono::Utc>,
  session: Option<AuthIdentifyResponseUserInfo>,
}

impl Default for AuthIdentifyResponse {
  fn default() -> Self {
    Self {
      ok: false,
      timestamp: chrono::Utc::now(),
      session: None,
    }
  }
}

// ROUTE: attempts to fetch user information from cookie.
pub async fn identify(request: Request<State>) -> Result {
  let claims = cookie(&request).and_then(|cook| {
    log::info!("found cookie - {:?}", cook.value());
    Claims::decode(&cook.value()).ok()
  });

  log::info!("attempting to identify user from claims - {:?}", claims);
  let mut res = AuthIdentifyResponse::default();
  let oauth = request.state().oauth();

  if let Some(claims) = claims {
    let user = oauth.fetch_detailed_user_info(&claims.oid).await.ok();
    let roles = oauth.fetch_user_roles(&claims.oid).await.ok().unwrap_or_default();

    if roles.iter().any(|role| role.is_admin()) {
      res.ok = user.is_some();
      res.session = user.map(|user| AuthIdentifyResponseUserInfo { user, roles });
    }
  }

  Body::from_json(&res).map(|bod| Response::builder(200).body(bod).build())
}

// ROUTE: callback for oauth, completes cookie storage.
pub async fn complete(request: Request<State>) -> Result {
  let code = request
    .url()
    .query_pairs()
    .find_map(|(k, v)| if k == "code" { Some(v) } else { None })
    .ok_or(Error::from_str(404, "no-code"))?;

  let oauth = request.state().oauth();
  let user = oauth.fetch_initial_user_info(&code).await.map_err(|error| {
    log::warn!("unable to fetch initial user info - {}", error);
    Error::from_str(500, "bad-oauth")
  })?;

  if user.email_verified.unwrap_or(false) != true {
    log::warn!("user email not verified for sub '{}'", user.sub);
    return Err(Error::from_str(404, "user-not-found"));
  }

  let roles = oauth.fetch_user_roles(&user.sub).await.map_err(|error| {
    log::warn!("unable to fetch user roles - {}", error);
    Error::from_str(500, "bad-roles-listing")
  })?;

  // TODO: should non-admins be allowed to see info?
  if roles.iter().any(|role| role.is_admin()) != true {
    log::warn!("user not admin, skippping cookie setting (roles {:?})", roles);
    return Err(Error::from_str(404, "user-not-found"));
  }

  log::info!("found user roles - {:?}", roles);

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

// ROUTE: simple redirect, starts oauth flow.
pub async fn start(request: Request<State>) -> Result {
  log::info!("initializing oauth redirect");
  let destination = request.state().oauth().redirect_uri().map_err(|error| {
    log::warn!("{}", error);
    Error::from_str(500, "bad-oauth")
  })?;
  Ok(Redirect::temporary(destination).into())
}
