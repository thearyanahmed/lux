use color_eyre::eyre::{eyre, Result};
use core::fmt;
use reqwest::{header::HeaderMap, Client};
use secrecy::{ExposeSecret, SecretString};
use serde::de::DeserializeOwned;
use std::{collections::HashMap, env};

use crate::{config::Config, VERSION};

use super::types::ApiUser;

pub struct LighthouseAPIClient {
    pub base_url: String,
    pub api_version: String,
    env: Env,
    client: Client,
    token: Option<SecretString>,
}

impl LighthouseAPIClient {
    fn new(
        base_url: LighthouseAPIClientBaseURL,
        api_version: &str,
        env: Env,
        token: Option<SecretString>,
    ) -> LighthouseAPIClient {
        LighthouseAPIClient {
            base_url: base_url.0,
            api_version: api_version.to_string(),
            env,
            client: Client::new(),
            token,
        }
    }

    pub fn from_config(config: &Config) -> LighthouseAPIClient {
        let mut client = LighthouseAPIClient::default();
        client.token = Some(SecretString::from(config.expose_token().to_string()));
        client
    }

    fn auth_headers(&self) -> Result<HeaderMap> {
        let token = self
            .token
            .as_ref()
            .ok_or_else(|| eyre!("no auth token configured"))?;

        let mut headers = HeaderMap::new();
        headers.insert(
            "Authorization",
            format!("Bearer {}", token.expose_secret()).parse()?,
        );
        headers.insert("Accept", "application/json".parse()?);
        Ok(headers)
    }

    // when we deserialize JSON, we're creating owned data. But
    // there are two Deserialize traits:
    //
    // 1. Deserialize<'de> - Can borrow data from the input (has a lifetime)
    // 2. DeserializeOwned - Must own all its data (no lifetimes)
    //
    // when we write a generic function that returns T, you need T to be able
    // to own its data.
    //
    // #[derive(Deserialize)]  // This implements BOTH Deserialize<'de> AND
    // DeserializeOwned
    // struct User {
    //     username: String,  // String is owned data
    //     email: String,
    // }
    //
    // let user: User = response.json().await?;
    // The User is created with owned Strings, not borrowed data
    //
    async fn get<T: DeserializeOwned>(
        &self,
        endpoint: &str,
        query_params: Option<HashMap<String, String>>,
        headers: Option<HeaderMap>,
    ) -> Result<T> {
        let url = format!("{}/api/{}/{}", self.base_url, self.api_version, endpoint);

        let mut request = self.client.get(url);

        if let Some(query_params) = query_params {
            request = request.query(&query_params);
        }

        if let Some(headers) = headers {
            request = request.headers(headers);
        }

        let response = request.send().await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(eyre!("{}", error_text));
        }

        let data = response.json::<T>().await?;
        Ok(data)
    }
}

impl LighthouseAPIClient {
    pub async fn me(&self) -> Result<ApiUser> {
        let headers = self.auth_headers()?;
        self.get::<ApiUser>("user", None, Some(headers)).await
    }

    pub async fn projects(&self) {
        // TODO: implement
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

#[derive(Debug)]
struct LighthouseAPIClientBaseURL(String);

impl LighthouseAPIClientBaseURL {
    pub fn from(base_url: &str, environment: Env) -> Result<Self, String> {
        let pattern = match environment {
            // DEV: allow localhost or 0.0.0.0 (http or https, any port)
            Env::DEV => r"^https?://(localhost|0\.0\.0\.0)(:\d+)?(/.*)?$",
            // RELEASE: only allow https://*projectlighthouse.io
            Env::RELEASE => r"^https://([a-zA-Z0-9-]+\.)*projectlighthouse\.io(/.*)?$",
        };

        let re =
            regex::Regex::new(pattern).map_err(|e| format!("invalid regex pattern: {}", e))?;

        if re.is_match(base_url) {
            Ok(LighthouseAPIClientBaseURL(base_url.to_string()))
        } else {
            let err_msg = match environment {
                Env::DEV => "invalid URL: must be localhost in DEV environment",
                Env::RELEASE => {
                    "invalid URL: must be https://*.projectlighthouse.io in RELEASE environment"
                }
            };
            Err(err_msg.to_string())
        }
    }

    pub fn default_for_env(environment: Env) -> Self {
        let url = match environment {
            Env::DEV => "http://localhost:8000",
            Env::RELEASE => "https://api.projectlighthouse.io",
        };
        LighthouseAPIClientBaseURL(url.to_string())
    }
}

impl Default for LighthouseAPIClient {
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
                match LighthouseAPIClientBaseURL::from(&val, lux_env) {
                    Ok(url) => url,
                    Err(e) => {
                        log::warn!("invalid LUX_API_BASE_URL: {}. using default.", e);
                        LighthouseAPIClientBaseURL::default_for_env(Env::DEV)
                    }
                }
            }
            Err(_) => LighthouseAPIClientBaseURL::default_for_env(lux_env),
        };

