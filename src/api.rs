use std::env;

pub struct LighthouseAPI {
    pub base_url: String,
    pub api_version: String,
}

impl LighthouseAPI {
    pub fn new(base_url: &str, api_version: &str) -> LighthouseAPI {
        LighthouseAPI {
            base_url: base_url.to_string(),
            api_version: api_version.to_string(),
        }
    }
}

#[derive(Clone, Copy)]
enum Env {
    DEV,
    RELEASE,
}

struct LighthouseAPIBaseURL(String);

impl LighthouseAPIBaseURL {
    pub fn from(base_url: &str, env: Env) -> Result<Self, String> {
        // Regex pattern:
        // 1. localhost (http or https, any port)
        // 2. OR *.projectlighthouse.io with https only
        let pattern = r"^(https?://localhost(:\d+)?(/.*)?|https://([a-zA-Z0-9-]+\.)*projectlighthouse\.io(/.*)?)\s*$";

        let re = regex::Regex::new(pattern)
            .map_err(|e| format!("invalid regex pattern: {}", e))?;

        if re.is_match(base_url) {
            Ok(LighthouseAPIBaseURL(base_url.to_string()))
        } else {
            Err("invalid URL: must be localhost or https://*.projectlighthouse.io".to_string())
        }
    }

    pub fn default_for_env(env: Env) -> Self {
        let url = match env {
            Env::DEV => "http://localhost:8080",
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
                "RELEASE" | "PROD" | "PRODUCTION" => Env::RELEASE,
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

        LighthouseAPI::new(&base_url.0, "v1")
    }
}

impl LighthouseAPI {
    pub fn ping(&self) {
        log::info!("pong")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lighthouse_api_base_url_validation() {
        // Valid localhost URLs
        assert!(LighthouseAPIBaseURL::from("http://localhost", Env::DEV).is_ok());
        assert!(LighthouseAPIBaseURL::from("https://localhost", Env::DEV).is_ok());
        assert!(LighthouseAPIBaseURL::from("http://localhost:8080", Env::DEV).is_ok());
        assert!(LighthouseAPIBaseURL::from("https://localhost:3000/api", Env::DEV).is_ok());

        // Valid projectlighthouse.io URLs (https only)
        assert!(LighthouseAPIBaseURL::from("https://projectlighthouse.io", Env::RELEASE).is_ok());
        assert!(LighthouseAPIBaseURL::from("https://api.projectlighthouse.io", Env::RELEASE).is_ok());
        assert!(LighthouseAPIBaseURL::from("https://api.projectlighthouse.io/v1", Env::RELEASE).is_ok());

        // Invalid URLs
        assert!(LighthouseAPIBaseURL::from("http://projectlighthouse.io", Env::RELEASE).is_err()); // http not allowed
        assert!(LighthouseAPIBaseURL::from("https://example.com", Env::RELEASE).is_err()); // wrong domain
        assert!(LighthouseAPIBaseURL::from("ftp://localhost", Env::DEV).is_err()); // wrong scheme
    }
}

