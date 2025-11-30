use crate::{
    api::{ApiUser, LighthouseAPIClient},
    config::Config,
};
use color_eyre::eyre::{eyre, Ok, Result};

pub struct TokenAuthenticator {
    pub token: String,
    client: LighthouseAPIClient,
}

impl TokenAuthenticator {
    pub fn new(client: LighthouseAPIClient, token: &str) -> Self {
        TokenAuthenticator {
            token: token.to_string(),
            client,
        }
    }

    pub async fn authenticate(&self) -> Result<ApiUser> {
        // sanity checkn
        if self.token.is_empty() {
            // return Error invalid error
            return Err(eyre!("token must not be empty."));
        }

        let user = self.client.me(&self.token).await?;

        let cfg = Config::new(&self.token);

        cfg.save()?;

        Ok(user)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::LighthouseAPIClient;

    #[tokio::test]
    async fn test_authentication_with_empty_token_should_fail() {
        let api_client = LighthouseAPIClient::default();
        let token_authenticator = TokenAuthenticator::new(api_client, "");

        let result = token_authenticator.authenticate().await;
        assert!(result.is_err());

        let error_msg = result.unwrap_err().to_string();
        assert_eq!(error_msg, "token must not be empty.");
    }
}
