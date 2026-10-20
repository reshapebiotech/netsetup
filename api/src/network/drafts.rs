use super::types::{
    ConnectionState, NetworkDraft, NetworkSettings, NetworkTransport, SavedNetwork,
    SavedNetworkDetails,
};
use super::{NetworkError, Result};
use std::collections::HashMap;
use std::time::{Duration, Instant};
use uuid::Uuid;

const RETENTION: Duration = Duration::from_secs(30 * 60);
const MAX_RECORDS: usize = 16;

#[derive(Clone)]
pub(super) struct TestRecord {
    pub state: ConnectionState,
    pub settings: NetworkSettings,
    pub transport: NetworkTransport,
    updated: Instant,
}

impl TestRecord {
    pub fn new(settings: NetworkSettings, transport: NetworkTransport) -> Self {
        Self {
            state: ConnectionState::Testing,
            settings,
            transport,
            updated: Instant::now(),
        }
    }

    pub fn finish(&mut self, state: ConnectionState) {
        self.state = state;
        self.updated = Instant::now();
        if matches!(self.state, ConnectionState::Saved(_)) {
            self.settings.password = None;
            self.settings.eap_password = None;
        }
    }

    pub fn summary(&self, id: Uuid) -> NetworkDraft {
        NetworkDraft {
            test_id: id,
            transport: self.transport.clone(),
            display_name: self
                .settings
                .ssid
                .clone()
                .unwrap_or_else(|| "Ethernet".into()),
            state: self.state.clone(),
        }
    }

    pub fn details(&self, id: Uuid) -> Result<SavedNetworkDetails> {
        if matches!(self.state, ConnectionState::Saved(_)) {
            return Err(NetworkError::NotFound(
                "This connection has already been saved.".into(),
            ));
        }
        let security = if self.settings.eap_login.is_some() {
            "enterprise"
        } else if self.settings.password.is_some() {
            "personal"
        } else {
            "open"
        };
        let mut settings = self.settings.clone();
        settings.password = None;
        settings.eap_password = None;
        settings.id = None;
        Ok(SavedNetworkDetails {
            network: SavedNetwork {
                id: id.to_string(),
                transport: self.transport.clone(),
                display_name: self.summary(id).display_name,
                managed: false,
            },
            settings,
            security: security.into(),
            edit_unavailable: None,
        })
    }

    pub fn retain_credentials(
        &self,
        transport: &NetworkTransport,
        settings: &mut NetworkSettings,
    ) -> Result<()> {
        if matches!(
            self.state,
            ConnectionState::Testing | ConnectionState::Saving | ConnectionState::Saved(_)
        ) {
            return Err(NetworkError::ConnectionFailed(
                "This test cannot be edited right now.".into(),
            ));
        }
        if self.transport != *transport
            || self.settings.ssid != settings.ssid
            || self.settings.eap_login != settings.eap_login
        {
            return Err(NetworkError::ConnectionFailed(
                "The network or identity changed. Enter its password again.".into(),
            ));
        }
        settings.password = self.settings.password.clone();
        settings.eap_password = self.settings.eap_password.clone();
        settings.id = None;
        Ok(())
    }
}

pub(super) fn insert(records: &mut HashMap<Uuid, TestRecord>, id: Uuid, record: TestRecord) {
    records.retain(|_, previous| {
        previous.transport != record.transport
            || matches!(
                previous.state,
                ConnectionState::Testing | ConnectionState::Saving | ConnectionState::Saved(_)
            )
    });
    records.insert(id, record);
    prune(records);
}

pub(super) fn prune(records: &mut HashMap<Uuid, TestRecord>) {
    records.retain(|_, record| {
        matches!(
            record.state,
            ConnectionState::Testing | ConnectionState::Saving
        ) || record.updated.elapsed() < RETENTION
    });
    while records.len() > MAX_RECORDS {
        let oldest = records
            .iter()
            .filter(|(_, record)| {
                !matches!(
                    record.state,
                    ConnectionState::Testing | ConnectionState::Saving
                )
            })
            .min_by_key(|(_, record)| record.updated)
            .map(|(id, _)| *id);
        if let Some(id) = oldest {
            records.remove(&id);
        } else {
            break;
        }
    }
}

