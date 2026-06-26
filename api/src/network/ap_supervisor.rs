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
    force_on_enable_requested: bool,
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
            force_on_enable_requested: false,
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
        if self.persistent_enabled == value {
            return;
        }
        self.persistent_enabled = value;
        self.good_streak = 0;
        self.bad_streak = 0;
        self.force_on_enable_requested = false;
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
                if input.hotspot_enabled {
                    self.force_on_enable_requested = false;
                } else if self.cooldown_remaining == 0 && !self.force_on_enable_requested {
                    action = Some(SupervisorAction::Enable);
                    self.force_on_enable_requested = true;
                }
            }
            SupervisorMode::Auto => {
                self.force_on_enable_requested = false;
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
    use proptest::strategy::Strategy;

    #[derive(Debug, Clone)]
    enum Event {
        SetPersistent(bool),
        Tick {
            test_in_progress: bool,
            save_in_progress: bool,
            ethernet_connected: bool,
            uplink_healthy: bool,
        },
    }

    fn events_strategy() -> impl Strategy<Value = Vec<Event>> {
        prop::collection::vec(
            prop_oneof![
                1 => any::<bool>().prop_map(Event::SetPersistent),
                9 => (any::<bool>(), any::<bool>(), any::<bool>(), any::<bool>())
                    .prop_map(|(test_in_progress, save_in_progress, ethernet_connected, uplink_healthy)| Event::Tick {
                        test_in_progress,
                        save_in_progress,
                        ethernet_connected,
                        uplink_healthy,
                    }),
            ],
            1..300,
        )
    }

    fn apply_event(
        machine: &mut ApSupervisorMachine,
        hotspot_enabled: &mut bool,
        event: &Event,
    ) -> Option<SupervisorAction> {
        match event {
            Event::SetPersistent(value) => {
                machine.set_persistent_enabled(*value);
                None
            }
            Event::Tick {
                test_in_progress,
                save_in_progress,
                ethernet_connected,
                uplink_healthy,
            } => {
                let decision = machine.step(SupervisorInput {
                    test_in_progress: *test_in_progress,
                    save_in_progress: *save_in_progress,
                    ethernet_connected: *ethernet_connected,
                    uplink_healthy: *uplink_healthy,
                    hotspot_enabled: *hotspot_enabled,
                });
                if let Some(action) = decision.action {
                    *hotspot_enabled = matches!(action, SupervisorAction::Enable);
                }
                decision.action
            }
        }
    }

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

    #[test]
    fn force_on_never_repeats_enable_without_hotspot_state_change() {
        let mut machine = ApSupervisorMachine::new();
        machine.set_persistent_enabled(true);

        let mut enable_count = 0usize;
        for _ in 0..30 {
            let action = machine
                .step(SupervisorInput {
                    test_in_progress: false,
                    save_in_progress: false,
                    ethernet_connected: true,
                    uplink_healthy: false,
                    hotspot_enabled: false,
                })
                .action;
            if action == Some(SupervisorAction::Enable) {
                enable_count += 1;
            }
        }

        assert_eq!(enable_count, 1);
    }

    #[test]
    fn reapplying_same_persistent_setting_does_not_trigger_second_enable() {
        let mut machine = ApSupervisorMachine::new();
        machine.set_persistent_enabled(true);

        let first = machine
            .step(SupervisorInput {
                test_in_progress: false,
                save_in_progress: false,
                ethernet_connected: true,
                uplink_healthy: false,
                hotspot_enabled: false,
            })
            .action;
        assert_eq!(first, Some(SupervisorAction::Enable));

        machine.set_persistent_enabled(true);

        let second = machine
            .step(SupervisorInput {
                test_in_progress: false,
                save_in_progress: false,
                ethernet_connected: true,
                uplink_healthy: false,
                hotspot_enabled: false,
            })
            .action;
        assert_eq!(second, None);
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
        fn force_on_over_ethernet_never_disables(events in events_strategy()) {
            let mut machine = ApSupervisorMachine::new();
            let mut hotspot_enabled = false;
            for event in &events {
                if let Event::Tick {
                    test_in_progress,
                    save_in_progress,
                    ethernet_connected,
                    uplink_healthy: _,
                } = event
                {
                    let action = apply_event(&mut machine, &mut hotspot_enabled, event);
                    if machine.persistent_enabled()
                        && *ethernet_connected
                        && !(*test_in_progress || *save_in_progress)
                    {
                        prop_assert_ne!(action, Some(SupervisorAction::Disable));
                    }
                } else {
                    let _ = apply_event(&mut machine, &mut hotspot_enabled, event);
                }
            }
        }
    }

    proptest! {
        #[test]
        fn action_spacing_respects_cooldown(events in events_strategy()) {
            let mut machine = ApSupervisorMachine::new();
            let mut hotspot_enabled = false;
            let mut ticks_since_action: Option<usize> = None;
            for event in &events {
                if let Event::Tick { test_in_progress, save_in_progress, .. } = event
                    && !(*test_in_progress || *save_in_progress)
                    && let Some(value) = ticks_since_action.as_mut()
                {
                    *value += 1;
                }
                let action = apply_event(&mut machine, &mut hotspot_enabled, event);
                if action.is_some() {
                    if let Some(elapsed) = ticks_since_action {
                        prop_assert!(elapsed >= ACTION_COOLDOWN_TICKS as usize);
                    }
                    ticks_since_action = Some(0);
                }
            }
        }
    }

    proptest! {
        #[test]
        fn auto_mode_actions_require_expected_history(events in events_strategy()) {
            let mut machine = ApSupervisorMachine::new();
            let mut hotspot_enabled = false;
            let mut unhealthy_streak = 0u8;
            let mut healthy_streak = 0u8;
            let mut persistent_enabled = false;

            for event in &events {
                match event {
                    Event::SetPersistent(value) => {
                        let changed = *value != persistent_enabled;
                        machine.set_persistent_enabled(*value);
                        if changed {
                            unhealthy_streak = 0;
                            healthy_streak = 0;
                            persistent_enabled = *value;
                        }
                    }
                    Event::Tick {
                        test_in_progress,
                        save_in_progress,
                        ethernet_connected,
                        uplink_healthy,
                    } => {
                        let paused = *test_in_progress || *save_in_progress;
                        let force_on = persistent_enabled && *ethernet_connected;
                        let in_auto_mode = !force_on;

                        if paused || !in_auto_mode {
                            unhealthy_streak = 0;
                            healthy_streak = 0;
                        } else if *uplink_healthy {
                            healthy_streak = healthy_streak.saturating_add(1);
                            unhealthy_streak = 0;
                        } else {
                            unhealthy_streak = unhealthy_streak.saturating_add(1);
                            healthy_streak = 0;
                        }

                        let action = apply_event(&mut machine, &mut hotspot_enabled, event);

                        if in_auto_mode {
                            if action == Some(SupervisorAction::Enable) {
                                prop_assert!(unhealthy_streak >= ENABLE_BAD_STREAK);
                                unhealthy_streak = 0;
                                healthy_streak = 0;
                            }
                            if action == Some(SupervisorAction::Disable) {
                                prop_assert!(healthy_streak >= DISABLE_GOOD_STREAK);
                                unhealthy_streak = 0;
                                healthy_streak = 0;
                            }
                        }
                    }
                }
            }
        }
    }

    proptest! {
        #[test]
        fn stable_input_does_not_oscillate(
            persistent in any::<bool>(),
            ethernet_connected in any::<bool>(),
            uplink_healthy in any::<bool>(),
            initial_hotspot_enabled in any::<bool>(),
            ticks in 1usize..120,
        ) {
            let mut machine = ApSupervisorMachine::new();
            machine.set_persistent_enabled(persistent);
            let mut hotspot_enabled = initial_hotspot_enabled;
            let mut action_count = 0usize;
            for _ in 0..ticks {
                let action = machine.step(SupervisorInput {
                    test_in_progress: false,
                    save_in_progress: false,
                    ethernet_connected,
                    uplink_healthy,
                    hotspot_enabled,
                }).action;
                if let Some(a) = action {
                    action_count += 1;
                    hotspot_enabled = matches!(a, SupervisorAction::Enable);
                }
            }
            prop_assert!(action_count <= 1);
        }
    }

    #[test]
    fn force_on_emits_enable_on_first_eligible_tick() {
        let mut machine = ApSupervisorMachine::new();
        machine.set_persistent_enabled(true);

        let decision = machine.step(SupervisorInput {
            test_in_progress: false,
            save_in_progress: false,
            ethernet_connected: true,
            uplink_healthy: false,
            hotspot_enabled: false,
        });

        assert_eq!(decision.action, Some(SupervisorAction::Enable));
    }
}
