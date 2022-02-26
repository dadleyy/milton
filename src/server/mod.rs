use std::io::Result;

mod authority;
mod cookie;
mod routes;
mod sec;
mod state;

pub(crate) use cookie::cookie;

pub use state::{State, StateBuilder};

pub async fn run<S>(state: State, addr: S) -> Result<()>
where
  S: AsRef<str>,
{
  let mut app = tide::with_state(state);

  app.at("/incoming-webhook").post(routes::webhook::hook);

  app.at("/control").post(routes::control::command);
  app.at("/control").get(routes::control::query);
  app.at("/control/snapshot").get(routes::control::snapshot);
  app.at("/control/pattern").post(routes::control::write_pattern);

  app.at("/auth/start").get(routes::auth::start);
  app.at("/auth/complete").get(routes::auth::complete);
  app.at("/auth/identify").get(routes::auth::identify);

  app.at("/*").all(routes::missing);
  app.listen(addr.as_ref()).await
}