pub(super) fn list(records: &mut HashMap<Uuid, TestRecord>) -> Vec<NetworkDraft> {
    prune(records);
    let mut drafts: Vec<_> = records
        .iter()
        .filter(|(_, record)| !matches!(record.state, ConnectionState::Saved(_)))
        .collect();
    drafts.sort_by_key(|(_, record)| std::cmp::Reverse(record.updated));
    drafts
        .into_iter()
        .map(|(id, record)| record.summary(*id))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    fn record() -> TestRecord {
        TestRecord::new(
            NetworkSettings {
                ssid: Some("Lab".into()),
                password: Some("secret".into()),
                id: None,
                eap_login: None,
                eap_password: None,
                ipv4: None,
            },
            NetworkTransport::Wifi,
        )
    }

    #[test]
    fn expires_finished_drafts_but_preserves_running_operations() {
        let mut running = record();
        running.updated -= RETENTION;
        let mut failed = running.clone();
        failed.state = ConnectionState::Failed("Restored".into());
        let running_id = Uuid::new_v4();
        let mut records = HashMap::from([(running_id, running), (Uuid::new_v4(), failed)]);
        prune(&mut records);
        assert_eq!(records.len(), 1);
        assert!(records.contains_key(&running_id));
    }

    #[test]
    fn recovery_redacts_secrets_and_retains_them_only_for_same_identity() {
        let mut record = record();
        record.state = ConnectionState::Failed("Restored".into());
        let details = record.details(Uuid::new_v4()).unwrap();
        assert_eq!(details.settings.password, None);
        assert_eq!(details.settings.eap_password, None);
        let mut settings = details.settings;
        record
            .retain_credentials(&NetworkTransport::Wifi, &mut settings)
            .unwrap();
        assert_eq!(settings.password.as_deref(), Some("secret"));
        settings.ssid = Some("Other".into());
        assert!(
            record
                .retain_credentials(&NetworkTransport::Wifi, &mut settings)
                .is_err()
        );
        settings.ssid = record.settings.ssid.clone();
        settings.eap_login = Some("other-person".into());
        assert!(
            record
                .retain_credentials(&NetworkTransport::Wifi, &mut settings)
                .is_err()
        );
    }

    #[test]
    fn enterprise_recovery_redacts_password_and_rejects_changed_identity() {
        let mut record = record();
        record.state = ConnectionState::Failed("Restored".into());
        record.settings.password = None;
        record.settings.eap_login = Some("operator".into());
        record.settings.eap_password = Some("enterprise-secret".into());
        let details = record.details(Uuid::new_v4()).unwrap();
        assert_eq!(details.security, "enterprise");
        assert!(
            !serde_json::to_string(&details)
                .unwrap()
                .contains("enterprise-secret")
        );
        let mut settings = details.settings;
        record
            .retain_credentials(&NetworkTransport::Wifi, &mut settings)
            .unwrap();
        assert_eq!(settings.eap_password.as_deref(), Some("enterprise-secret"));
        assert!(
            record
                .retain_credentials(&NetworkTransport::Ethernet, &mut settings)
                .is_err()
        );
        settings.eap_login = Some("other".into());
        assert!(
            record
                .retain_credentials(&NetworkTransport::Wifi, &mut settings)
                .is_err()
        );
    }

    #[test]
    fn limits_abandoned_records() {
        let mut records = HashMap::new();
        for _ in 0..32 {
            let mut record = record();
            record.state = ConnectionState::Failed("Restored".into());
            records.insert(Uuid::new_v4(), record);
        }
        prune(&mut records);
        assert_eq!(records.len(), MAX_RECORDS);
    }
}
