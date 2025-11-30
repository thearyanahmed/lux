use color_eyre::eyre::{ Result, eyre};
use crate::{api::{ApiUser, LighthouseAPIClient}, config::Config};

pub struct TokenAuthenticator {
    pub token: String,
    client: LighthouseAPIClient,
}

impl TokenAuthenticator {
    pub fn new( client: LighthouseAPIClient, token: &str) -> Self {
        TokenAuthenticator {
            token: token.to_string(),
            client,
        }
    }

    pub async fn authenticate(&self) -> Result<ApiUser> {
        // sanity checkn
        if self.token.is_empty(){
            // return Error invalid error 
            return Err(eyre!("token must not be empty."))
        }

        let cfg = Config::new(&self.token);
        
        cfg.save()?;

        match self.client.me(&self.token).await {
            Ok(user) => Ok(user), 
            Err(err) => Err(eyre!("{}", err))
        }
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
        assert_eq!(error_msg, "token must not be empty." );
    }
}
