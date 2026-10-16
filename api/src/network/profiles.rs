use super::nmcli::parse_nmcli_terse_line;
use super::transaction::Commands;
use super::types::{
    Address, IpConfig, IpMethod, NetworkSettings, NetworkTransport, SavedNetwork,
    SavedNetworkDetails,
};
use super::{NetworkError, Result};
use std::collections::HashMap;

pub(super) async fn inspect(
    commands: &impl Commands,
    network: SavedNetwork,
) -> Result<SavedNetworkDetails> {
    let args = ["-t", "-f", "all", "connection", "show", "uuid", &network.id].map(str::to_owned);
    let raw = commands.run("nmcli", &args).await?;
    parse(network, &raw)
}

fn parse(network: SavedNetwork, raw: &str) -> Result<SavedNetworkDetails> {
    let properties: HashMap<String, String> = raw
        .lines()
        .filter_map(|line| {
            let mut fields = parse_nmcli_terse_line(line).into_iter();
            let key = fields.next()?;
            Some((key, fields.collect::<Vec<_>>().join(":")))
        })
        .collect();
    let get = |key: &str| {
        properties
            .get(key)
            .filter(|v| !v.is_empty() && v.as_str() != "--")
            .cloned()
    };
    let enterprise = get("802-1x.eap").is_some();
    let security = if enterprise {
        "enterprise"
    } else if get("802-11-wireless-security.key-mgmt").is_some() {
        "personal"
    } else {
        "open"
    };
    let mut addresses = Vec::new();
    if let Some(raw) = get("ipv4.addresses") {
        for value in raw.split(',').map(str::trim) {
            let (address, prefix) = value
                .split_once('/')
                .ok_or_else(|| NetworkError::Internal("Could not read saved IP address".into()))?;
            addresses.push(Address {
                address: address.into(),
                prefix: prefix
                    .parse()
                    .map_err(|_| NetworkError::Internal("Could not read saved IP prefix".into()))?,
            });
        }
    }
    let method = get("ipv4.method").unwrap_or_default();
    let unsupported = !matches!(method.as_str(), "auto" | "manual")
        || addresses.len() > 1
        || (enterprise
            && (get("802-1x.eap").as_deref() != Some("peap")
                || get("802-1x.phase2-auth").as_deref() != Some("mschapv2")))
        || (!enterprise
            && get("802-11-wireless-security.key-mgmt").is_some_and(|v| v != "wpa-psk"));
    let settings = NetworkSettings {
        id: None,
        ssid: get("802-11-wireless.ssid"),
        password: None,
        eap_login: get("802-1x.identity"),
        eap_password: None,
        ipv4: Some(IpConfig {
            method: if method == "manual" {
                IpMethod::Manual
            } else {
                IpMethod::Auto
            },
            address_data: Some(addresses),
            gateway: get("ipv4.gateway"),
            dns: Some(
                get("ipv4.dns")
                    .unwrap_or_default()
                    .split([',', ' '])
                    .filter(|s| !s.is_empty())
                    .map(str::to_owned)
                    .collect(),
            ),
        }),
    };
    Ok(SavedNetworkDetails {
        network, settings, security: security.into(),
        edit_unavailable: unsupported.then(|| "This profile uses settings that this editor cannot preserve. Configure it through your network administrator.".into()),
    })
}

pub(super) async fn retain_credentials(
    commands: &impl Commands,
    details: &SavedNetworkDetails,
    transport: &NetworkTransport,
    settings: &mut NetworkSettings,
) -> Result<()> {
    if details.network.transport != *transport
        || details.settings.ssid != settings.ssid
        || details.settings.eap_login != settings.eap_login
        || details.edit_unavailable.is_some()
    {
        return Err(NetworkError::ConnectionFailed(
            "The network or identity changed. Enter its password again.".into(),
        ));
    }
    let property = match details.security.as_str() {
        "enterprise" => "802-1x.password",
        "personal" => "802-11-wireless-security.psk",
        _ => return Ok(()),
    };
    let args = [
        "--show-secrets",
        "--escape",
        "no",
        "-g",
        property,
        "connection",
        "show",
        "uuid",
        &details.network.id,
    ]
    .map(str::to_owned);
    let secret = commands.run("nmcli", &args).await?;
    if secret.is_empty() || secret == "--" {
        return Err(NetworkError::ConnectionFailed(
            "The saved password is unavailable. Enter it again.".into(),
        ));
    }
    if details.security == "enterprise" {
        settings.eap_password = Some(secret);
    } else {
        settings.password = Some(secret);
    }
    settings.id = None;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    fn network() -> SavedNetwork {
        SavedNetwork {
            id: "example-uuid".into(),
            transport: NetworkTransport::Wifi,
            display_name: "Lab".into(),
            managed: true,
        }
    }
    #[test]
    fn inspection_preserves_escaped_names_and_never_serializes_secrets() {
        let details = parse(network(), "802-11-wireless.ssid:Lab\\:West\nipv4.method:manual\nipv4.addresses:192.168.1.20/24\nipv4.gateway:192.168.1.1\nipv4.dns:1.1.1.1,8.8.8.8\n802-11-wireless-security.key-mgmt:wpa-psk\n802-11-wireless-security.psk:secret-sentinel").unwrap();
        assert_eq!(details.settings.ssid.as_deref(), Some("Lab:West"));
        assert_eq!(
            details
                .settings
                .ipv4
                .as_ref()
                .unwrap()
                .dns
                .as_ref()
                .unwrap()
                .len(),
            2
        );
        assert!(
            !serde_json::to_string(&details)
                .unwrap()
                .contains("secret-sentinel")
        );
        assert!(details.edit_unavailable.is_none());
    }
    struct Secrets;
    #[async_trait]
    impl Commands for Secrets {
        async fn run(&self, program: &str, args: &[String]) -> Result<String> {
            assert_eq!(program, "nmcli");
            assert!(args.contains(&"--show-secrets".to_owned()));
            assert!(args.contains(&"show".to_owned()));
            Ok("saved-password".into())
        }
    }
    #[tokio::test]
    async fn credentials_are_retained_only_for_the_same_network_and_identity() {
        let details = parse(
            network(),
            "802-11-wireless.ssid:Lab\nipv4.method:auto\n802-11-wireless-security.key-mgmt:wpa-psk",
        )
        .unwrap();
        let mut settings = details.settings.clone();
        retain_credentials(&Secrets, &details, &NetworkTransport::Wifi, &mut settings)
            .await
            .unwrap();
        assert_eq!(settings.password.as_deref(), Some("saved-password"));
        settings.ssid = Some("Different network".into());
        assert!(
            retain_credentials(&Secrets, &details, &NetworkTransport::Wifi, &mut settings)
                .await
                .is_err()
        );
    }
}
