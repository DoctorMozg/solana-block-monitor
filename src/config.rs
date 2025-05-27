use std::env;
use tokio::fs;
use tracing::Level;

#[derive(Debug, Clone)]
pub struct Config {
    pub solana_rpc_url: String,
    pub server_port: u16,
    pub log_level: String,
    pub monitor_interval_ms: u64,
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

            if let Some((key, value)) = parse_env_line(line) {
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

    fn build_config() -> Result<Self, ConfigError> {
        let solana_rpc_url = env::var("SOLANA_RPC_URL")
            .map_err(|_| ConfigError::MissingVariable("SOLANA_RPC_URL".to_string()))?;

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

        Ok(Config {
            solana_rpc_url,
            server_port,
            log_level,
            monitor_interval_ms,
        })
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use tokio::fs;

    #[test]
    fn test_parse_env_line() {
        assert_eq!(parse_env_line("KEY=value"), Some(("KEY", "value")));
        assert_eq!(
            parse_env_line("KEY=\"quoted value\""),
            Some(("KEY", "quoted value"))
        );
        assert_eq!(
            parse_env_line("KEY='single quoted'"),
            Some(("KEY", "single quoted"))
        );
        assert_eq!(parse_env_line("# comment"), None);
        assert_eq!(parse_env_line(""), None);
        assert_eq!(parse_env_line("INVALID_LINE"), None);
    }

    #[tokio::test]
    async fn test_load_from_env_file() {
        let test_content = r#"
# Test configuration
SOLANA_RPC_URL=https://test-rpc.solana.com
SERVER_PORT=3000
LOG_LEVEL=debug
MONITOR_INTERVAL_MS=1000
"#;

        fs::write("test.env", test_content).await.unwrap();

        let config = Config::load_from_env_file("test.env").await.unwrap();
        assert_eq!(config.solana_rpc_url, "https://test-rpc.solana.com");
        assert_eq!(config.server_port, 3000);
        assert_eq!(config.log_level, "debug");
        assert_eq!(config.monitor_interval_ms, 1000);

        fs::remove_file("test.env").await.unwrap();

        unsafe {
            env::remove_var("SOLANA_RPC_URL");
            env::remove_var("SERVER_PORT");
            env::remove_var("LOG_LEVEL");
            env::remove_var("MONITOR_INTERVAL_MS");
        }
    }
}
