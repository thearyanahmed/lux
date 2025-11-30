use std::{collections::HashMap, fs, path::PathBuf,path::Path};
use color_eyre::eyre::{self, Ok};
use secrecy::{ExposeSecret, SecretString};

// we'll always use this path.
static CFG_DIR : &str = ".lux";
static CFG_FILE : &str = "cfg";


#[derive(Debug)]
pub struct Config {
    token: SecretString,
}

impl Config {
    pub fn new(token : &str) -> Config {
        Config {
            token: SecretString::from(token)
        }
    }

    pub fn token(&self) -> &SecretString {
        &self.token
    }

    pub fn expose_token(&self) -> &str {
        self.token.expose_secret()
    }
}

impl Config {
    fn config_path() -> Result<PathBuf, eyre::Error> {
        let home = dirs::home_dir()
            .ok_or_else(|| eyre::eyre!("could not determine home dir"))?;

        Ok(home.join(CFG_DIR).join(CFG_FILE))
    }

    pub fn load() -> Result<Config, eyre::Error> {
        let path = Self::config_path()?;
        Self::load_from_path(&path)
    }

    fn load_from_path(path: &PathBuf) -> Result<Config, eyre::Error> {
        let content = fs::read_to_string(path)
            .map_err(|e| eyre::eyre!("failed to read config file: {}", e))?;

        let map: HashMap<&str, &str> = content
            .lines()
            .filter(|line| !line.trim().is_empty())
            .filter_map(|line| {
                let mut parts = line.splitn(2, "=");
                Some((parts.next()?.trim(), parts.next()?.trim()))
            })
            .collect();

        let token = map.get("token")
            .copied()
            .ok_or_else(|| eyre::eyre!("token not found in config"))?;

        Ok(Config::new(token))
    }

    pub fn exists() -> Result<bool, eyre::Error> {
        let path = Self::config_path()?;
        Self::exists_at_path(&path)
    }

    fn exists_at_path(path: &Path)-> Result<bool, eyre::Error> {
        Ok(path.exists())
    }

    pub fn save(&self) -> Result<bool, eyre::Error> {
        let path = Self::config_path()?;
        self.save_to_path(&path)
    }

    fn save_to_path(&self, path: &PathBuf) -> Result<bool, eyre::Error> {
        if let Some(dir) = path.parent() {
            fs::create_dir_all(dir)?;
            log::debug!("creating all dir {}", dir.display());
        }

        let content = format!("token={}\n", self.expose_token());
        fs::write(path, content)?;
        log::debug!("config written successfully to path {}", path.display());

        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn temp_config_path(temp_dir: &TempDir) -> PathBuf {
        temp_dir.path().join("cfg")
    }

    #[test]
    fn test_new_creates_config_with_token() {
        let config = Config::new("test-token");
        assert_eq!(config.expose_token(), "test-token");
    }

    #[test]
    fn test_token_returns_secret_string() {
        let config = Config::new("secret");
        assert_eq!(config.token().expose_secret(), "secret");
    }

    #[test]
    fn test_save_and_load_roundtrip() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_config_path(&temp_dir);

        let config = Config::new("my-secret-token");
        let save_result = config.save_to_path(&path);
        assert!(save_result.is_ok());
        assert!(save_result.unwrap());

        let loaded = Config::load_from_path(&path);
        assert!(loaded.is_ok());
        assert_eq!(loaded.unwrap().expose_token(), "my-secret-token");
    }

    #[test]
    fn test_exists_returns_true_when_file_exists() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_config_path(&temp_dir);

        let config = Config::new("token");
        config.save_to_path(&path).unwrap();

        let exists = Config::exists_at_path(&path);
        assert!(exists.is_ok());
        assert!(exists.unwrap());
    }

    #[test]
    fn test_exists_returns_false_when_no_file() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_config_path(&temp_dir);

        let exists = Config::exists_at_path(&path);
        assert!(exists.is_ok());
        assert!(!exists.unwrap());
    }

    #[test]
    fn test_load_fails_when_file_missing() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_config_path(&temp_dir);

        let loaded = Config::load_from_path(&path);
        assert!(loaded.is_err());
    }

    #[test]
    fn test_load_fails_when_token_missing() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_config_path(&temp_dir);

        fs::write(&path, "other_key=value\n").unwrap();

        let loaded = Config::load_from_path(&path);
        assert!(loaded.is_err());
        assert!(loaded.unwrap_err().to_string().contains("token not found"));
    }

    #[test]
    fn test_load_handles_whitespace() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_config_path(&temp_dir);

        fs::write(&path, "  token  =  spaced-token  \n").unwrap();

        let loaded = Config::load_from_path(&path);
        assert!(loaded.is_ok());
        assert_eq!(loaded.unwrap().expose_token(), "spaced-token");
    }

    #[test]
    fn test_load_handles_empty_lines() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_config_path(&temp_dir);

        fs::write(&path, "\n\ntoken=valid\n\n").unwrap();

        let loaded = Config::load_from_path(&path);
        assert!(loaded.is_ok());
        assert_eq!(loaded.unwrap().expose_token(), "valid");
    }

    #[test]
    fn test_load_handles_token_with_equals_sign() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_config_path(&temp_dir);

        fs::write(&path, "token=abc=def=ghi\n").unwrap();

        let loaded = Config::load_from_path(&path);
        assert!(loaded.is_ok());
        assert_eq!(loaded.unwrap().expose_token(), "abc=def=ghi");
    }

    #[test]
    fn test_save_creates_parent_directories() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().join("nested").join("dir").join("cfg");

        let config = Config::new("token");
        let result = config.save_to_path(&path);

        assert!(result.is_ok());
        assert!(path.exists());
    }

    #[test]
    fn test_config_path_returns_expected_path() {
        let path = Config::config_path();
        assert!(path.is_ok());

        let path = path.unwrap();
        assert!(path.ends_with(".lux/cfg"));
    }
}
