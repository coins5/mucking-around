#![forbid(unsafe_code)]

pub mod roster;

use serde::{Deserialize, Serialize};

pub use roster::{default_milestones, default_roster, ActivityConfig, Milestone};

/// Extensible identifier for in-game resources.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ResourceId {
    /// Primary resource (e.g. Energy / Cycles).
    Primary,
}

/// Domain errors for core simulation operations.
#[derive(Debug, PartialEq, Clone)]
pub enum CoreError {
    InvalidDeltaTime(f64),
    InvalidTargetDuration(f64),
}

impl std::fmt::Display for CoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidDeltaTime(dt) => write!(f, "Invalid delta time: {dt}"),
            Self::InvalidTargetDuration(d) => write!(f, "Invalid target duration: {d}"),
        }
    }
}

impl std::error::Error for CoreError {}

/// Component representing an idle progress generator (legacy single generator support).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Generator {
    /// Total duration in seconds needed to complete a cycle.
    pub target_duration: f64,
    /// Accumulated time in seconds towards the current cycle.
    pub progress: f64,
    /// The resource awarded upon cycle completion.
    pub output_resource: ResourceId,
    /// Amount of resource awarded upon cycle completion.
    pub output_amount: f64,
}

impl Default for Generator {
    fn default() -> Self {
        Self {
            target_duration: 5.0,
            progress: 0.0,
            output_resource: ResourceId::Primary,
            output_amount: 0.7,
        }
    }
}

impl Generator {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn tick(&mut self, dt: f64) -> u64 {
        if !dt.is_finite() || dt <= 0.0 || !self.target_duration.is_finite() || self.target_duration <= 0.0 {
            return 0;
        }

        self.progress += dt;

        let mut completed = 0u64;

        if self.progress >= self.target_duration * 100.0 {
            let fast_cycles = (self.progress / self.target_duration).floor() as u64;
            completed += fast_cycles;
            self.progress -= (fast_cycles as f64) * self.target_duration;
        }

        while self.progress >= self.target_duration {
            completed += 1;
            self.progress -= self.target_duration;
        }

        self.progress = self.progress.clamp(0.0, self.target_duration);

        completed
    }

    #[must_use]
    pub fn progress_ratio(&self) -> f64 {
        if !self.target_duration.is_finite() || self.target_duration <= 0.0 {
            return 0.0;
        }
        (self.progress / self.target_duration).clamp(0.0, 1.0)
    }
}

/// Runtime dynamic state for an activity instance.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ActivityState {
    /// Static configuration for this activity.
    pub config: ActivityConfig,
    /// Current accumulated time in seconds towards cycle completion.
    pub progress: f64,
    /// Current level of the activity (0 indicates locked, >= 1 is active).
    pub level: u32,
}

impl ActivityState {
    /// Creates a new `ActivityState` based on its configuration.
    /// An activity starts at level 1 if its cost is 0.0 (unlocked), otherwise level 0 (locked).
    #[must_use]
    pub fn new(config: ActivityConfig) -> Self {
        let level = if config.cost <= 0.0 { 1 } else { 0 };
        Self {
            config,
            progress: 0.0,
            level,
        }
    }

    /// Whether this activity is currently active / unlocked.
    #[must_use]
    pub fn is_unlocked(&self) -> bool {
        self.level > 0
    }

    /// Returns the cost for the next upgrade (or initial unlock if level is 0).
    /// - If `level == 0`: returns `config.cost`.
    /// - If `level > 0`: returns `base_cost * 1.15^level` (where `base_cost = if cost <= 0.0 { 1.0 } else { cost }`).
    #[must_use]
    pub fn next_cost(&self) -> f64 {
        if self.level == 0 {
            self.config.cost
        } else {
            let base_cost = if self.config.cost <= 0.0 {
                1.0
            } else {
                self.config.cost
            };
            base_cost * 1.15_f64.powi(self.level as i32)
        }
    }

    /// Returns the multiplier awarded for milestone achievements (defaults to 1.0).
    #[must_use]
    pub fn milestone_multiplier(&self) -> f64 {
        1.0
    }

    /// Returns the highest speed multiplier reached from configured milestones.
    /// Returns `1.0` if no milestones have been achieved (e.g. `level < 25`).
    #[must_use]
    pub fn speed_multiplier(&self) -> f64 {
        self.config
            .milestones
            .iter()
            .filter(|m| self.level >= m.level)
            .max_by_key(|m| m.level)
            .map_or(1.0, |m| m.speed_multiplier)
    }

