use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SupervisorAction {
    Enable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SupervisorInput {
    pub test_in_progress: bool,
    pub save_in_progress: bool,
    pub uplink_healthy: bool,
    pub hotspot_enabled: bool,
    pub boot_grace_elapsed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SupervisorDecision {
    pub action: Option<SupervisorAction>,
    pub automation_paused: bool,
    pub persistent_enabled: bool,
    pub resolved: bool,
    pub last_reason: String,
}

#[derive(Debug, Clone)]
pub struct ApSupervisorMachine {
    persistent_enabled: bool,
    boot_decision_resolved: bool,
    last_hotspot_enabled: Option<bool>,
    last_reason: String,
}

pub const DEFAULT_BOOT_GRACE: Duration = Duration::from_secs(90);

impl ApSupervisorMachine {
    pub fn new() -> Self {
        Self::for_boot_elapsed(Some(Duration::ZERO), DEFAULT_BOOT_GRACE)
    }

    pub fn for_boot_elapsed(boot_elapsed: Option<Duration>, boot_grace: Duration) -> Self {
        let boot_decision_active = boot_elapsed.is_some_and(|elapsed| elapsed <= boot_grace);
        let last_reason = if boot_decision_active {
            "Waiting for internet during boot grace period".into()
        } else if boot_elapsed.is_some() {
            "Device was not recently booted; AP boot automation skipped".into()
        } else {
            "Device uptime unavailable; AP boot automation skipped".into()
        };

        Self {
            persistent_enabled: false,
            boot_decision_resolved: !boot_decision_active,
            last_hotspot_enabled: None,
            last_reason,
        }
    }

    pub fn last_reason(&self) -> &str {
        &self.last_reason
    }

    pub fn persistent_enabled(&self) -> bool {
        self.persistent_enabled
    }

    pub fn resolved(&self) -> bool {
        self.boot_decision_resolved
    }

    pub fn set_persistent_enabled(&mut self, value: bool) {
        if self.persistent_enabled == value {
            return;
        }
        self.persistent_enabled = value;
        self.boot_decision_resolved = true;
        self.last_reason = if value {
            "Persistent AP enabled by user".into()
        } else {
            "Persistent AP disabled by user".into()
        };
    }

    pub fn step(&mut self, input: SupervisorInput) -> SupervisorDecision {
        if self.boot_decision_resolved {
            return SupervisorDecision {
                action: None,
                automation_paused: false,
                persistent_enabled: self.persistent_enabled,
                resolved: true,
                last_reason: self.last_reason.clone(),
            };
        }

        if self.persistent_enabled {
            self.boot_decision_resolved = true;
            self.last_reason = "Persistent AP enabled by user".into();
            return SupervisorDecision {
                action: None,
                automation_paused: false,
                persistent_enabled: true,
                resolved: true,
                last_reason: self.last_reason.clone(),
            };
        }

        if input.test_in_progress || input.save_in_progress {
            self.last_reason = "AP automation paused during connection transition".into();
            return SupervisorDecision {
                action: None,
                automation_paused: true,
                persistent_enabled: self.persistent_enabled,
                resolved: false,
                last_reason: self.last_reason.clone(),
            };
        }

        if self.last_hotspot_enabled == Some(true) && !input.hotspot_enabled {
            self.boot_decision_resolved = true;
            self.last_reason =
                "Setup AP was disabled during boot; leaving setup AP unchanged".into();
            return SupervisorDecision {
                action: None,
                automation_paused: false,
                persistent_enabled: self.persistent_enabled,
                resolved: true,
                last_reason: self.last_reason.clone(),
            };
        }

        self.last_hotspot_enabled = Some(input.hotspot_enabled);

        let mut action = None;
        if input.uplink_healthy {
            self.boot_decision_resolved = true;
            self.last_reason = "Internet arrived during boot; leaving setup AP unchanged".into();
        } else if input.boot_grace_elapsed {
            self.boot_decision_resolved = true;
            if input.hotspot_enabled {
                self.last_reason = "Setup AP already enabled after boot grace period".into();
            } else {
                self.last_reason = "No internet after boot grace period; enabling setup AP".into();
                action = Some(SupervisorAction::Enable);
            }
        } else {
            self.last_reason = "Waiting for internet during boot grace period".into();
        }

        SupervisorDecision {
            action,
            automation_paused: false,
            persistent_enabled: self.persistent_enabled,
            resolved: self.boot_decision_resolved,
            last_reason: self.last_reason.clone(),
        }
    }

    pub fn note_action_error(&mut self, action: SupervisorAction, err: &str) {
        self.boot_decision_resolved = false;
        self.last_reason = match action {
            SupervisorAction::Enable => format!("Failed to enable AP: {err}"),
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn waits_during_boot_grace_without_internet() {
        let mut machine = ApSupervisorMachine::for_boot_elapsed(
            Some(Duration::from_secs(10)),
            DEFAULT_BOOT_GRACE,
        );
        let decision = machine.step(SupervisorInput {
            test_in_progress: false,
            save_in_progress: false,
            uplink_healthy: false,
            hotspot_enabled: false,
            boot_grace_elapsed: false,
        });

        assert_eq!(decision.action, None);
        assert!(!decision.resolved);
    }

    #[test]
    fn internet_during_boot_grace_resolves_without_action() {
        let mut machine = ApSupervisorMachine::for_boot_elapsed(
            Some(Duration::from_secs(10)),
            DEFAULT_BOOT_GRACE,
        );
        let decision = machine.step(SupervisorInput {
            test_in_progress: false,
            save_in_progress: false,
            uplink_healthy: true,
            hotspot_enabled: false,
            boot_grace_elapsed: false,
        });

        assert_eq!(decision.action, None);
        assert!(decision.resolved);

        let later = machine.step(SupervisorInput {
            test_in_progress: false,
            save_in_progress: false,
            uplink_healthy: false,
            hotspot_enabled: false,
            boot_grace_elapsed: true,
        });
        assert_eq!(later.action, None);
        assert!(later.resolved);
    }

    #[test]
    fn no_internet_after_boot_grace_enables_once() {
        let mut machine = ApSupervisorMachine::for_boot_elapsed(
            Some(Duration::from_secs(10)),
            DEFAULT_BOOT_GRACE,
        );
        let decision = machine.step(SupervisorInput {
            test_in_progress: false,
            save_in_progress: false,
            uplink_healthy: false,
            hotspot_enabled: false,
            boot_grace_elapsed: true,
        });
        assert_eq!(decision.action, Some(SupervisorAction::Enable));
        assert!(decision.resolved);

        let later = machine.step(SupervisorInput {
            test_in_progress: false,
            save_in_progress: false,
            uplink_healthy: false,
            hotspot_enabled: false,
            boot_grace_elapsed: true,
        });
        assert_eq!(later.action, None);
        assert!(later.resolved);
    }

    #[test]
    fn existing_hotspot_after_boot_grace_resolves_without_action() {
        let mut machine = ApSupervisorMachine::for_boot_elapsed(
            Some(Duration::from_secs(10)),
            DEFAULT_BOOT_GRACE,
        );
        let decision = machine.step(SupervisorInput {
            test_in_progress: false,
            save_in_progress: false,
            uplink_healthy: false,
            hotspot_enabled: true,
            boot_grace_elapsed: true,
        });

        assert_eq!(decision.action, None);
        assert!(decision.resolved);
    }

    #[test]
    fn service_start_after_boot_grace_skips_automation() {
        let mut machine = ApSupervisorMachine::for_boot_elapsed(
            Some(DEFAULT_BOOT_GRACE + Duration::from_secs(1)),
            DEFAULT_BOOT_GRACE,
        );
        let decision = machine.step(SupervisorInput {
            test_in_progress: false,
            save_in_progress: false,
            uplink_healthy: false,
            hotspot_enabled: false,
            boot_grace_elapsed: true,
        });

        assert_eq!(decision.action, None);
        assert!(decision.resolved);
    }

    #[test]
    fn missing_uptime_skips_automation() {
        let mut machine = ApSupervisorMachine::for_boot_elapsed(None, DEFAULT_BOOT_GRACE);
        let decision = machine.step(SupervisorInput {
            test_in_progress: false,
            save_in_progress: false,
            uplink_healthy: false,
            hotspot_enabled: false,
            boot_grace_elapsed: true,
        });

        assert_eq!(decision.action, None);
        assert!(decision.resolved);
    }

    #[test]
    fn paused_transition_never_resolves_or_emits_action() {
        let mut machine = ApSupervisorMachine::for_boot_elapsed(
            Some(Duration::from_secs(10)),
            DEFAULT_BOOT_GRACE,
        );
        let decision = machine.step(SupervisorInput {
            test_in_progress: true,
            save_in_progress: false,
            uplink_healthy: false,
            hotspot_enabled: false,
            boot_grace_elapsed: true,
        });

        assert_eq!(decision.action, None);
        assert!(decision.automation_paused);
        assert!(!decision.resolved);
    }

    #[test]
    fn persistent_mode_resolves_boot_automation_without_action() {
        let mut machine = ApSupervisorMachine::for_boot_elapsed(
            Some(Duration::from_secs(10)),
            DEFAULT_BOOT_GRACE,
        );
        machine.set_persistent_enabled(true);

        let decision = machine.step(SupervisorInput {
            test_in_progress: false,
            save_in_progress: false,
            uplink_healthy: false,
            hotspot_enabled: false,
            boot_grace_elapsed: true,
        });

        assert_eq!(decision.action, None);
        assert!(decision.persistent_enabled);
        assert!(decision.resolved);
    }

    #[test]
    fn manual_disable_during_boot_grace_resolves_without_reenable() {
        let mut machine = ApSupervisorMachine::for_boot_elapsed(
            Some(Duration::from_secs(10)),
            DEFAULT_BOOT_GRACE,
        );
        let first = machine.step(SupervisorInput {
            test_in_progress: false,
            save_in_progress: false,
            uplink_healthy: false,
            hotspot_enabled: true,
            boot_grace_elapsed: false,
        });
        assert_eq!(first.action, None);
        assert!(!first.resolved);

        let disabled = machine.step(SupervisorInput {
            test_in_progress: false,
            save_in_progress: false,
            uplink_healthy: false,
            hotspot_enabled: false,
            boot_grace_elapsed: false,
        });
        assert_eq!(disabled.action, None);
        assert!(disabled.resolved);

        let after_grace = machine.step(SupervisorInput {
            test_in_progress: false,
            save_in_progress: false,
            uplink_healthy: false,
            hotspot_enabled: false,
            boot_grace_elapsed: true,
        });
        assert_eq!(after_grace.action, None);
        assert!(after_grace.resolved);
    }
}
