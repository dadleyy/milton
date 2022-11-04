use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
  pub exp: usize,
  pub iat: usize,
  pub oid: String,
}

impl Claims {
  pub fn for_sub<T>(oid: T) -> Self
  where
    T: std::fmt::Display,
  {
    let day = chrono::Utc::now()
      .checked_add_signed(chrono::Duration::minutes(60))
      .unwrap_or(chrono::Utc::now());

    let exp = day.timestamp() as usize;
    let iat = chrono::Utc::now().timestamp() as usize;
    log::debug!("encoding new jwt, expires {}", exp);

    Self {
      exp,
      iat,
      oid: format!("{}", oid),
    }
  }

  pub fn decode<T, S>(target: &T, secret: &S) -> std::io::Result<Self>
  where
    T: std::fmt::Display,
    S: std::convert::AsRef<str>,
  {
    let token = format!("{}", target);
    let key = jsonwebtoken::DecodingKey::from_secret(secret.as_ref().as_bytes());
    let validation = jsonwebtoken::Validation::new(jsonwebtoken::Algorithm::HS256);
    jsonwebtoken::decode::<Self>(token.as_str(), &key, &validation)
      .map_err(|error| {
        log::warn!("unable to decode token - {}", error);
        std::io::Error::new(std::io::ErrorKind::Other, "bad-jwt")
      })
      .map(|data| data.claims)
  }

  pub fn encode<S>(&self, secret: S) -> std::io::Result<String>
  where
    S: std::convert::AsRef<str>,
  {
    let header = &jsonwebtoken::Header::default();
    let secret = jsonwebtoken::EncodingKey::from_secret(secret.as_ref().as_bytes());

    jsonwebtoken::encode(&header, &self, &secret).map_err(|error| {
      log::warn!("unable to encode token - {}", error);
      std::io::Error::new(std::io::ErrorKind::Other, "bad-jwt")
    })
  }
}
