use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tracing::warn;

const DEFAULT_BRANDING_CONFIG_PATH: &str = "/etc/reshape-netsetup/branding.toml";

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct BrandingConfig {
    pub product_name: String,
    pub company_name: String,
    pub headline: String,
    pub support_text: String,
    pub success_message: String,
    pub logo_path: String,
    pub background_path: String,
    pub favicon_path: String,
    pub help_title: String,
    pub help_text: String,
    pub help_links: Vec<HelpLink>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct HelpLink {
    pub label: String,
    pub url: String,
}

impl Default for BrandingConfig {
    fn default() -> Self {
        Self {
            product_name: "Network Setup".into(),
            company_name: "Reshape Biotech".into(),
            headline: "Network configuration".into(),
            support_text: "Connect this device to your network.".into(),
            success_message: "Your device is now online and ready to use.".into(),
            logo_path: "/branding/logo.png".into(),
            background_path: "".into(),
            favicon_path: "/branding/favicon.ico".into(),
            help_title: "Need help?".into(),
            help_text: "Use the resources below for setup guidance.".into(),
            help_links: Vec::new(),
        }
    }
}

#[derive(Debug, Deserialize)]
struct BrandingConfigOverrides {
    product_name: Option<String>,
    company_name: Option<String>,
    headline: Option<String>,
    support_text: Option<String>,
    success_message: Option<String>,
    logo_path: Option<String>,
    background_path: Option<String>,
    favicon_path: Option<String>,
    help_title: Option<String>,
    help_text: Option<String>,
    help_links: Option<Vec<HelpLink>>,
}

impl BrandingConfig {
    fn apply_overrides(mut self, overrides: BrandingConfigOverrides) -> Self {
        if let Some(v) = overrides.product_name {
            self.product_name = v;
        }
        if let Some(v) = overrides.company_name {
            self.company_name = v;
        }
        if let Some(v) = overrides.headline {
            self.headline = v;
        }
        if let Some(v) = overrides.support_text {
            self.support_text = v;
        }
        if let Some(v) = overrides.success_message {
            self.success_message = v;
        }
        if let Some(v) = overrides.logo_path {
            self.logo_path = v;
        }
        if let Some(v) = overrides.background_path {
            self.background_path = v;
        }
        if let Some(v) = overrides.favicon_path {
            self.favicon_path = v;
        }
        if let Some(v) = overrides.help_title {
            self.help_title = v;
        }
        if let Some(v) = overrides.help_text {
            self.help_text = v;
        }
        if let Some(v) = overrides.help_links {
            self.help_links = v;
        }
        self
    }
}

pub fn load_branding_config() -> BrandingConfig {
    let path = branding_config_path();

    let defaults = BrandingConfig::default();
    let raw = match std::fs::read_to_string(&path) {
        Ok(raw) => raw,
        Err(err) => {
            warn!(
                "Failed to read branding config at {}: {}. Falling back to defaults.",
                path, err
            );
            return defaults;
        }
    };

    match toml::from_str::<BrandingConfigOverrides>(&raw) {
        Ok(overrides) => defaults.apply_overrides(overrides),
        Err(err) => {
            warn!(
                "Invalid branding config in {}: {}. Falling back to defaults.",
                path, err
            );
            defaults
        }
    }
}

pub fn branding_config_path() -> String {
    std::env::var("BRANDING_CONFIG_PATH")
        .ok()
        .map(|p| p.trim().to_string())
        .filter(|p| !p.is_empty())
        .unwrap_or_else(|| DEFAULT_BRANDING_CONFIG_PATH.to_string())
}

pub fn branding_assets_dir(config_path: &str) -> String {
    if let Some(override_path) = std::env::var("BRANDING_ASSETS_DIR")
        .ok()
        .map(|p| p.trim().to_string())
        .filter(|p| !p.is_empty())
    {
        return override_path;
    }

    Path::new(config_path)
        .parent()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
        .to_string_lossy()
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::{BrandingConfig, branding_assets_dir};

    #[test]
    fn default_branding_is_generic() {
        let defaults = BrandingConfig::default();
        assert_eq!(defaults.product_name, "Network Setup");
        assert_eq!(
            defaults.success_message,
            "Your device is now online and ready to use."
        );
    }

    #[test]
    fn branding_assets_defaults_to_config_parent() {
        assert_eq!(
            branding_assets_dir("/etc/reshape-netsetup/branding.toml"),
            "/etc/reshape-netsetup"
        );
    }
}
