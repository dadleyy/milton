use std::io::{Error, ErrorKind, Result};

#[derive(Debug, Clone)]
pub struct RedisConfig {
  host: String,
  port: String,
  password: String,
}

impl RedisConfig {
  pub async fn send<T, V>(&self, cmd: &kramer::Command<T, V>) -> Result<kramer::Response>
  where
    T: std::fmt::Display,
    V: std::fmt::Display,
  {
    let url = format!("{}:{}", self.host, self.port);
    let mut stream = async_std::net::TcpStream::connect(url).await?;
    kramer::execute(&mut stream, &cmd).await
  }

  pub async fn connect(&self) -> Result<()> {
    let cmd = kramer::Command::Auth::<&String, &String>(kramer::AuthCredentials::Password(&self.password));
    self.send(&cmd).await.map(|result| {
      log::info!("connected to redis - {:?}", result);
      ()
    })
  }
}

#[inline]
pub fn from_env() -> Result<RedisConfig> {
  let host = std::env::var("REDIS_HOSTNAME").map_err(|error| {
    log::warn!("missing 'REDIS_HOSTNAME' env - {}", error);
    Error::new(ErrorKind::Other, "missing-redis-host")
  })?;
  let port = std::env::var("REDIS_PORT").map_err(|error| {
    log::warn!("missing 'REDIS_PORT' env - {}", error);
    Error::new(ErrorKind::Other, "missing-redis-host")
  })?;
  let password = std::env::var("REDIS_AUTH_KEY").map_err(|error| {
    log::warn!("missing 'REDIS_AUTH_KEY' env - {}", error);
    Error::new(ErrorKind::Other, "missing-redis-auth")
  })?;
  Ok(RedisConfig { host, port, password })
}
