use serde::{Deserialize, Serialize};
use tide::{Body, Error, Redirect, Request, Response, Result};

use super::{cookie, sec::Claims, State};

/// The name of our session cookie used within our `Set-Cookie` headers.
pub(crate) const COOKIE_NAME: &str = "_milton_session";

/// When setting the cookie, these flags are used alongside the actual value.
#[cfg(debug_assertions)]
const COOKIE_SET_FLAGS: &str = "Max-Age=3600; Path=/; SameSite=Strict; HttpOnly";
#[cfg(not(debug_assertions))]
const COOKIE_SET_FLAGS: &str = "Max-Age=3600; Path=/; SameSite=Strict; HttpOnly; Secure";

/// When clearing a cookie, these flags are sent.
#[cfg(debug_assertions)]
const COOKIE_CLEAR_FLAGS: &str = "Max-Age=0; Expires=Wed, 21 Oct 2015 07:28:00 GMT; Path=/; SameSite=Strict; HttpOnly";
#[cfg(not(debug_assertions))]
const COOKIE_CLEAR_FLAGS: &str =
  "Max-Age=0; Expires=Wed, 21 Oct 2015 07:28:00 GMT; Path=/; SameSite=Strict; HttpOnly; Secure";

/// The inner type sent in our identify endpoint when a user is available.
#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct AuthIdentifyResponseUserInfo {
  /// Contains information provided by our oauth provider.
  user: crate::oauth::ManagementUserInfoResponse,

  /// The list of Auth0 roles our current user is assigned to.
  pub(crate) roles: Vec<crate::oauth::UserRole>,
}

/// The json-serializable response structure for our identify endpoint.
#[derive(Debug, Serialize)]
struct AuthIdentifyResponse {
  /// This field is true when were are able to verify an authenticated user from the cookie data.
  ok: bool,

  /// Include the version in our auth payload.
  version: String,

  /// The current time.
  timestamp: chrono::DateTime<chrono::Utc>,

  /// Optionally-included information about the user if we found one.
  session: Option<AuthIdentifyResponseUserInfo>,
}

impl Default for AuthIdentifyResponse {
  fn default() -> Self {
    Self {
      ok: false,
      timestamp: chrono::Utc::now(),
      session: None,
      version: "unknown".to_string(),
    }
  }
}

/// ROUTE: clears the cookie
pub async fn end(request: Request<State>) -> Result {
  let claims = cookie(&request).and_then(|cook| {
    log::info!("found cookie - {:?}", cook.value());
    Claims::decode(&cook.value(), &request.state().config.jwt_secret).ok()
  });

  if let Some(inner) = claims {
    log::debug!("attempting to delete session for '{}'", inner.oid);

    if let Err(error) = request
      .state()
      .command(kramer::Command::Del::<&str, &str>(kramer::Arity::One(&inner.oid)))
      .await
    {
      log::error!("unable to dleete session data - '{error}'");
    }
  }

  let clear_cookie = format!(
    "{}=''; {}; Domain={}",
    COOKIE_NAME,
    COOKIE_CLEAR_FLAGS,
    request.state().config.domain
  );

  let response = Response::builder(302)
    .header("Set-Cookie", &clear_cookie)
    .header("Location", &request.state().config.auth_complete_uri)
    .build();

  log::debug!("clearing session cookie via {clear_cookie}");

  Ok(response)
}

/// ROUTE: attempts to fetch user information from cookie.
pub async fn identify(request: Request<State>) -> Result {
  let claims = cookie(&request).and_then(|cook| {
    log::info!("found cookie - {:?}", cook.value());
    Claims::decode(&cook.value(), &request.state().config.jwt_secret).ok()
  });

  log::info!("attempting to identify user from claims - {:?}", claims);
  let mut res = AuthIdentifyResponse::default();
  res.version = request.state().version.clone();

  if let Some(claims) = claims {
    let session_data = request.state().user_from_session(&claims.oid).await.ok_or_else(|| {
      log::warn!("unable to load session data for claims {}", claims.oid);
      Error::from_str(404, "no-session")
    })?;

    if session_data.roles.iter().any(|role| role.is_admin()) {
      res.ok = true;
      res.session = Some(session_data);
    }
  }

  Body::from_json(&res).map(|bod| Response::builder(200).body(bod).build())
}

/// ROUTE: callback for oauth, completes cookie storage.
pub async fn complete(request: Request<State>) -> Result {
  log::debug!("completing oauth exchange");

  let code = request
    .url()
    .query_pairs()
    .find_map(|(k, v)| if k == "code" { Some(v) } else { None })
    .ok_or_else(|| Error::from_str(404, "no-code"))?;

  log::debug!("attempting to exchange code '{code}'");

  let oauth = &request.state().oauth;
  let user = oauth.fetch_initial_user_info(&code).await.map_err(|error| {
    log::warn!("unable to fetch initial user info - {}", error);
    Error::from_str(500, "bad-oauth")
  })?;

  if user.email_verified.is_none() {
    log::warn!("user email not verified for sub '{}'", user.sub);
    return Err(Error::from_str(404, "user-not-found"));
  }

  let roles = oauth.fetch_user_roles(&user.sub).await.map_err(|error| {
    log::warn!("unable to fetch user roles - {}", error);
    Error::from_str(500, "bad-roles-listing")
  })?;

  // TODO: should non-admins be allowed to see info?
  if !roles.iter().any(|role| role.is_admin()) {
    log::warn!("user not admin, skippping cookie setting (roles {:?})", roles);
    return Err(Error::from_str(404, "user-not-found"));
  }

  log::info!(
    "found user roles - {:?} (sending to {})",
    roles,
    request.state().config.auth_complete_uri
  );

  // Fetch the complete user information available to us from the auth0 api.
  let user = oauth.fetch_detailed_user_info(&user.sub).await.map_err(|error| {
    log::warn!("unable to load complete user information from auth0 - {error}");
    error
  })?;

  // Create a serializable representation of our user information
  let session_data = AuthIdentifyResponseUserInfo { user, roles };
  let session_id = uuid::Uuid::new_v4().to_string();
  let serialized_session = serde_json::to_string(&session_data).map_err(|error| {
    log::warn!("unable to serialize session data - {error}");
    error
  })?;

  // Perist that user information into our redis storage.
  let command = kramer::Command::Strings(kramer::StringCommand::Set(
    kramer::Arity::One((&session_id, &serialized_session)),
    None,
    kramer::Insertion::Always,
  ));
  request.state().command(command).await.map_err(|error| {
    log::warn!("unable to persist session information - {error}");
    error
  })?;

  // Create our json web token, including the unique identifier we generated for this session.
  let jwt = Claims::for_sub(&session_id).encode(&request.state().config.jwt_secret)?;
  let cookie = format!(
    "{}={}; {}; Domain={}",
    COOKIE_NAME,
    jwt,
    COOKIE_SET_FLAGS,
    &request.state().config.domain
  );

  // TODO - determine where to send the user. Once the web UI is created, we will send the user to some login page
  // where an attempt will be made to fetch identity information using the newly-set cookie.
  let response = Response::builder(302)
    .header("Set-Cookie", cookie)
    .header("Location", request.state().config.auth_complete_uri.as_str())
    .build();

  Ok(response)
}

/// ROUTE: simple redirect, starts oauth flow.
pub async fn start(request: Request<State>) -> Result {
  log::info!("initializing oauth redirect");
  let destination = request.state().oauth.redirect_uri().map_err(|error| {
    log::warn!("{}", error);
    Error::from_str(500, "bad-oauth")
  })?;
  Ok(Redirect::temporary(destination).into())
}
