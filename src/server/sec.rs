use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
  pub exp: usize,
  pub oid: String,
  pub token: String,
}

impl Claims {
  pub fn for_sub<T>(oid: T, tok: T) -> Self
  where
    T: std::fmt::Display,
  {
    let day = chrono::Utc::now()
      .checked_add_signed(chrono::Duration::minutes(60))
      .unwrap_or(chrono::Utc::now());

    let exp = day.timestamp() as usize;
    log::debug!("encoding new jwt, expires {}", exp);

    Self {
      exp,
      oid: format!("{}", oid),
      token: format!("{}", tok),
    }
  }

  pub fn decode<T>(target: &T) -> std::io::Result<Self>
  where
    T: std::fmt::Display,
  {
    let token = format!("{}", target);
    let secret = std::env::var("SESSION_JWT_SECRET").unwrap_or_else(|error| {
      log::warn!("no session jwt found in environment - {}", error);
      String::default()
    });
    let key = jsonwebtoken::DecodingKey::from_secret(secret.as_bytes());
    let validation = jsonwebtoken::Validation::new(jsonwebtoken::Algorithm::HS256);
    jsonwebtoken::decode::<Self>(token.as_str(), &key, &validation)
      .map_err(|error| {
        log::warn!("unable to decode token - {}", error);
        std::io::Error::new(std::io::ErrorKind::Other, "bad-jwt")
      })
      .map(|data| data.claims)
  }

  pub fn encode(&self) -> std::io::Result<String> {
    let header = &jsonwebtoken::Header::default();
    let secret = std::env::var("SESSION_JWT_SECRET").unwrap_or_else(|error| {
      log::warn!("NO JWT SECRET DEFINED - {}", error);
      "blinkrs".into()
    });
    let secret = jsonwebtoken::EncodingKey::from_secret(secret.as_bytes());

    jsonwebtoken::encode(&header, &self, &secret).map_err(|error| {
      log::warn!("unable to encode token - {}", error);
      std::io::Error::new(std::io::ErrorKind::Other, "bad-jwt")
    })
  }
}