        log::debug!("initiating lighthouse api with {}", base_url.0);

        LighthouseAPIClient::new(base_url, "v1", lux_env, None)
    }
}

impl fmt::Display for LighthouseAPIClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "cli_version: {} base_url: {} api_version: {} env: {}",
            VERSION, self.base_url, self.api_version, self.env
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    // Mutex to ensure env var tests don't interfere with each other
    static ENV_LOCK: Mutex<()> = Mutex::new(());

    // Helper to run tests with specific env vars, then restore original state
    fn with_env_vars<F, R>(vars: &[(&str, Option<&str>)], f: F) -> R
    where
        F: FnOnce() -> R,
    {
        let _guard = ENV_LOCK.lock().unwrap();

        // Save original values
        let originals: Vec<_> = vars
            .iter()
            .map(|(key, _)| (*key, env::var(*key).ok()))
            .collect();

        // Set new values
        for (key, value) in vars {
            match value {
                Some(v) => env::set_var(*key, v),
                None => env::remove_var(*key),
            }
        }

        let result = f();

        // Restore original values
        for (key, original) in originals {
            match original {
                Some(v) => env::set_var(key, v),
                None => env::remove_var(key),
            }
        }

        result
    }

    #[test]
    fn test_env_display_dev() {
        assert_eq!(format!("{}", Env::DEV), "dev");
    }

    #[test]
    fn test_env_display_release() {
        assert_eq!(format!("{}", Env::RELEASE), "release");
    }

    #[test]
    fn test_env_clone() {
        let env = Env::DEV;
        let cloned = env;
        assert_eq!(format!("{}", cloned), "dev");
    }

    #[test]
    fn test_lighthouse_api_base_url_dev_env() {
        // Valid localhost URLs in DEV
        assert!(LighthouseAPIClientBaseURL::from("http://localhost", Env::DEV).is_ok());
        assert!(LighthouseAPIClientBaseURL::from("https://localhost", Env::DEV).is_ok());
        assert!(LighthouseAPIClientBaseURL::from("http://localhost:8080", Env::DEV).is_ok());
        assert!(LighthouseAPIClientBaseURL::from("https://localhost:3000/api", Env::DEV).is_ok());

        // projectlighthouse.io NOT allowed in DEV
        assert!(
            LighthouseAPIClientBaseURL::from("https://projectlighthouse.io", Env::DEV).is_err()
        );
        assert!(
            LighthouseAPIClientBaseURL::from("https://api.projectlighthouse.io", Env::DEV).is_err()
        );

        // Invalid URLs in DEV
        assert!(LighthouseAPIClientBaseURL::from("ftp://localhost", Env::DEV).is_err()); // wrong scheme
        assert!(LighthouseAPIClientBaseURL::from("https://example.com", Env::DEV).is_err());
        // wrong domain
    }

    #[test]
    fn test_lighthouse_api_base_url_release_env() {
        // Valid projectlighthouse.io URLs in RELEASE (https only)
        assert!(
            LighthouseAPIClientBaseURL::from("https://projectlighthouse.io", Env::RELEASE).is_ok()
        );
        assert!(
            LighthouseAPIClientBaseURL::from("https://projectlighthouse.io/api", Env::RELEASE)
                .is_ok()
        );
        assert!(
            LighthouseAPIClientBaseURL::from("https://api.projectlighthouse.io", Env::RELEASE)
                .is_ok()
        );
        assert!(LighthouseAPIClientBaseURL::from(
            "https://api.projectlighthouse.io/v1",
            Env::RELEASE
        )
        .is_ok());

        // localhost NOT allowed in RELEASE
        assert!(LighthouseAPIClientBaseURL::from("http://localhost", Env::RELEASE).is_err());
        assert!(LighthouseAPIClientBaseURL::from("https://localhost:8080", Env::RELEASE).is_err());

        // Invalid URLs in RELEASE
        assert!(
            LighthouseAPIClientBaseURL::from("http://projectlighthouse.io", Env::RELEASE).is_err()
        ); // http not allowed
        assert!(LighthouseAPIClientBaseURL::from("https://example.com", Env::RELEASE).is_err());
        // wrong domain
    }

    #[test]
    fn test_lighthouse_api_base_url_dev_with_paths() {
        // Various path combinations
        assert!(LighthouseAPIClientBaseURL::from("http://localhost/", Env::DEV).is_ok());
        assert!(LighthouseAPIClientBaseURL::from("http://localhost/api/v1", Env::DEV).is_ok());
        assert!(LighthouseAPIClientBaseURL::from(
            "http://localhost:8080/api/v1/exercises",
            Env::DEV
        )
        .is_ok());
    }

    #[test]
    fn test_lighthouse_api_base_url_release_subdomains() {
        // Multiple subdomain levels
        assert!(LighthouseAPIClientBaseURL::from(
            "https://api.v2.projectlighthouse.io",
            Env::RELEASE
        )
        .is_ok());
        assert!(LighthouseAPIClientBaseURL::from(
            "https://staging.api.projectlighthouse.io",
            Env::RELEASE
        )
        .is_ok());
    }

    #[test]
    fn test_lighthouse_api_base_url_error_messages() {
        let dev_err =
            LighthouseAPIClientBaseURL::from("https://example.com", Env::DEV).unwrap_err();
        assert!(dev_err.contains("localhost"));
        assert!(dev_err.contains("DEV"));

        let release_err =
            LighthouseAPIClientBaseURL::from("http://localhost", Env::RELEASE).unwrap_err();
        assert!(release_err.contains("projectlighthouse.io"));
        assert!(release_err.contains("RELEASE"));
    }

    #[test]
    fn test_lighthouse_api_base_url_default_for_env_dev() {
        let url = LighthouseAPIClientBaseURL::default_for_env(Env::DEV);
        assert_eq!(url.0, "http://localhost:8000");
    }

    #[test]
    fn test_lighthouse_api_base_url_default_for_env_release() {
        let url = LighthouseAPIClientBaseURL::default_for_env(Env::RELEASE);
        assert_eq!(url.0, "https://api.projectlighthouse.io");
    }

    #[test]
    fn test_lighthouse_api_new() {
        let base_url = LighthouseAPIClientBaseURL::from("http://localhost:8080", Env::DEV).unwrap();
        let api = LighthouseAPIClient::new(base_url, "v2", Env::DEV, None);

        assert_eq!(api.base_url, "http://localhost:8080");
        assert_eq!(api.api_version, "v2");
    }

    #[test]
    fn test_lighthouse_api_new_release() {
        let base_url =
            LighthouseAPIClientBaseURL::from("https://api.projectlighthouse.io", Env::RELEASE)
                .unwrap();
        let api = LighthouseAPIClient::new(base_url, "v1", Env::RELEASE, None);

        assert_eq!(api.base_url, "https://api.projectlighthouse.io");
        assert_eq!(api.api_version, "v1");
    }

    #[test]
    fn test_lighthouse_api_default_no_env_vars() {
        with_env_vars(&[("LUX_ENV", None), ("LUX_API_BASE_URL", None)], || {
            let api = LighthouseAPIClient::default();
            // Should default to DEV with localhost
            assert_eq!(api.base_url, "http://localhost:8000");
            assert_eq!(api.api_version, "v1");
        });
    }

    #[test]
    fn test_lighthouse_api_default_release_env() {
        with_env_vars(
            &[("LUX_ENV", Some("RELEASE")), ("LUX_API_BASE_URL", None)],
            || {
                let api = LighthouseAPIClient::default();
                assert_eq!(api.base_url, "https://api.projectlighthouse.io");
                assert_eq!(api.api_version, "v1");
            },
        );
    }

    #[test]
    fn test_lighthouse_api_default_release_lowercase() {
        with_env_vars(
            &[("LUX_ENV", Some("release")), ("LUX_API_BASE_URL", None)],
            || {
                let api = LighthouseAPIClient::default();
                assert_eq!(api.base_url, "https://api.projectlighthouse.io");
            },
        );
    }

    #[test]
    fn test_lighthouse_api_default_dev_env_explicit() {
        with_env_vars(
            &[("LUX_ENV", Some("DEV")), ("LUX_API_BASE_URL", None)],
            || {
                let api = LighthouseAPIClient::default();
                assert_eq!(api.base_url, "http://localhost:8000");
            },
        );
    }

    #[test]
    fn test_lighthouse_api_default_invalid_env_defaults_to_dev() {
        with_env_vars(
            &[("LUX_ENV", Some("INVALID")), ("LUX_API_BASE_URL", None)],
            || {
                let api = LighthouseAPIClient::default();
                // Invalid env should default to DEV
                assert_eq!(api.base_url, "http://localhost:8000");
            },
        );
    }

    #[test]
    fn test_lighthouse_api_default_custom_base_url_dev() {
        with_env_vars(
            &[
                ("LUX_ENV", Some("DEV")),
                ("LUX_API_BASE_URL", Some("http://localhost:9000")),
            ],
            || {
                let api = LighthouseAPIClient::default();
                assert_eq!(api.base_url, "http://localhost:9000");
            },
        );
    }

    #[test]
    fn test_lighthouse_api_default_custom_base_url_release() {
        with_env_vars(
            &[
                ("LUX_ENV", Some("RELEASE")),
                (
                    "LUX_API_BASE_URL",
                    Some("https://staging.projectlighthouse.io"),
                ),
            ],
            || {
                let api = LighthouseAPIClient::default();
                assert_eq!(api.base_url, "https://staging.projectlighthouse.io");
            },
        );
    }

    #[test]
    fn test_lighthouse_api_default_invalid_base_url_falls_back() {
        with_env_vars(
            &[
                ("LUX_ENV", Some("DEV")),
                ("LUX_API_BASE_URL", Some("https://invalid.com")),
            ],
            || {
                let api = LighthouseAPIClient::default();
                // Invalid URL should fall back to DEV default
                assert_eq!(api.base_url, "http://localhost:8000");
            },
        );
    }

    #[test]
    fn test_lighthouse_api_default_invalid_base_url_release_falls_back() {
        with_env_vars(
            &[
                ("LUX_ENV", Some("RELEASE")),
                ("LUX_API_BASE_URL", Some("http://localhost:8080")),
            ],
            || {
                let api = LighthouseAPIClient::default();
                // localhost not allowed in RELEASE, should fall back to DEV default (per current logic)
                assert_eq!(api.base_url, "http://localhost:8000");
            },
        );
    }

    #[test]
    fn test_lighthouse_api_display() {
        with_env_vars(&[("LUX_ENV", None), ("LUX_API_BASE_URL", None)], || {
            let api = LighthouseAPIClient::default();
            let display = format!("{}", api);

            assert!(display.contains("cli_version:"));
            assert!(display.contains("base_url:"));
            assert!(display.contains("api_version:"));
            assert!(display.contains("env:"));
            assert!(display.contains(VERSION));
            assert!(display.contains("v1"));
        });
    }

    #[test]
    fn test_lighthouse_api_display_contains_all_fields() {
        let base_url = LighthouseAPIClientBaseURL::from("http://localhost:3000", Env::DEV).unwrap();
        let api = LighthouseAPIClient::new(base_url, "v2", Env::DEV, None);
        let display = format!("{}", api);

        assert!(display.contains("http://localhost:3000"));
        assert!(display.contains("v2"));
        assert!(display.contains("dev"));
    }

    #[test]
    fn test_lighthouse_api_display_release_env() {
        let base_url =
            LighthouseAPIClientBaseURL::from("https://api.projectlighthouse.io", Env::RELEASE)
                .unwrap();
        let api = LighthouseAPIClient::new(base_url, "v1", Env::RELEASE, None);
        let display = format!("{}", api);

        assert!(display.contains("https://api.projectlighthouse.io"));
        assert!(display.contains("release"));
    }
}