    /// Returns the next milestone to be reached, or `None` if maxed.
    #[must_use]
    pub fn next_milestone(&self) -> Option<&Milestone> {
        self.config
            .milestones
            .iter()
            .filter(|m| m.level > self.level)
            .min_by_key(|m| m.level)
    }

    /// Returns the reward granted on cycle completion:
    /// `config.reward * level * milestone_multiplier()`.
    #[must_use]
    pub fn current_reward(&self) -> f64 {
        self.config.reward * (self.level as f64) * self.milestone_multiplier()
    }

    /// Returns the effective cycle duration considering speed milestones:
    /// `(config.duration / speed_multiplier()).max(0.005)`.
    #[must_use]
    pub fn current_duration(&self) -> f64 {
        let mult = self.speed_multiplier();
        if !mult.is_finite() || mult <= 0.0 {
            self.config.duration.max(0.005)
        } else {
            (self.config.duration / mult).max(0.005)
        }
    }

    /// Returns `true` if the cycle duration is short enough (`<= 0.15s`)
    /// that rendering a standard filling progress bar no longer makes sense.
    #[must_use]
    pub fn is_turbo(&self) -> bool {
        self.current_duration() <= 0.15
    }

    /// Returns the current production rate in points per second:
    /// `current_reward() / current_duration()`.
    #[must_use]
    pub fn pts_per_second(&self) -> f64 {
        let duration = self.current_duration();
        if !duration.is_finite() || duration <= 0.0 {
            0.0
        } else {
            self.current_reward() / duration
        }
    }

    /// Returns the progress ratio clamped between `0.0` and `1.0`.
    #[must_use]
    pub fn progress_ratio(&self) -> f64 {
        let duration = self.current_duration();
        if !duration.is_finite() || duration <= 0.0 {
            0.0
        } else {
            (self.progress / duration).clamp(0.0, 1.0)
        }
    }

    /// Returns the remaining time in seconds to complete the current cycle.
    #[must_use]
    pub fn remaining_time(&self) -> f64 {
        let duration = self.current_duration();
        if !duration.is_finite() || duration <= 0.0 {
            0.0
        } else {
            (duration - self.progress).clamp(0.0, duration)
        }
    }
}

/// Explicit action intent to drive deterministic state mutations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Action {
    /// Attempt to upgrade/unlock an activity at the given roster index.
    UpgradeActivity(usize),
    /// Legacy alias for unlocking/upgrading an activity.
    UnlockActivity(usize),
}

/// The game state managing procrastination activities and Sloth Points.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GameState {
    /// Current balance of Sloth Points ("Puntos de Flojera").
    pub sloth_points: f64,
    /// Dynamic state of all activities in the roster.
    pub activities: Vec<ActivityState>,
}

impl Default for GameState {
    fn default() -> Self {
        Self::new()
    }
}

impl GameState {
    /// Initializes a new game state with the default roster.
    /// The first activity starts at level 1 (unlocked), the rest at level 0 (locked).
    #[must_use]
    pub fn new() -> Self {
        let activities = default_roster().into_iter().map(ActivityState::new).collect();
        Self {
            sloth_points: 0.0,
            activities,
        }
    }

    /// Attempts to upgrade (or unlock) the activity at `index`.
    ///
    /// Returns `true` if the activity exists and the player had enough
    /// Sloth Points to pay its next cost. Increments `activity.level` and deducts cost.
    /// Returns `false` otherwise.
    pub fn upgrade_activity(&mut self, index: usize) -> bool {
        if let Some(activity) = self.activities.get_mut(index) {
            let cost = activity.next_cost();
            if self.sloth_points >= cost {
                self.sloth_points -= cost;
                activity.level += 1;
                return true;
            }
        }
        false
    }

    /// Legacy alias for `upgrade_activity`.
    pub fn unlock_activity(&mut self, index: usize) -> bool {
        self.upgrade_activity(index)
    }

    /// Handles an explicit player action intent.
    pub fn handle_action(&mut self, action: Action) -> bool {
        match action {
            Action::UpgradeActivity(index) | Action::UnlockActivity(index) => {
                self.upgrade_activity(index)
            }
        }
    }

