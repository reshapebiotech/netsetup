use serde::{Deserialize, Serialize};
use std::{ffi::OsString, fs, path::Path};

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hostname: Option<String>,
    pub password: String,
    #[serde(
        default = "default_log_level",
        skip_serializing_if = "is_default_log_level"
    )]
    pub log_level: String,
}

fn default_log_level() -> String {
    "info".into()
}

fn is_default_log_level(value: &str) -> bool {
    value == "info"
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

    fn legacy(contents: &str) -> Result<Self, String> {
        let mut hostname = None;
        let mut password = None;
        for line in contents.lines() {
            let Some((key, value)) = line.split_once('=') else {
                continue;
            };
            let value = value.trim();
            let value = value
                .strip_prefix('"')
                .and_then(|s| s.strip_suffix('"'))
                .or_else(|| value.strip_prefix('\'').and_then(|s| s.strip_suffix('\'')))
                .unwrap_or(value);
            match key.trim() {
                "DEVICE_NAME" => hostname = Some(value.to_owned()),
                "HOTSPOT_PASSWORD" => password = Some(value.to_owned()),
                _ => {}
            }
        }
        let config = Self {
            hostname,
            password: password.ok_or("Legacy .env has no HOTSPOT_PASSWORD; provide --config")?,
            log_level: default_log_level(),
        };
        config.validate()?;
        Ok(config)
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
            [flag, path] if flag == "--import-legacy-config" || flag == "--legacy-ap-name" => {
                let contents =
                    fs::read_to_string(path).map_err(|_| "Could not read legacy .env")?;
                let config = Self::legacy(&contents)?;
                if flag == "--legacy-ap-name" {
                    println!(
                        "{}",
                        config.hostname.ok_or("Legacy .env has no DEVICE_NAME")?
                    );
                } else {
                    print!(
                        "{}",
                        toml::to_string(&config).map_err(|_| "Could not serialize config")?
                    );
                }
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
                println!(
                    "reshape-netsetup [--config PATH | --check-config PATH | --import-legacy-config PATH | --legacy-ap-name PATH | --version]"
                );
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

    #[test]
    fn imports_existing_ap_identity_and_password_without_executing_shell() {
        let config = Config::legacy("DEVICE_NAME=\"RIS0001\"\nHOTSPOT_PASSWORD=\"safe$(echo secret)\"\nPUBLIC_API_PORT=9000").unwrap();
        assert_eq!(config.hostname.as_deref(), Some("RIS0001"));
        assert_eq!(config.password, "safe$(echo secret)");
        assert_eq!(
            Config::parse(&toml::to_string(&config).unwrap())
                .unwrap()
                .password,
            config.password
        );
    }
}
