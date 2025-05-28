use std::env;
use tokio::fs;
use tracing::Level;

/// Configuration loading from environment variables
///
/// This implementation provides a custom environment variable loading mechanism
/// that:
/// - Loads variables from a .env file
/// - Supports comments and empty lines
/// - Validates required variables
/// - Provides clear error messages
///
/// While there are libraries like dotenv that provide similar functionality,
/// this custom implementation was chosen as i can't use other crates.
///
/// The implementation uses async I/O for file reading to avoid blocking
/// the main thread during configuration loading.

#[derive(Debug, Clone)]
pub struct Config {
    pub solana_rpc_url: String,
    pub solana_rpc_key: String,
    pub server_port: u16,
    pub log_level: String,
    pub monitor_interval_ms: u64,
    pub monitoring_depth: usize,
}

#[derive(Debug)]
pub enum ConfigError {
    FileNotFound(String),
    ParseError(String),
    MissingVariable(String),
    IoError(std::io::Error),
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConfigError::FileNotFound(path) => write!(f, "Config file not found: {}", path),
            ConfigError::ParseError(msg) => write!(f, "Parse error: {}", msg),
            ConfigError::MissingVariable(key) => write!(f, "Missing required variable: {}", key),
            ConfigError::IoError(err) => write!(f, "IO error: {}", err),
        }
    }
}

impl std::error::Error for ConfigError {}

impl From<std::io::Error> for ConfigError {
    fn from(err: std::io::Error) -> Self {
        ConfigError::IoError(err)
    }
}

impl Config {
    pub async fn load_from_env_file(path: &str) -> Result<Self, ConfigError> {
        let content = fs::read_to_string(path).await?;

        for (line_num, line) in content.lines().enumerate() {
            let line = line.trim();

            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            if let Some((key, value)) = Config::parse_env_line(line) {
                unsafe {
                    env::set_var(key, value);
                }
            } else {
                return Err(ConfigError::ParseError(format!(
                    "Invalid format at line {}: {}",
                    line_num + 1,
                    line
                )));
            }
        }

        Self::build_config()
    }

    pub async fn load() -> Result<Self, ConfigError> {
        Self::load_from_env_file(".env").await
    }

    pub fn get_tracing_level(&self) -> Level {
        match self.log_level.to_lowercase().as_str() {
            "trace" => Level::TRACE,
            "debug" => Level::DEBUG,
            "info" => Level::INFO,
            "warn" => Level::WARN,
            "error" => Level::ERROR,
            _ => Level::INFO,
        }
    }

    fn parse_env_line(line: &str) -> Option<(&str, &str)> {
        let mut parts = line.splitn(2, '=');
        let key = parts.next()?.trim();
        let value = parts.next()?.trim();

        if key.is_empty() {
            return None;
        }

        let value = if (value.starts_with('"') && value.ends_with('"'))
            || (value.starts_with('\'') && value.ends_with('\''))
        {
            &value[1..value.len() - 1]
        } else {
            value
        };

        Some((key, value))
    }

    fn build_config() -> Result<Self, ConfigError> {
        let solana_rpc_url = env::var("SOLANA_RPC_URL")
            .map_err(|_| ConfigError::MissingVariable("SOLANA_RPC_URL".to_string()))?;

        let solana_rpc_key = env::var("SOLANA_RPC_KEY")
            .map_err(|_| ConfigError::MissingVariable("SOLANA_RPC_KEY".to_string()))?;

        let server_port = env::var("SERVER_PORT")
            .map_err(|_| ConfigError::MissingVariable("SERVER_PORT".to_string()))?
            .parse()
            .map_err(|_| ConfigError::ParseError("Invalid SERVER_PORT value".to_string()))?;

        let log_level = env::var("LOG_LEVEL")
            .map_err(|_| ConfigError::MissingVariable("LOG_LEVEL".to_string()))?;

        let monitor_interval_ms = env::var("MONITOR_INTERVAL_MS")
            .map_err(|_| ConfigError::MissingVariable("MONITOR_INTERVAL_MS".to_string()))?
            .parse()
            .map_err(|_| {
                ConfigError::ParseError("Invalid MONITOR_INTERVAL_MS value".to_string())
            })?;

        let monitoring_depth = env::var("MONITORING_DEPTH")
            .map_err(|_| ConfigError::MissingVariable("MONITORING_DEPTH".to_string()))?
            .parse()
            .map_err(|_| ConfigError::ParseError("Invalid MONITORING_DEPTH value".to_string()))?;

        Ok(Config {
            solana_rpc_url,
            solana_rpc_key,
            server_port,
            log_level,
            monitor_interval_ms,
            monitoring_depth,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use tokio::fs;

    #[tokio::test]
    async fn test_load_from_env_file() {
        let test_content = r#"
# Test configuration
SOLANA_RPC_URL=https://test-rpc.solana.com
SOLANA_RPC_KEY=test-rpc-key
SERVER_PORT=3000
LOG_LEVEL=debug
MONITOR_INTERVAL_MS=1000
MONITORING_DEPTH=50
"#;

        fs::write("test.env", test_content).await.unwrap();

        let config = Config::load_from_env_file("test.env").await.unwrap();
        assert_eq!(config.solana_rpc_url, "https://test-rpc.solana.com");
        assert_eq!(config.solana_rpc_key, "test-rpc-key");
        assert_eq!(config.server_port, 3000);
        assert_eq!(config.log_level, "debug");
        assert_eq!(config.monitor_interval_ms, 1000);
        assert_eq!(config.monitoring_depth, 50);

        fs::remove_file("test.env").await.unwrap();

        unsafe {
            env::remove_var("SOLANA_RPC_URL");
            env::remove_var("SOLANA_RPC_KEY");
            env::remove_var("SERVER_PORT");
            env::remove_var("LOG_LEVEL");
            env::remove_var("MONITOR_INTERVAL_MS");
            env::remove_var("MONITORING_DEPTH");
        }
    }
}
