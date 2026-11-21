use std::env;
use std::fmt;
use std::fs;
use std::path::Path;

use serde::Deserialize;

#[derive(Clone, PartialEq, Eq)]
pub struct Config {
    pub data_listen: String,
    pub admin_listen: String,
    pub database_url: String,
    pub log_level: String,
    pub log_json: bool,
    pub log_mode: String,
    pub default_timeout_ms: u64,
    pub max_body_bytes: usize,
    pub upstream_base_url: Option<String>,
    pub upstream_api_key: Option<String>,
    pub anthropic_base_url: String,
    pub anthropic_api_key: Option<String>,
}

impl fmt::Debug for Config {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Config")
            .field("data_listen", &self.data_listen)
            .field("admin_listen", &self.admin_listen)
            .field("database_url", &self.database_url)
            .field("log_level", &self.log_level)
            .field("log_json", &self.log_json)
            .field("log_mode", &self.log_mode)
            .field("default_timeout_ms", &self.default_timeout_ms)
            .field("max_body_bytes", &self.max_body_bytes)
            .field(
                "upstream_base_url",
                &self
                    .upstream_base_url
                    .as_deref()
                    .map(|_| "<configured>")
                    .unwrap_or("<none>"),
            )
            .field(
                "upstream_api_key",
                &if self.upstream_api_key.is_some() {
                    "<redacted>"
                } else {
                    "<none>"
                },
            )
            .field("anthropic_base_url", &self.anthropic_base_url)
            .field(
                "anthropic_api_key",
                &if self.anthropic_api_key.is_some() {
                    "<redacted>"
                } else {
                    "<none>"
                },
            )
            .finish()
    }
}

#[derive(Debug)]
pub enum ConfigError {
    Io(std::io::Error),
    Toml(toml::de::Error),
    Invalid(String),
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(f, "cannot read config: {error}"),
            Self::Toml(error) => write!(f, "invalid TOML config: {error}"),
            Self::Invalid(message) => write!(f, "invalid config: {message}"),
        }
    }
}

impl std::error::Error for ConfigError {}

impl From<std::io::Error> for ConfigError {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error)
    }
}

impl From<toml::de::Error> for ConfigError {
    fn from(error: toml::de::Error) -> Self {
        Self::Toml(error)
    }
}

pub trait Env {
    fn get(&self, key: &str) -> Option<String>;
}

pub struct SystemEnv;

impl Env for SystemEnv {
    fn get(&self, key: &str) -> Option<String> {
        env::var(key).ok()
    }
}

#[derive(Default, Deserialize)]
struct TomlConfig {
    server: Option<ServerSection>,
    database: Option<DatabaseSection>,
    logging: Option<LoggingSection>,
    limits: Option<LimitsSection>,
}

#[derive(Default, Deserialize)]
struct ServerSection {
    data_listen: Option<String>,
    admin_listen: Option<String>,
}

#[derive(Default, Deserialize)]
struct DatabaseSection {
    url: Option<String>,
}

#[derive(Default, Deserialize)]
struct LoggingSection {
    mode: Option<String>,
    level: Option<String>,
    json: Option<bool>,
}

#[derive(Default, Deserialize)]
struct LimitsSection {
    default_timeout_ms: Option<u64>,
    max_body_bytes: Option<usize>,
}

impl Config {
    pub fn load() -> Result<Self, ConfigError> {
        let env = SystemEnv;
        warn_unknown_env();
        let path = env
            .get("GATEWAY_CONFIG")
            .unwrap_or_else(|| "./config.toml".to_owned());
        Self::load_from(&env, Path::new(&path))
    }

