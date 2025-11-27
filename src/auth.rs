use color_eyre::eyre::{ Result, eyre};
use crate::api::LighthouseAPI;

pub struct TokenAuthenticator {
    pub token: String,
    client: LighthouseAPI,
}

#[derive(Debug)]
pub struct ApiUser {
    id: i32,
    name: String, 
}

impl ApiUser {
    pub fn id(&self) -> i32{
        self.id
    }

    pub fn name(&self) -> &str {
        &self.name
    }
}

impl TokenAuthenticator {
    pub fn new(token: &str, client: LighthouseAPI) -> Self {
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


        // it will make an api call, that can return 
        // store it it in ~/.lux/cfg
        // return a simple user adta, that the caller will use to display `welcome $username`

        let fake_user = ApiUser {
            id: 1, 
            name: "not-an-user".to_string(),
        };

        Ok(fake_user)
    }
}    

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::LighthouseAPI;

    #[tokio::test]
    async fn test_authentication_with_empty_token_should_fail() {
        let api_client = LighthouseAPI::default();
        let token_authenticator = TokenAuthenticator::new("", api_client);

        let result = token_authenticator.authenticate().await;
        assert!(result.is_err());

        let error_msg = result.unwrap_err().to_string();
        assert_eq!(error_msg, "token must not be empty." );
    }
}
