#![forbid(unsafe_code)]

pub mod roster;

use serde::{Deserialize, Serialize};

pub use roster::{default_roster, ActivityConfig};

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
    /// Whether this activity is unlocked and producing rewards.
    pub unlocked: bool,
}

impl ActivityState {
    /// Creates a new `ActivityState` based on its configuration.
    /// An activity is unlocked by default if its cost is 0.0.
    #[must_use]
    pub fn new(config: ActivityConfig) -> Self {
        let unlocked = config.cost <= 0.0;
        Self {
            config,
            progress: 0.0,
            unlocked,
        }
    }

    /// Returns the progress ratio clamped between `0.0` and `1.0`.
    #[must_use]
    pub fn progress_ratio(&self) -> f64 {
        if !self.config.duration.is_finite() || self.config.duration <= 0.0 {
            0.0
        } else {
            (self.progress / self.config.duration).clamp(0.0, 1.0)
        }
    }

    /// Returns the remaining time in seconds to complete the current cycle.
    #[must_use]
    pub fn remaining_time(&self) -> f64 {
        if !self.config.duration.is_finite() || self.config.duration <= 0.0 {
            0.0
        } else {
            (self.config.duration - self.progress).clamp(0.0, self.config.duration)
        }
    }
}

/// Explicit action intent to drive deterministic state mutations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Action {
    /// Attempt to unlock an activity at the given roster index.
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
    /// The first activity is unlocked by default.
    #[must_use]
    pub fn new() -> Self {
        let activities = default_roster().into_iter().map(ActivityState::new).collect();
        Self {
            sloth_points: 0.0,
            activities,
        }
    }

    /// Attempts to unlock the activity at `index`.
    ///
    /// Returns `true` if the activity exists, was locked, and the player had enough
    /// Sloth Points to pay its cost. Returns `false` otherwise.
    pub fn unlock_activity(&mut self, index: usize) -> bool {
        if let Some(activity) = self.activities.get_mut(index)
            && !activity.unlocked
            && self.sloth_points >= activity.config.cost
        {
            self.sloth_points -= activity.config.cost;
            activity.unlocked = true;
            return true;
        }
        false
    }

    /// Handles an explicit player action intent.
    pub fn handle_action(&mut self, action: Action) -> bool {
        match action {
            Action::UnlockActivity(index) => self.unlock_activity(index),
        }
    }

    /// Advances the simulation by `dt` seconds deterministically.
    ///
    /// Only unlocked activities advance their progress.
    /// When an activity completes (`progress >= duration`), its reward is added to
    /// `sloth_points` and any remainder is preserved (`progress -= duration`).
    pub fn tick(&mut self, dt: f64) {
        if !dt.is_finite() || dt <= 0.0 {
            return;
        }

        for activity in &mut self.activities {
            if !activity.unlocked {
                continue;
            }

            if !activity.config.duration.is_finite() || activity.config.duration <= 0.0 {
                continue;
            }

            activity.progress += dt;

            // Fast-forward cycle calculations for very large delta times (e.g. offline progression)
            if activity.progress >= activity.config.duration * 100.0 {
                let fast_cycles = (activity.progress / activity.config.duration).floor();
                self.sloth_points += fast_cycles * activity.config.reward;
                activity.progress -= fast_cycles * activity.config.duration;
            }

            while activity.progress >= activity.config.duration {
                self.sloth_points += activity.config.reward;
                activity.progress -= activity.config.duration;
            }

            // Ensure progress is clamped to prevent negative precision drift or overflow
            activity.progress = activity.progress.clamp(0.0, activity.config.duration);
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

        // First activity must be unlocked by default
        assert!(state.activities[0].unlocked);
        assert!((state.activities[0].progress - 0.0).abs() < EPSILON);
        assert_eq!(state.activities[0].config.id, "wait_bar");

        // Remaining activities must be locked
        for (i, activity) in state.activities.iter().enumerate().skip(1) {
            assert!(!activity.unlocked, "Activity at index {i} should be locked initially");
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
    fn test_cannot_unlock_without_sufficient_points() {
        let mut state = GameState::new();
        // Activity 1 requires 5.0 points, state starts with 0.0
        let unlocked = state.unlock_activity(1);
        assert!(!unlocked);
        assert!(!state.activities[1].unlocked);
        assert_eq!(state.sloth_points, 0.0);

        // Even with partial points
        state.sloth_points = 4.9;
        let unlocked = state.unlock_activity(1);
        assert!(!unlocked);
        assert!(!state.activities[1].unlocked);
        assert!((state.sloth_points - 4.9).abs() < EPSILON);
    }

    #[test]
    fn test_cannot_unlock_invalid_indices_or_already_unlocked() {
        let mut state = GameState::new();
        state.sloth_points = 1000.0;

        // Index out of bounds
        assert!(!state.unlock_activity(99));

        // Already unlocked activity (index 0)
        assert!(!state.unlock_activity(0));
        assert_eq!(state.sloth_points, 1000.0);
    }

    #[test]
    fn test_successful_purchase_and_subsequent_tick() {
        let mut state = GameState::new();

        // Accumulate 5.0 points by running 5 full cycles of activity 0 (5 * 5.0s = 25.0s)
        state.tick(25.0);
        assert!((state.sloth_points - 5.0).abs() < EPSILON);

        // Unlock activity 1 (cost 5.0, reward 3.5, duration 12.5s)
        let success = state.unlock_activity(1);
        assert!(success);
        assert!(state.activities[1].unlocked);
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
        state.sloth_points = 10.0;

        assert!(state.handle_action(Action::UnlockActivity(1)));
        assert!(state.activities[1].unlocked);
        assert!((state.sloth_points - 5.0).abs() < EPSILON);
    }

    #[test]
    fn test_serialization_persistence() {
        let mut state = GameState::new();
        state.sloth_points = 42.5;
        state.activities[0].progress = 3.2;

        fn assert_traits<T: Serialize + for<'de> Deserialize<'de> + Clone + Send + Sync>() {}
        assert_traits::<ActivityConfig>();
        assert_traits::<ActivityState>();
        assert_traits::<Action>();
        assert_traits::<GameState>();
    }
}