    pub fn load_from(env: &dyn Env, path: &Path) -> Result<Self, ConfigError> {
        let file: TomlConfig = if path.exists() {
            let contents = fs::read_to_string(path)?;
            toml::from_str(&contents)?
        } else {
            TomlConfig::default()
        };

        let mut config = Self {
            data_listen: "0.0.0.0:8080".to_owned(),
            admin_listen: "127.0.0.1:8081".to_owned(),
            database_url: "sqlite://./gateway.db".to_owned(),
            log_level: "info".to_owned(),
            log_json: true,
            log_mode: "metadata".to_owned(),
            default_timeout_ms: 30_000,
            max_body_bytes: 1_048_576,
            upstream_base_url: None,
            upstream_api_key: None,
            anthropic_base_url: "https://api.anthropic.com/v1".to_owned(),
            anthropic_api_key: None,
        };

        if let Some(server) = file.server {
            if let Some(value) = server.data_listen {
                config.data_listen = value;
            }
            if let Some(value) = server.admin_listen {
                config.admin_listen = value;
            }
        }
        if let Some(database) = file.database {
            if let Some(value) = database.url {
                config.database_url = value;
            }
        }
        if let Some(logging) = file.logging {
            if let Some(value) = logging.mode {
                config.log_mode = value;
            }
            if let Some(value) = logging.level {
                config.log_level = value;
            }
            if let Some(value) = logging.json {
                config.log_json = value;
            }
        }
        if let Some(limits) = file.limits {
            if let Some(value) = limits.default_timeout_ms {
                config.default_timeout_ms = value;
            }
            if let Some(value) = limits.max_body_bytes {
                config.max_body_bytes = value;
            }
        }

        if let Some(value) = env.get("GATEWAY_DATA_LISTEN") {
            config.data_listen = value;
        }
        if let Some(value) = env.get("GATEWAY_ADMIN_LISTEN") {
            config.admin_listen = value;
        }
        if let Some(value) = env.get("DATABASE_URL") {
            config.database_url = value;
        }
        if let Some(value) = env.get("RUST_LOG") {
            config.log_level = value;
        }
        if let Some(value) = env.get("GATEWAY_LOG_JSON") {
            config.log_json = parse_bool(&value).ok_or_else(|| {
                ConfigError::Invalid(format!("GATEWAY_LOG_JSON must be a boolean, got '{value}'"))
            })?;
        }
        if let Some(value) = env.get("GATEWAY_LOG_MODE") {
            config.log_mode = value;
        }
        if let Some(value) = env.get("GATEWAY_DEFAULT_TIMEOUT_MS") {
            config.default_timeout_ms = value.parse().map_err(|_| {
                ConfigError::Invalid("GATEWAY_DEFAULT_TIMEOUT_MS must be a number".to_owned())
            })?;
        }
        if let Some(value) = env.get("GATEWAY_MAX_BODY_BYTES") {
            config.max_body_bytes = value.parse().map_err(|_| {
                ConfigError::Invalid("GATEWAY_MAX_BODY_BYTES must be a number".to_owned())
            })?;
        }
        config.upstream_base_url = env.get("GATEWAY_UPSTREAM_BASE_URL");
        config.upstream_api_key = env.get("GATEWAY_UPSTREAM_API_KEY");
        if let Some(value) = env.get("GATEWAY_ANTHROPIC_BASE_URL") {
            config.anthropic_base_url = value;
        }
        config.anthropic_api_key = env.get("GATEWAY_ANTHROPIC_API_KEY");

        config.validate()?;
        Ok(config)
    }

    fn validate(&self) -> Result<(), ConfigError> {
        if self.data_listen == self.admin_listen {
            return Err(ConfigError::Invalid(
                "data and admin listeners must differ".to_owned(),
            ));
        }
        if !matches!(self.log_mode.as_str(), "off" | "metadata" | "full") {
            return Err(ConfigError::Invalid(format!(
                "logging.mode must be off, metadata, or full; got '{}'",
                self.log_mode
            )));
        }
        if self.max_body_bytes == 0 {
            return Err(ConfigError::Invalid(
                "limits.max_body_bytes must be greater than zero".to_owned(),
            ));
        }
        if self.default_timeout_ms == 0 {
            return Err(ConfigError::Invalid(
                "limits.default_timeout_ms must be greater than zero".to_owned(),
            ));
        }
        Ok(())
    }
}

fn parse_bool(value: &str) -> Option<bool> {
    match value.to_ascii_lowercase().as_str() {
        "true" | "1" | "yes" => Some(true),
        "false" | "0" | "no" => Some(false),
        _ => None,
    }
}

