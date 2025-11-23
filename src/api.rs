use core::fmt;
use std::env;

use crate::VERSION;

pub struct LighthouseAPI {
    pub base_url: String,
    pub api_version: String,
    env: Env,
}

impl LighthouseAPI {
    fn new(base_url: LighthouseAPIBaseURL, api_version: &str, env: Env) -> LighthouseAPI {
        LighthouseAPI {
            base_url: base_url.0,
            api_version: api_version.to_string(),
            env,
        }
    }
}

#[derive(Clone, Copy)]
pub enum Env {
    DEV,
    RELEASE,
}

impl fmt::Display for Env {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Env::DEV => write!(f, "dev"),
            Env::RELEASE => write!(f, "release"),
        }
    }
}

struct LighthouseAPIBaseURL(String);

impl LighthouseAPIBaseURL {
    pub fn from(base_url: &str, environment: Env) -> Result<Self, String> {
        let pattern = match environment {
            // DEV: allow localhost (http or https, any port)
            Env::DEV => r"^https?://localhost(:\d+)?(/.*)?$",
            // RELEASE: only allow https://*projectlighthouse.io
            Env::RELEASE => r"^https://([a-zA-Z0-9-]+\.)*projectlighthouse\.io(/.*)?$",
        };

        let re = regex::Regex::new(pattern)
            .map_err(|e| format!("invalid regex pattern: {}", e))?;

        if re.is_match(base_url) {
            Ok(LighthouseAPIBaseURL(base_url.to_string()))
        } else {
            let err_msg = match environment {
                Env::DEV => "invalid URL: must be localhost in DEV environment",
                Env::RELEASE => "invalid URL: must be https://*.projectlighthouse.io in RELEASE environment",
            };
            Err(err_msg.to_string())
        }
    }

    pub fn default_for_env(environment: Env) -> Self {
        let url = match environment {
            Env::DEV => "http://localhost:8000",
            Env::RELEASE => "https://api.projectlighthouse.io",
        };
        LighthouseAPIBaseURL(url.to_string())
    }
}


impl Default for LighthouseAPI {
    fn default() -> Self {
        // 1. get the env first from LUX_ENV, it should map to the enum Env::DEV or Env::RELEASE
        // 2. default to Env::DEV in case of error or if not set
        let lux_env = match env::var("LUX_ENV") {
            Ok(val) => match val.to_uppercase().as_str() {
                "RELEASE" => Env::RELEASE,
                _ => Env::DEV,
            },
            Err(_) => Env::DEV,
        };

        // 3. get base_url from env var or use default for the environment
        let base_url = match env::var("LUX_API_BASE_URL") {
            Ok(val) => {
                // Validate the URL if provided
                match LighthouseAPIBaseURL::from(&val, lux_env) {
                    Ok(url) => url,
                    Err(e) => {
                        log::warn!("invalid LUX_API_BASE_URL: {}. using default.", e);
                        LighthouseAPIBaseURL::default_for_env(Env::DEV)
                    }
                }
            }
            Err(_) => LighthouseAPIBaseURL::default_for_env(lux_env),
        };

        log::info!("initiating lighthouse api with {}", base_url.0);

        LighthouseAPI::new(base_url, "v1", lux_env)
    }
}


impl fmt::Display for LighthouseAPI {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f,"cli_version: {} base_url: {} api_version: {} env: {}",VERSION, self.base_url, self.api_version, self.env)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lighthouse_api_base_url_dev_env() {
        // Valid localhost URLs in DEV
        assert!(LighthouseAPIBaseURL::from("http://localhost", Env::DEV).is_ok());
        assert!(LighthouseAPIBaseURL::from("https://localhost", Env::DEV).is_ok());
        assert!(LighthouseAPIBaseURL::from("http://localhost:8080", Env::DEV).is_ok());
        assert!(LighthouseAPIBaseURL::from("https://localhost:3000/api", Env::DEV).is_ok());

        // projectlighthouse.io NOT allowed in DEV
        assert!(LighthouseAPIBaseURL::from("https://projectlighthouse.io", Env::DEV).is_err());
        assert!(LighthouseAPIBaseURL::from("https://api.projectlighthouse.io", Env::DEV).is_err());

        // Invalid URLs in DEV
        assert!(LighthouseAPIBaseURL::from("ftp://localhost", Env::DEV).is_err()); // wrong scheme
        assert!(LighthouseAPIBaseURL::from("https://example.com", Env::DEV).is_err()); // wrong domain
    }

    #[test]
    fn test_lighthouse_api_base_url_release_env() {
        // Valid projectlighthouse.io URLs in RELEASE (https only)
        assert!(LighthouseAPIBaseURL::from("https://projectlighthouse.io", Env::RELEASE).is_ok());
        assert!(LighthouseAPIBaseURL::from("https://projectlighthouse.io/api", Env::RELEASE).is_ok());
        assert!(LighthouseAPIBaseURL::from("https://api.projectlighthouse.io", Env::RELEASE).is_ok());
        assert!(LighthouseAPIBaseURL::from("https://api.projectlighthouse.io/v1", Env::RELEASE).is_ok());

        // localhost NOT allowed in RELEASE
        assert!(LighthouseAPIBaseURL::from("http://localhost", Env::RELEASE).is_err());
        assert!(LighthouseAPIBaseURL::from("https://localhost:8080", Env::RELEASE).is_err());

        // Invalid URLs in RELEASE
        assert!(LighthouseAPIBaseURL::from("http://projectlighthouse.io", Env::RELEASE).is_err()); // http not allowed
        assert!(LighthouseAPIBaseURL::from("https://example.com", Env::RELEASE).is_err()); // wrong domain
    }
}

