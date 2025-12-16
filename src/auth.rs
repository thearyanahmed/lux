use crate::{
    api::{ApiUser, LighthouseAPIClient},
    config::Config,
};
use color_eyre::eyre::{eyre, Ok, Result};

pub struct TokenAuthenticator {
    pub token: String,
}

impl TokenAuthenticator {
    pub fn new(token: &str) -> Self {
        TokenAuthenticator {
            token: token.to_string(),
        }
    }

    pub async fn authenticate(&self) -> Result<ApiUser> {
        if self.token.is_empty() {
            return Err(eyre!("token must not be empty."));
        }

        // Create a temporary config to build the client with token
        let cfg = Config::new(&self.token);
        let client = LighthouseAPIClient::from_config(&cfg);

        let user = client.me().await?;

        // Save config only after successful authentication
        cfg.save()?;

        Ok(user)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_authentication_with_empty_token_should_fail() {
        let token_authenticator = TokenAuthenticator::new("");

        let result = token_authenticator.authenticate().await;
        assert!(result.is_err());

        let error_msg = result.unwrap_err().to_string();
        assert_eq!(error_msg, "token must not be empty.");
    }
}