fn warn_unknown_env() {
    const KNOWN: [&str; 13] = [
        "GATEWAY_CONFIG",
        "GATEWAY_DATA_LISTEN",
        "GATEWAY_ADMIN_LISTEN",
        "GATEWAY_LOG_JSON",
        "GATEWAY_LOG_MODE",
        "GATEWAY_DEFAULT_TIMEOUT_MS",
        "GATEWAY_MAX_BODY_BYTES",
        "GATEWAY_UPSTREAM_BASE_URL",
        "GATEWAY_UPSTREAM_API_KEY",
        "GATEWAY_ANTHROPIC_BASE_URL",
        "GATEWAY_ANTHROPIC_API_KEY",
        "GATEWAY_MASTER_KEY",
        "GATEWAY_MASTER_KEY_PREVIOUS",
    ];
    for (key, _) in env::vars() {
        if key.starts_with("GATEWAY_") && !KNOWN.contains(&key.as_str()) {
            tracing::warn!(key = %key, "unknown gateway environment variable ignored");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    struct FakeEnv(HashMap<String, String>);

    impl Env for FakeEnv {
        fn get(&self, key: &str) -> Option<String> {
            self.0.get(key).cloned()
        }
    }

    fn missing_path() -> &'static Path {
        Path::new("/nonexistent/llmgw-config-test.toml")
    }

    #[test]
    fn applies_defaults_without_file_or_env() {
        let config = Config::load_from(&FakeEnv(HashMap::new()), missing_path()).expect("config");
        assert_eq!(config.data_listen, "0.0.0.0:8080");
        assert_eq!(config.admin_listen, "127.0.0.1:8081");
        assert_eq!(config.log_level, "info");
        assert!(config.log_json);
    }

    #[test]
    fn environment_overrides_defaults() {
        let mut values = HashMap::new();
        values.insert(
            "GATEWAY_DATA_LISTEN".to_owned(),
            "127.0.0.1:9000".to_owned(),
        );
        values.insert("RUST_LOG".to_owned(), "debug".to_owned());
        values.insert(
            "GATEWAY_ANTHROPIC_BASE_URL".to_owned(),
            "http://127.0.0.1:9999/v1".to_owned(),
        );
        values.insert(
            "GATEWAY_ANTHROPIC_API_KEY".to_owned(),
            "test-anthropic-key".to_owned(),
        );
        let config = Config::load_from(&FakeEnv(values), missing_path()).expect("config");
        assert_eq!(config.data_listen, "127.0.0.1:9000");
        assert_eq!(config.log_level, "debug");
        assert_eq!(config.anthropic_base_url, "http://127.0.0.1:9999/v1");
        assert!(config.anthropic_api_key.is_some());
    }

    #[test]
    fn environment_overrides_toml() {
        let directory = env::temp_dir();
        let path = directory.join(format!("llmgw-config-{}.toml", std::process::id()));
        fs::write(
            &path,
            "[server]\ndata_listen = '127.0.0.1:7000'\n[logging]\nmode = 'full'\n",
        )
        .expect("write config");

        let mut values = HashMap::new();
        values.insert(
            "GATEWAY_DATA_LISTEN".to_owned(),
            "127.0.0.1:8000".to_owned(),
        );
        let config = Config::load_from(&FakeEnv(values), &path).expect("config");
        fs::remove_file(&path).expect("remove config");

        assert_eq!(config.data_listen, "127.0.0.1:8000");
        assert_eq!(config.log_mode, "full");
    }

    #[test]
    fn rejects_invalid_log_mode() {
        let mut values = HashMap::new();
        values.insert("GATEWAY_LOG_MODE".to_owned(), "verbose".to_owned());
        let error = Config::load_from(&FakeEnv(values), missing_path()).unwrap_err();
        assert!(matches!(error, ConfigError::Invalid(_)));
    }

    #[test]
    fn rejects_identical_listeners() {
        let mut values = HashMap::new();
        values.insert("GATEWAY_DATA_LISTEN".to_owned(), "0.0.0.0:8080".to_owned());
        values.insert("GATEWAY_ADMIN_LISTEN".to_owned(), "0.0.0.0:8080".to_owned());
        let error = Config::load_from(&FakeEnv(values), missing_path()).unwrap_err();
        assert!(matches!(error, ConfigError::Invalid(_)));
    }
}