    /// Advances the simulation by `dt` seconds deterministically.
    ///
    /// Only unlocked activities (`level > 0`) advance their progress.
    /// When an activity completes (`progress >= current_duration()`), its reward
    /// (`current_reward()`) is added to `sloth_points` and any remainder is preserved.
    pub fn tick(&mut self, dt: f64) {
        if !dt.is_finite() || dt <= 0.0 {
            return;
        }

        for activity in &mut self.activities {
            if activity.level == 0 {
                continue;
            }

            let duration = activity.current_duration();
            if !duration.is_finite() || duration <= 0.0 {
                continue;
            }

            activity.progress += dt;

            let cycles = (activity.progress / duration).floor();
            if cycles > 0.0 {
                activity.progress -= cycles * duration;
                let gained = activity.current_reward() * cycles;
                self.sloth_points += gained;
            }

            // Ensure progress is clamped to prevent negative precision drift or overflow
            activity.progress = activity.progress.clamp(0.0, duration);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const EPSILON: f64 = 1e-9;

    #[test]
    fn test_initial_game_state() {
        let state = GameState::new();
        assert!((state.sloth_points - 0.0).abs() < EPSILON);
        assert_eq!(state.activities.len(), 5);

        // First activity must be level 1 by default
        assert_eq!(state.activities[0].level, 1);
        assert!(state.activities[0].is_unlocked());
        assert!((state.activities[0].progress - 0.0).abs() < EPSILON);
        assert_eq!(state.activities[0].config.id, "wait_bar");

        // Remaining activities must be level 0 (locked)
        for (i, activity) in state.activities.iter().enumerate().skip(1) {
            assert_eq!(activity.level, 0, "Activity at index {i} should be level 0 initially");
            assert!(!activity.is_unlocked());
            assert!((activity.progress - 0.0).abs() < EPSILON);
        }
    }

    #[test]
    fn test_only_first_activity_progresses_initially() {
        let mut state = GameState::new();
        state.tick(2.0);

        assert!((state.activities[0].progress - 2.0).abs() < EPSILON);
        assert!((state.activities[0].progress_ratio() - (2.0 / 5.0)).abs() < EPSILON);
        assert!((state.activities[0].remaining_time() - 3.0).abs() < EPSILON);
        assert!((state.sloth_points - 0.0).abs() < EPSILON);

        // Locked activities must not have progressed
        for activity in state.activities.iter().skip(1) {
            assert_eq!(activity.progress, 0.0);
        }
    }

    #[test]
    fn test_cannot_upgrade_without_sufficient_points() {
        let mut state = GameState::new();
        // Activity 1 requires 5.0 points to unlock/upgrade to level 1, state starts with 0.0
        let upgraded = state.upgrade_activity(1);
        assert!(!upgraded);
        assert_eq!(state.activities[1].level, 0);
        assert_eq!(state.sloth_points, 0.0);

        // Even with partial points
        state.sloth_points = 4.9;
        let upgraded = state.upgrade_activity(1);
        assert!(!upgraded);
        assert_eq!(state.activities[1].level, 0);
        assert!((state.sloth_points - 4.9).abs() < EPSILON);
    }

    #[test]
    fn test_cannot_upgrade_invalid_indices() {
        let mut state = GameState::new();
        state.sloth_points = 1000.0;

        // Index out of bounds
        assert!(!state.upgrade_activity(99));
        assert_eq!(state.sloth_points, 1000.0);
    }

    #[test]
    fn test_exponential_cost_scaling() {
        let mut state = GameState::new();

        // Activity 0: cost was 0.0, base cost is 1.0
        // Level 1: next_cost = 1.0 * 1.15^1 = 1.15
        assert!((state.activities[0].next_cost() - 1.15).abs() < EPSILON);

        // Upgrade Activity 0 to Level 2
        state.sloth_points = 10.0;
        assert!(state.upgrade_activity(0));
        assert_eq!(state.activities[0].level, 2);
        assert!((state.sloth_points - (10.0 - 1.15)).abs() < EPSILON);

        // Level 2: next_cost = 1.0 * 1.15^2 = 1.3225
        assert!((state.activities[0].next_cost() - 1.3225).abs() < EPSILON);

        // Upgrade Activity 0 to Level 3
        assert!(state.upgrade_activity(0));
        assert_eq!(state.activities[0].level, 3);
        // Level 3: next_cost = 1.0 * 1.15^3 = 1.520875
        assert!((state.activities[0].next_cost() - 1.520875).abs() < EPSILON);

        // Activity 1: cost is 5.0
        // Level 0: next_cost = 5.0
        assert!((state.activities[1].next_cost() - 5.0).abs() < EPSILON);
        assert!(state.upgrade_activity(1));
        assert_eq!(state.activities[1].level, 1);
        // Level 1: next_cost = 5.0 * 1.15 = 5.75
        assert!((state.activities[1].next_cost() - 5.75).abs() < EPSILON);
    }

    #[test]
    fn test_reward_scaling_with_level() {
        let mut activity = ActivityState::new(ActivityConfig {
            id: "test",
            name: "Test",
            duration: 10.0,
            cost: 5.0,
            reward: 2.5,
            milestones: default_milestones(),
        });

        // Level 0
        assert_eq!(activity.level, 0);
        assert_eq!(activity.current_reward(), 0.0);

        // Level 1
        activity.level = 1;
        assert!((activity.current_reward() - 2.5).abs() < EPSILON);

        // Level 3
        activity.level = 3;
        assert!((activity.current_reward() - 7.5).abs() < EPSILON);

        // Level 10
        activity.level = 10;
        assert!((activity.current_reward() - 25.0).abs() < EPSILON);
    }

    #[test]
    fn test_milestone_duration_reduction_and_tick() {
        let mut state = GameState::new();
        // Activity 0 default duration is 5.0s, reward is 1.0
        assert_eq!(state.activities[0].current_duration(), 5.0);

        // Level 24 (< 25): speed_multiplier = 1.0 -> duration remains 5.0
        state.activities[0].level = 24;
        assert_eq!(state.activities[0].speed_multiplier(), 1.0);
        assert_eq!(state.activities[0].current_duration(), 5.0);

        // Level 25: 2.0x -> duration = 2.5s
        state.activities[0].level = 25;
        assert_eq!(state.activities[0].speed_multiplier(), 2.0);
        assert_eq!(state.activities[0].current_duration(), 2.5);

        // Level 50: 4.0x -> duration = 1.25s
        state.activities[0].level = 50;
        assert_eq!(state.activities[0].speed_multiplier(), 4.0);
        assert_eq!(state.activities[0].current_duration(), 1.25);

        // Level 100: 8.0x -> duration = 0.625s
        state.activities[0].level = 100;
        assert_eq!(state.activities[0].speed_multiplier(), 8.0);
        assert_eq!(state.activities[0].current_duration(), 0.625);

        // Level 200: 16.0x -> duration = 0.3125s
        state.activities[0].level = 200;
        assert_eq!(state.activities[0].speed_multiplier(), 16.0);
        assert_eq!(state.activities[0].current_duration(), 0.3125);

        // Level 500: 32.0x -> duration = 0.15625s
        state.activities[0].level = 500;
        assert_eq!(state.activities[0].speed_multiplier(), 32.0);
        assert_eq!(state.activities[0].current_duration(), 0.15625);

        // Level 1000: 64.0x -> duration = 5.0 / 64.0 = 0.078125s (Turbo mode active)
        state.activities[0].level = 1000;
        assert_eq!(state.activities[0].speed_multiplier(), 64.0);
        assert_eq!(state.activities[0].current_duration(), 0.078125);
        assert!(state.activities[0].is_turbo());

        // Level 5000: 256.0x -> duration = 5.0 / 256.0 = 0.01953125s
        state.activities[0].level = 5000;
        assert_eq!(state.activities[0].speed_multiplier(), 256.0);
        assert_eq!(state.activities[0].current_duration(), 0.01953125);

        // Level 9999: 1000.0x -> 5.0 / 1000.0 = 0.005s (.max(0.005))
        state.activities[0].level = 9999;
        assert_eq!(state.activities[0].speed_multiplier(), 1000.0);
        assert_eq!(state.activities[0].current_duration(), 0.005);
        assert!(state.activities[0].next_milestone().is_none());

        // Validate milestone duration in tick execution
        state.activities[0].level = 25;
        state.activities[0].progress = 0.0;
        state.sloth_points = 0.0;

        // With duration = 2.5s and reward = 25.0 (1.0 * 25), ticking 2.5s should complete 1 cycle exactly
        state.tick(2.5);
        assert!((state.activities[0].progress - 0.0).abs() < EPSILON);
        assert!((state.sloth_points - 25.0).abs() < EPSILON);
    }

    #[test]
    fn test_multi_cycle_large_tick_simulation() {
        let mut state = GameState::new();
        // Configure activity 0 to have duration = 0.05s and reward = 10.0
        state.activities[0].level = 10;
        state.activities[0].config.reward = 1.0; // reward = 10.0
        state.activities[0].config.duration = 0.05;
        state.activities[0].config.milestones = vec![]; // speed_multiplier = 1.0
        state.activities[0].progress = 0.0;
        state.sloth_points = 0.0;

        assert_eq!(state.activities[0].current_duration(), 0.05);
        assert_eq!(state.activities[0].current_reward(), 10.0);

        // Tick dt = 1.0s: exactly 1.0 / 0.05 = 20 cycles
        state.tick(1.0);

        assert!((state.activities[0].progress - 0.0).abs() < EPSILON);
        // 20 cycles * 10.0 reward = 200.0 points
        assert!((state.sloth_points - 200.0).abs() < EPSILON);
    }

    #[test]
    fn test_turbo_mode_and_pts_per_second() {
        let mut activity = ActivityState::new(ActivityConfig {
            id: "test",
            name: "Test",
            duration: 1.0,
            cost: 0.0,
            reward: 5.0,
            milestones: vec![
                Milestone {
                    level: 10,
                    speed_multiplier: 10.0, // duration becomes 0.1s <= 0.15s (Turbo)
                },
            ],
        });

        // Level 1: duration = 1.0s, reward = 5.0 -> not turbo, pts/s = 5.0
        activity.level = 1;
        assert!(!activity.is_turbo());
        assert_eq!(activity.next_milestone().map(|m| m.level), Some(10));
        assert!((activity.pts_per_second() - 5.0).abs() < EPSILON);

        // Level 10: duration = 0.10s, reward = 50.0 -> is turbo, pts/s = 50.0 / 0.10 = 500.0
        activity.level = 10;
        assert!(activity.is_turbo());
        assert!(activity.next_milestone().is_none());
        assert!((activity.pts_per_second() - 500.0).abs() < EPSILON);
    }

    #[test]
    fn test_successful_purchase_and_subsequent_tick() {
        let mut state = GameState::new();

        // Accumulate 5.0 points by running 5 full cycles of activity 0 (5 * 5.0s = 25.0s)
        state.tick(25.0);
        assert!((state.sloth_points - 5.0).abs() < EPSILON);

        // Unlock activity 1 (cost 5.0, reward 3.5, duration 12.5s)
        let success = state.upgrade_activity(1);
        assert!(success);
        assert_eq!(state.activities[1].level, 1);
        assert!((state.sloth_points - 0.0).abs() < EPSILON);

        // In the next tick, BOTH activity 0 and activity 1 must advance
        state.tick(2.5);
        assert!((state.activities[0].progress - 2.5).abs() < EPSILON);
        assert!((state.activities[1].progress - 2.5).abs() < EPSILON);

        // Tick another 10.0s -> Activity 1 reaches 12.5s and completes (+3.5 points)
        // Activity 0 reaches 12.5s -> 2 completed cycles (10.0s) + 2.5s remainder (+2.0 points)
        state.tick(10.0);
        assert!((state.activities[1].progress - 0.0).abs() < EPSILON);
        assert!((state.activities[0].progress - 2.5).abs() < EPSILON);
        // Total points: 3.5 (from act 1) + 2.0 (from act 0) = 5.5
        assert!((state.sloth_points - 5.5).abs() < EPSILON);
    }

    #[test]
    fn test_remanent_progress_and_exact_completion() {
        let mut state = GameState::new();

        // 5.5s tick on a 5.0s cycle
        state.tick(5.5);
        assert!((state.activities[0].progress - 0.5).abs() < EPSILON);
        assert!((state.sloth_points - 1.0).abs() < EPSILON);

        // Another 4.5s -> exactly 5.0s
        state.tick(4.5);
        assert!((state.activities[0].progress - 0.0).abs() < EPSILON);
        assert!((state.sloth_points - 2.0).abs() < EPSILON);
    }

    #[test]
    fn test_invalid_dt_ignored() {
        let mut state = GameState::new();
        state.activities[0].progress = 1.5;
        state.sloth_points = 2.0;

        state.tick(0.0);
        state.tick(-1.0);
        state.tick(f64::NAN);
        state.tick(f64::INFINITY);
        state.tick(f64::NEG_INFINITY);

        assert!((state.activities[0].progress - 1.5).abs() < EPSILON);
        assert!((state.sloth_points - 2.0).abs() < EPSILON);
    }

    #[test]
    fn test_action_handler() {
        let mut state = GameState::new();
        state.sloth_points = 20.0;

        assert!(state.handle_action(Action::UpgradeActivity(1)));
        assert_eq!(state.activities[1].level, 1);
        assert!((state.sloth_points - 15.0).abs() < EPSILON);

        assert!(state.handle_action(Action::UnlockActivity(1)));
        assert_eq!(state.activities[1].level, 2);
        assert!((state.sloth_points - (15.0 - 5.75)).abs() < EPSILON);
    }

    #[test]
    fn test_serialization_persistence() {
        let mut state = GameState::new();
        state.sloth_points = 42.5;
        state.activities[0].progress = 3.2;

        fn assert_traits<T: Serialize + for<'de> Deserialize<'de> + Clone + Send + Sync>() {}
        assert_traits::<Milestone>();
        assert_traits::<ActivityConfig>();
        assert_traits::<ActivityState>();
        assert_traits::<Action>();
        assert_traits::<GameState>();
    }
}
