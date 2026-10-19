use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SupervisorAction {
    Enable,
    EnableUnconfigured,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SupervisorInput {
    pub test_in_progress: bool,
    pub save_in_progress: bool,
    pub uplink_healthy: bool,
    pub hotspot_enabled: bool,
    pub boot_grace_elapsed: bool,
    pub has_client_configuration: Option<bool>,
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
    unconfigured_handled: bool,
    unconfigured_observations: u8,
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
            unconfigured_handled: false,
            unconfigured_observations: 0,
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

    pub fn resolve(&mut self, reason: &str) {
        self.boot_decision_resolved = true;
        self.last_reason = reason.into();
    }

    pub fn set_persistent_enabled(&mut self, value: bool) {
        self.persistent_enabled = value;
        self.unconfigured_handled = true;
        self.unconfigured_observations = 0;
        self.boot_decision_resolved = true;
        self.last_reason = if value {
            "Persistent AP enabled by user".into()
        } else {
            "Persistent AP disabled by user".into()
        };
    }

    pub fn step(&mut self, input: SupervisorInput) -> SupervisorDecision {
        if input.test_in_progress || input.save_in_progress {
            self.unconfigured_observations = 0;
            return SupervisorDecision {
                action: None,
                automation_paused: true,
                persistent_enabled: self.persistent_enabled,
                resolved: self.boot_decision_resolved,
                last_reason: "AP automation paused during connection transition".into(),
            };
        }

        match input.has_client_configuration {
            Some(true) => {
                self.unconfigured_handled = false;
                self.unconfigured_observations = 0;
            }
            Some(false) if input.hotspot_enabled => {
                self.unconfigured_handled = true;
                self.unconfigured_observations = 0;
            }
            Some(false) => {
                self.unconfigured_observations = self.unconfigured_observations.saturating_add(1);
            }
            None => self.unconfigured_observations = 0,
        }

        if self.boot_decision_resolved {
            let action = if !self.persistent_enabled
                && input.boot_grace_elapsed
                && !input.hotspot_enabled
                && !self.unconfigured_handled
                && self.unconfigured_observations >= 2
            {
                self.unconfigured_handled = true;
                self.last_reason =
                    "No saved Wi-Fi client or Ethernet profiles; enabling setup AP".into();
                Some(SupervisorAction::EnableUnconfigured)
            } else {
                None
            };
            return SupervisorDecision {
                action,
                automation_paused: false,
                persistent_enabled: self.persistent_enabled,
                resolved: true,
                last_reason: self.last_reason.clone(),
            };
        }

        if self.last_hotspot_enabled == Some(true) && !input.hotspot_enabled {
            self.boot_decision_resolved = true;
            self.unconfigured_handled = true;
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
        match action {
            SupervisorAction::Enable => self.boot_decision_resolved = false,
            SupervisorAction::EnableUnconfigured => {
                self.unconfigured_handled = false;
                self.unconfigured_observations = 0;
            }
        }
        self.last_reason = format!("Failed to enable AP: {err}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn completed_decision_survives_service_restart_during_boot_grace() {
        let mut machine = ApSupervisorMachine::new();
        machine.resolve("AP boot decision already made for this boot");
        let decision = machine.step(SupervisorInput {
            test_in_progress: false,
            save_in_progress: false,
            uplink_healthy: false,
            hotspot_enabled: false,
            boot_grace_elapsed: true,
            has_client_configuration: Some(true),
        });
        assert!(decision.resolved);
        assert_eq!(decision.action, None);
    }

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
            has_client_configuration: Some(true),
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
            has_client_configuration: Some(true),
        });

        assert_eq!(decision.action, None);
        assert!(decision.resolved);

        let later = machine.step(SupervisorInput {
            test_in_progress: false,
            save_in_progress: false,
            uplink_healthy: false,
            hotspot_enabled: false,
            boot_grace_elapsed: true,
            has_client_configuration: Some(true),
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
            has_client_configuration: Some(true),
        });
        assert_eq!(decision.action, Some(SupervisorAction::Enable));
        assert!(decision.resolved);

        let later = machine.step(SupervisorInput {
            test_in_progress: false,
            save_in_progress: false,
            uplink_healthy: false,
            hotspot_enabled: false,
            boot_grace_elapsed: true,
            has_client_configuration: Some(true),
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
            has_client_configuration: Some(true),
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
            has_client_configuration: Some(true),
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
            has_client_configuration: Some(true),
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
            has_client_configuration: Some(true),
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
            has_client_configuration: Some(true),
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
            has_client_configuration: Some(true),
        });
        assert_eq!(first.action, None);
        assert!(!first.resolved);

        let disabled = machine.step(SupervisorInput {
            test_in_progress: false,
            save_in_progress: false,
            uplink_healthy: false,
            hotspot_enabled: false,
            boot_grace_elapsed: false,
            has_client_configuration: Some(true),
        });
        assert_eq!(disabled.action, None);
        assert!(disabled.resolved);

        let after_grace = machine.step(SupervisorInput {
            test_in_progress: false,
            save_in_progress: false,
            uplink_healthy: false,
            hotspot_enabled: false,
            boot_grace_elapsed: true,
            has_client_configuration: Some(true),
        });
        assert_eq!(after_grace.action, None);
        assert!(after_grace.resolved);
    }

    fn late_unconfigured() -> SupervisorInput {
        SupervisorInput {
            test_in_progress: false,
            save_in_progress: false,
            uplink_healthy: false,
            hotspot_enabled: false,
            boot_grace_elapsed: true,
            has_client_configuration: Some(false),
        }
    }

    fn after_boot() -> ApSupervisorMachine {
        ApSupervisorMachine::for_boot_elapsed(Some(Duration::from_secs(100)), DEFAULT_BOOT_GRACE)
    }

    #[test]
    fn deleting_last_client_profile_after_boot_enables_once_after_confirmation() {
        let mut machine = after_boot();
        let input = late_unconfigured();
        machine.step(SupervisorInput {
            has_client_configuration: Some(true),
            ..input
        });
        assert_eq!(machine.step(input).action, None);
        assert_eq!(
            machine.step(input).action,
            Some(SupervisorAction::EnableUnconfigured)
        );
        for _ in 0..4 {
            assert_eq!(machine.step(input).action, None);
        }
        machine.step(SupervisorInput {
            has_client_configuration: Some(true),
            ..input
        });
        assert_eq!(machine.step(input).action, None);
        assert_eq!(
            machine.step(input).action,
            Some(SupervisorAction::EnableUnconfigured)
        );
    }

    #[test]
    fn configured_outage_never_reopens_boot_decision() {
        let mut machine = after_boot();
        for _ in 0..200 {
            let decision = machine.step(SupervisorInput {
                has_client_configuration: Some(true),
                ..late_unconfigured()
            });
            assert_eq!(decision.action, None);
            assert!(decision.resolved);
        }
    }

    #[test]
    fn unconfigured_device_still_waits_for_boot_grace() {
        let mut machine = ApSupervisorMachine::new();
        for _ in 0..4 {
            assert_eq!(
                machine
                    .step(SupervisorInput {
                        boot_grace_elapsed: false,
                        ..late_unconfigured()
                    })
                    .action,
                None
            );
        }
        assert_eq!(
            machine.step(late_unconfigured()).action,
            Some(SupervisorAction::Enable)
        );
    }

    #[test]
    fn unknown_profile_state_resets_confirmation() {
        let mut machine = after_boot();
        let input = late_unconfigured();
        assert_eq!(machine.step(input).action, None);
        for _ in 0..4 {
            assert_eq!(
                machine
                    .step(SupervisorInput {
                        has_client_configuration: None,
                        ..input
                    })
                    .action,
                None
            );
        }
        assert_eq!(machine.step(input).action, None);
        assert_eq!(
            machine.step(input).action,
            Some(SupervisorAction::EnableUnconfigured)
        );
    }

    #[test]
    fn tests_and_saves_pause_fallback_after_boot_and_reset_confirmation() {
        for (test_in_progress, save_in_progress) in [(true, false), (false, true)] {
            let mut machine = after_boot();
            let input = late_unconfigured();
            machine.step(input);
            for _ in 0..4 {
                let decision = machine.step(SupervisorInput {
                    test_in_progress,
                    save_in_progress,
                    ..input
                });
                assert_eq!(decision.action, None);
                assert!(decision.automation_paused);
            }
            assert_eq!(machine.step(input).action, None);
            assert_eq!(
                machine.step(input).action,
                Some(SupervisorAction::EnableUnconfigured)
            );
        }
    }

    #[test]
    fn existing_ap_and_explicit_manual_off_prevent_repeated_reenable() {
        let input = late_unconfigured();
        let mut machine = after_boot();
        machine.step(SupervisorInput {
            hotspot_enabled: true,
            ..input
        });
        for _ in 0..4 {
            assert_eq!(machine.step(input).action, None);
        }
        let mut machine = after_boot();
        machine.set_persistent_enabled(false);
        for _ in 0..4 {
            assert_eq!(machine.step(input).action, None);
        }
        machine.set_persistent_enabled(true);
        for _ in 0..4 {
            assert_eq!(machine.step(input).action, None);
        }
    }

    #[test]
    fn failed_fallback_retries_without_reviving_boot_outage_behavior() {
        let mut machine = after_boot();
        let input = late_unconfigured();
        machine.step(input);
        let action = machine.step(input).action.unwrap();
        machine.note_action_error(action, "activation failed");
        assert!(machine.resolved());
        assert_eq!(machine.step(input).action, None);
        assert_eq!(
            machine.step(input).action,
            Some(SupervisorAction::EnableUnconfigured)
        );
        machine.note_action_error(action, "activation failed");
        for _ in 0..4 {
            assert_eq!(
                machine
                    .step(SupervisorInput {
                        has_client_configuration: Some(true),
                        ..input
                    })
                    .action,
                None
            );
        }
    }
}
