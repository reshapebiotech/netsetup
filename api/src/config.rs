use serde::Deserialize;
use std::{ffi::OsString, fs, path::Path};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    #[serde(default)]
    pub hostname: Option<String>,
    pub password: String,
    #[serde(default = "default_log_level")]
    pub log_level: String,
}

fn default_log_level() -> String {
    "info".into()
}

impl Config {
    fn parse(contents: &str) -> Result<Self, String> {
        // Parser diagnostics can include the source line containing the password.
        let config: Self = toml::from_str(contents).map_err(|_| {
            "Invalid config: expected password, optional hostname, and optional log_level"
                .to_owned()
        })?;
        config.validate()?;
        Ok(config)
    }

    fn validate(&self) -> Result<(), String> {
        if !(8..=63).contains(&self.password.len())
            || !self.password.bytes().all(|byte| (32..=126).contains(&byte))
        {
            return Err("AP password must contain 8–63 printable ASCII characters".into());
        }
        if self.hostname.as_ref().is_some_and(|name| {
            name.is_empty() || name.len() > 32 || name.chars().any(char::is_control)
        }) {
            return Err("AP hostname must contain 1–32 bytes without control characters".into());
        }
        if !["error", "warn", "info", "debug", "trace"].contains(&self.log_level.as_str()) {
            return Err("log_level must be error, warn, info, debug, or trace".into());
        }
        Ok(())
    }

    fn load(path: &Path) -> Result<Self, String> {
        let contents = fs::read_to_string(path)
            .map_err(|error| format!("Could not read config {}: {error}", path.display()))?;
        Self::parse(&contents)
    }

    pub fn command_line() -> Result<Self, String> {
        let args: Vec<OsString> = std::env::args_os().skip(1).collect();
        match args.as_slice() {
            [] => {
                let config = Self {
                    hostname: std::env::var("HOSTNAME").ok(),
                    password: std::env::var("DEVICE_AP_PASSWORD")
                        .unwrap_or_else(|_| "reshape123".into()),
                    log_level: default_log_level(),
                };
                config.validate()?;
                Ok(config)
            }
            [flag, path] if flag == "--config" => Self::load(Path::new(path)),
            [flag, path] if flag == "--check-config" => {
                Self::load(Path::new(path))?;
                println!("Configuration valid");
                std::process::exit(0);
            }
            [flag] if flag == "--version" => {
                println!(
                    "reshape-netsetup {}",
                    option_env!("RESHAPE_NETSETUP_VERSION").unwrap_or(env!("CARGO_PKG_VERSION"))
                );
                std::process::exit(0);
            }
            [flag] if flag == "--help" => {
                println!("reshape-netsetup [--config PATH | --check-config PATH | --version]");
                std::process::exit(0);
            }
            _ => Err("Unknown arguments; use --help".into()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn minimal_configuration_uses_device_hostname_and_normal_logging() {
        let config = Config::parse("password = 'wifi password'").unwrap();
        assert!(config.hostname.is_none());
        assert_eq!(config.log_level, "info");
    }

    #[test]
    fn invalid_configuration_does_not_echo_credentials() {
        for contents in [
            "password = 'secret'",
            "password = 'super-secret'\nunknown = 1",
            "password = 'super-secret",
        ] {
            let error = Config::parse(contents).err().unwrap();
            assert!(!error.contains("secret"));
        }
        assert!(Config::parse("password = 'super-secret'\nlog_level = 'invalid'").is_err());
        assert!(Config::parse("password = 'super-secret'\nhostname = ''").is_err());
    }
}
