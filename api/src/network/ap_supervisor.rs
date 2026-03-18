#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SupervisorAction {
    Enable,
    Disable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SupervisorMode {
    Auto,
    ForceOn,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SupervisorInput {
    pub test_in_progress: bool,
    pub save_in_progress: bool,
    pub ethernet_connected: bool,
    pub uplink_healthy: bool,
    pub hotspot_enabled: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SupervisorDecision {
    pub action: Option<SupervisorAction>,
    pub automation_paused: bool,
    pub persistent_enabled: bool,
    pub last_reason: String,
}

#[derive(Debug, Clone)]
pub struct ApSupervisorMachine {
    persistent_enabled: bool,
    good_streak: u8,
    bad_streak: u8,
    cooldown_remaining: u8,
    last_reason: String,
}

const ENABLE_BAD_STREAK: u8 = 2;
const DISABLE_GOOD_STREAK: u8 = 3;
const ACTION_COOLDOWN_TICKS: u8 = 3;

impl ApSupervisorMachine {
    pub fn new() -> Self {
        Self {
            persistent_enabled: false,
            good_streak: 0,
            bad_streak: 0,
            cooldown_remaining: 0,
            last_reason: "AP supervisor initializing".into(),
        }
    }

    pub fn persistent_enabled(&self) -> bool {
        self.persistent_enabled
    }

    pub fn last_reason(&self) -> &str {
        &self.last_reason
    }

    pub fn set_persistent_enabled(&mut self, value: bool) {
        self.persistent_enabled = value;
        self.good_streak = 0;
        self.bad_streak = 0;
        self.last_reason = if value {
            "Persistent AP enabled by user".into()
        } else {
            "Persistent AP disabled by user".into()
        };
    }

    pub fn step(&mut self, input: SupervisorInput) -> SupervisorDecision {
        if input.test_in_progress || input.save_in_progress {
            self.good_streak = 0;
            self.bad_streak = 0;
            self.last_reason = "AP automation paused during connection transition".into();
            return SupervisorDecision {
                action: None,
                automation_paused: true,
                persistent_enabled: self.persistent_enabled,
                last_reason: self.last_reason.clone(),
            };
        }

        if self.cooldown_remaining > 0 {
            self.cooldown_remaining -= 1;
        }

        let mode = if self.persistent_enabled && input.ethernet_connected {
            SupervisorMode::ForceOn
        } else {
            SupervisorMode::Auto
        };

        let mut action = None;
        match mode {
            SupervisorMode::ForceOn => {
                self.good_streak = 0;
                self.bad_streak = 0;
                self.last_reason = "Persistent AP enabled in Ethernet mode".into();
                if !input.hotspot_enabled && self.cooldown_remaining == 0 {
                    action = Some(SupervisorAction::Enable);
                }
            }
            SupervisorMode::Auto => {
                if input.uplink_healthy {
                    self.good_streak = self.good_streak.saturating_add(1);
                    self.bad_streak = 0;
                    self.last_reason = "Healthy uplink detected".into();
                    if input.hotspot_enabled
                        && self.good_streak >= DISABLE_GOOD_STREAK
                        && self.cooldown_remaining == 0
                    {
                        action = Some(SupervisorAction::Disable);
                    }
                } else {
                    self.bad_streak = self.bad_streak.saturating_add(1);
                    self.good_streak = 0;
                    self.last_reason = "No healthy uplink detected".into();
                    if !input.hotspot_enabled
                        && self.bad_streak >= ENABLE_BAD_STREAK
                        && self.cooldown_remaining == 0
                    {
                        action = Some(SupervisorAction::Enable);
                    }
                }
            }
        }

        if action.is_some() {
            self.cooldown_remaining = ACTION_COOLDOWN_TICKS;
            self.good_streak = 0;
            self.bad_streak = 0;
        }

        SupervisorDecision {
            action,
            automation_paused: false,
            persistent_enabled: self.persistent_enabled,
            last_reason: self.last_reason.clone(),
        }
    }

    pub fn note_action_error(&mut self, action: SupervisorAction, err: &str) {
        self.last_reason = match action {
            SupervisorAction::Enable => format!("Failed to enable AP: {err}"),
            SupervisorAction::Disable => format!("Failed to disable AP: {err}"),
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    #[test]
    fn persistent_mode_enables_without_disable() {
        let mut machine = ApSupervisorMachine::new();
        machine.set_persistent_enabled(true);

        let first = machine.step(SupervisorInput {
            test_in_progress: false,
            save_in_progress: false,
            ethernet_connected: true,
            uplink_healthy: true,
            hotspot_enabled: false,
        });
        assert_eq!(first.action, Some(SupervisorAction::Enable));

        let second = machine.step(SupervisorInput {
            test_in_progress: false,
            save_in_progress: false,
            ethernet_connected: true,
            uplink_healthy: true,
            hotspot_enabled: true,
        });
        assert_ne!(second.action, Some(SupervisorAction::Disable));
    }

    proptest! {
        #[test]
        fn paused_transition_never_emits_action(
            hotspot_enabled in any::<bool>(),
            ethernet_connected in any::<bool>(),
            uplink_healthy in any::<bool>(),
        ) {
            let mut machine = ApSupervisorMachine::new();
            let decision = machine.step(SupervisorInput {
                test_in_progress: true,
                save_in_progress: false,
                ethernet_connected,
                uplink_healthy,
                hotspot_enabled,
            });
            prop_assert_eq!(decision.action, None);
            prop_assert!(decision.automation_paused);
        }
    }

    proptest! {
        #[test]
        fn force_on_over_ethernet_never_disables(
            inputs in prop::collection::vec((any::<bool>(), any::<bool>(), any::<bool>()), 1..120)
        ) {
            let mut machine = ApSupervisorMachine::new();
            machine.set_persistent_enabled(true);
            for (hotspot_enabled, test_in_progress, save_in_progress) in inputs {
                let decision = machine.step(SupervisorInput {
                    test_in_progress,
                    save_in_progress,
                    ethernet_connected: true,
                    uplink_healthy: true,
                    hotspot_enabled,
                });
                if !(test_in_progress || save_in_progress) {
                    prop_assert_ne!(decision.action, Some(SupervisorAction::Disable));
                }
            }
        }
    }
}
