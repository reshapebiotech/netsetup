use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::{ffi::OsString, fs, path::Path};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    #[serde(default)]
    pub hostname: Option<String>,
    pub password: String,
    #[serde(default = "default_log_level")]
    pub log_level: String,
    #[serde(default)]
    pub connectivity: ConnectivityConfig,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ConnectivityConfig {
    #[serde(default)]
    pub groups: Vec<EndpointGroup>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct EndpointGroup {
    pub name: String,
    #[serde(default)]
    pub required: bool,
    pub endpoints: Vec<Endpoint>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Endpoint {
    pub name: String,
    pub url: String,
    #[serde(default)]
    pub description: String,
}

impl ConnectivityConfig {
    fn validate(&self) -> Result<(), String> {
        let valid_label = |value: &str| {
            !value.trim().is_empty() && value.len() <= 120 && !value.chars().any(char::is_control)
        };
        let mut names = HashSet::new();
        let mut urls = HashSet::new();
        if self.groups.len() > 8 {
            return Err("Connectivity supports at most 8 groups".into());
        }
        for group in &self.groups {
            if !valid_label(&group.name)
                || !names.insert(group.name.clone())
                || group.endpoints.is_empty()
            {
                return Err(
                    "Connectivity groups need unique names and at least one endpoint".into(),
                );
            }
            let mut endpoint_names = HashSet::new();
            for endpoint in &group.endpoints {
                if !valid_label(&endpoint.name)
                    || !endpoint_names.insert(endpoint.name.clone())
                    || endpoint.description.len() > 500
                    || endpoint.description.chars().any(char::is_control)
                {
                    return Err("Connectivity endpoints need unique names within their group and short descriptions".into());
                }
                let url = url::Url::parse(&endpoint.url).map_err(|_| {
                    "Connectivity endpoints must use absolute HTTPS URLs".to_string()
                })?;
                if url.scheme() != "https"
                    || url.host_str().is_none()
                    || !url.username().is_empty()
                    || url.password().is_some()
                    || url.fragment().is_some()
                    || matches!(url.host(), Some(url::Host::Ipv6(_)))
                    || endpoint.url.len() > 2048
                    || endpoint.url.chars().any(char::is_control)
                {
                    return Err("Connectivity endpoints must use HTTPS URLs without credentials or fragments (IPv4 only)".into());
                }
                if !urls.insert(url.to_string()) {
                    return Err("Connectivity endpoint URLs must be unique".into());
                }
            }
        }
        if urls.len() > 16 {
            return Err("Connectivity supports at most 16 endpoints".into());
        }
        Ok(())
    }
}

fn default_log_level() -> String {
    "info".into()
}

impl Config {
    fn parse(contents: &str) -> Result<Self, String> {
        // Parser diagnostics can include the source line containing the password.
        let config: Self = toml::from_str(contents)
            .map_err(|_| "Invalid TOML configuration or unrecognized settings".to_owned())?;
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
        self.connectivity.validate()
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
                    connectivity: ConnectivityConfig::default(),
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
    fn endpoint_configuration_is_optional_and_rejects_invalid_groups_without_echoing_urls() {
        let valid = r#"password = "wifi password"
[[connectivity.groups]]
name = "Core services"
required = true
endpoints = [{name = "API", url = "https://api.example.com:8443/health?probe=1", description = "Device communication"}]
"#;
        let config = Config::parse(valid).unwrap();
        assert_eq!(config.connectivity.groups[0].endpoints.len(), 1);
        for url in [
            "http://example.com",
            "https://user:secret@example.com",
            "https://example.com/#fragment",
            "https://[::1]/",
            "invalid",
        ] {
            let bad = valid.replace("https://api.example.com:8443/health?probe=1", url);
            let error = Config::parse(&bad).err().unwrap();
            assert!(!error.contains("secret"));
        }
        let group = config.connectivity.groups[0].clone();
        assert!(
            ConnectivityConfig {
                groups: vec![group.clone(), group.clone()]
            }
            .validate()
            .is_err()
        );
        let mut empty = group.clone();
        empty.endpoints.clear();
        assert!(
            ConnectivityConfig {
                groups: vec![empty]
            }
            .validate()
            .is_err()
        );
        let mut duplicate = group.clone();
        duplicate.endpoints.push(group.endpoints[0].clone());
        assert!(
            ConnectivityConfig {
                groups: vec![duplicate]
            }
            .validate()
            .is_err()
        );
        assert!(
            Config::parse("password = 'wifi password'")
                .unwrap()
                .connectivity
                .groups
                .is_empty()
        );
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
