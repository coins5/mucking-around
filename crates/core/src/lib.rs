#![forbid(unsafe_code)]

use std::collections::HashMap;
use serde::{Deserialize, Serialize};

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

/// Component representing an idle progress generator.
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
    /// Creates a new `Generator` with default settings (5.0s cycle, 0.7 Primary resource).
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Advances the generator by `dt` seconds deterministically.
    /// Returns the number of completed cycles.
    ///
    /// Preserves remainder progress when cycles complete.
    /// Clamps progress between `0.0` and `target_duration` to prevent numerical drift or visual overflow.
    pub fn tick(&mut self, dt: f64) -> u64 {
        if !dt.is_finite() || dt <= 0.0 || !self.target_duration.is_finite() || self.target_duration <= 0.0 {
            return 0;
        }

        self.progress += dt;

        let mut completed = 0u64;

        // Optimization for very large delta times (e.g., long offline progression)
        if self.progress >= self.target_duration * 100.0 {
            let fast_cycles = (self.progress / self.target_duration).floor() as u64;
            completed += fast_cycles;
            self.progress -= (fast_cycles as f64) * self.target_duration;
        }

        while self.progress >= self.target_duration {
            completed += 1;
            self.progress -= self.target_duration;
        }

        // Guard against negative precision drift and visual overflow
        self.progress = self.progress.clamp(0.0, self.target_duration);

        completed
    }

    /// Returns the progress ratio of the current cycle clamped between `0.0` and `1.0`.
    #[must_use]
    pub fn progress_ratio(&self) -> f64 {
        if !self.target_duration.is_finite() || self.target_duration <= 0.0 {
            return 0.0;
        }
        (self.progress / self.target_duration).clamp(0.0, 1.0)
    }
}

/// The game state managing the idle generator and resource balances.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GameState {
    /// The primary generator instance.
    pub generator: Generator,
    /// Extensible resource storage indexed by `ResourceId`.
    pub resources: HashMap<ResourceId, f64>,
}

impl Default for GameState {
    fn default() -> Self {
        let mut resources = HashMap::new();
        resources.insert(ResourceId::Primary, 0.0);
        Self {
            generator: Generator::default(),
            resources,
        }
    }
}

impl GameState {
    /// Creates a new `GameState` with default initial values.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Retrieves the current balance of a resource.
    /// Returns `0.0` if the resource is not initialized.
    #[must_use]
    pub fn get_resource(&self, id: ResourceId) -> f64 {
        self.resources
            .get(&id)
            .copied()
            .unwrap_or(0.0)
            .clamp(0.0, f64::MAX)
    }

    /// Sets the balance of a resource directly.
    /// Safely ignores negative, NaN, or non-finite values.
    pub fn set_resource(&mut self, id: ResourceId, amount: f64) {
        if amount.is_finite() && amount >= 0.0 {
            self.resources.insert(id, amount);
        }
    }

    /// Safely adds `amount` to the resource balance.
    /// Ignores non-positive, NaN, or infinite amounts.
    pub fn add_resource(&mut self, id: ResourceId, amount: f64) {
        if !amount.is_finite() || amount <= 0.0 {
            return;
        }
        let current = self.get_resource(id);
        let updated = (current + amount).clamp(0.0, f64::MAX);
        self.resources.insert(id, updated);
    }

    /// Advances the simulation by `dt` seconds deterministically.
    ///
    /// If `dt` is non-positive or non-finite, the call is safely ignored.
    /// Cycles completed by the generator add `output_amount` to `output_resource`.
    pub fn tick(&mut self, dt: f64) {
        if !dt.is_finite() || dt <= 0.0 {
            return;
        }

        let completed = self.generator.tick(dt);
        if completed > 0 {
            let reward = self.generator.output_amount * (completed as f64);
            self.add_resource(self.generator.output_resource, reward);
        }
    }

    /// Returns the progress ratio of the active generator cycle clamped between `0.0` and `1.0`.
    #[must_use]
    pub fn progress_ratio(&self) -> f64 {
        self.generator.progress_ratio()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const EPSILON: f64 = 1e-9;

    #[test]
    fn test_default_initial_values() {
        let state = GameState::new();
        assert!((state.generator.progress - 0.0).abs() < EPSILON);
        assert!((state.generator.target_duration - 5.0).abs() < EPSILON);
        assert_eq!(state.generator.output_resource, ResourceId::Primary);
        assert!((state.generator.output_amount - 0.7).abs() < EPSILON);
        assert!((state.get_resource(ResourceId::Primary) - 0.0).abs() < EPSILON);
        assert!((state.progress_ratio() - 0.0).abs() < EPSILON);
    }

    #[test]
    fn test_tick_sub_threshold() {
        let mut state = GameState::new();

        state.tick(1.0);
        assert!((state.generator.progress - 1.0).abs() < EPSILON);
        assert!((state.get_resource(ResourceId::Primary) - 0.0).abs() < EPSILON);
        assert!((state.progress_ratio() - 0.2).abs() < EPSILON);

        state.tick(2.5);
        assert!((state.generator.progress - 3.5).abs() < EPSILON);
        assert!((state.get_resource(ResourceId::Primary) - 0.0).abs() < EPSILON);
        assert!((state.progress_ratio() - 0.7).abs() < EPSILON);

        state.tick(1.4);
        assert!((state.generator.progress - 4.9).abs() < EPSILON);
        assert!((state.get_resource(ResourceId::Primary) - 0.0).abs() < EPSILON);
    }

    #[test]
    fn test_tick_exact_completion() {
        let mut state = GameState::new();

        state.tick(5.0);
        assert!((state.generator.progress - 0.0).abs() < EPSILON);
        assert!((state.get_resource(ResourceId::Primary) - 0.7).abs() < EPSILON);
    }

    #[test]
    fn test_tick_overflow_remanent() {
        let mut state = GameState::new();

        // 5.5s tick on a 5.0s cycle -> 0.5s remainder, 0.7 Primary awarded
        state.tick(5.5);
        assert!((state.generator.progress - 0.5).abs() < EPSILON);
        assert!((state.get_resource(ResourceId::Primary) - 0.7).abs() < EPSILON);

        // Another 4.6s tick -> total progress was 0.5 + 4.6 = 5.1s -> 0.1s remainder, 1.4 Primary total
        state.tick(4.6);
        assert!((state.generator.progress - 0.1).abs() < EPSILON);
        assert!((state.get_resource(ResourceId::Primary) - 1.4).abs() < EPSILON);
    }

    #[test]
    fn test_tick_multiple_cycles() {
        let mut state = GameState::new();

        // 12.0s tick -> 2 completed cycles (10.0s), 2.0s remainder, 1.4 Primary awarded
        state.tick(12.0);
        assert!((state.generator.progress - 2.0).abs() < EPSILON);
        assert!((state.get_resource(ResourceId::Primary) - 1.4).abs() < EPSILON);
    }

    #[test]
    fn test_fractional_frame_ticks() {
        let mut state = GameState::new();

        // 40 ticks of 0.125s (1/8s) = exactly 5.0s (exact in IEEE 754)
        for _ in 0..40 {
            state.tick(0.125);
        }

        assert!(state.generator.progress.abs() < EPSILON);
        assert!((state.get_resource(ResourceId::Primary) - 0.7).abs() < EPSILON);

        // Another 20 ticks of 0.25s = 5.0s
        for _ in 0..20 {
            state.tick(0.25);
        }

        assert!(state.generator.progress.abs() < EPSILON);
        assert!((state.get_resource(ResourceId::Primary) - 1.4).abs() < EPSILON);
    }

    #[test]
    fn test_invalid_dt_handling() {
        let mut state = GameState::new();
        state.generator.progress = 2.0;
        state.set_resource(ResourceId::Primary, 1.0);

        state.tick(0.0);
        state.tick(-1.5);
        state.tick(f64::NAN);
        state.tick(f64::INFINITY);
        state.tick(f64::NEG_INFINITY);

        assert!((state.generator.progress - 2.0).abs() < EPSILON);
        assert!((state.get_resource(ResourceId::Primary) - 1.0).abs() < EPSILON);
    }

    #[test]
    fn test_resource_store_operations() {
        let mut state = GameState::new();
        assert_eq!(state.get_resource(ResourceId::Primary), 0.0);

        state.add_resource(ResourceId::Primary, 1.5);
        assert!((state.get_resource(ResourceId::Primary) - 1.5).abs() < EPSILON);

        // Invalid additions should be ignored
        state.add_resource(ResourceId::Primary, -0.5);
        state.add_resource(ResourceId::Primary, 0.0);
        state.add_resource(ResourceId::Primary, f64::NAN);
        state.add_resource(ResourceId::Primary, f64::INFINITY);
        assert!((state.get_resource(ResourceId::Primary) - 1.5).abs() < EPSILON);

        state.set_resource(ResourceId::Primary, 10.0);
        assert!((state.get_resource(ResourceId::Primary) - 10.0).abs() < EPSILON);

        // Invalid sets should be ignored
        state.set_resource(ResourceId::Primary, -5.0);
        state.set_resource(ResourceId::Primary, f64::NAN);
        assert!((state.get_resource(ResourceId::Primary) - 10.0).abs() < EPSILON);
    }

    #[test]
    fn test_progress_ratio() {
        let mut state = GameState::new();
        assert!((state.progress_ratio() - 0.0).abs() < EPSILON);

        state.generator.progress = 2.5;
        assert!((state.progress_ratio() - 0.5).abs() < EPSILON);

        state.generator.progress = 5.0;
        assert!((state.progress_ratio() - 1.0).abs() < EPSILON);

        state.generator.target_duration = 0.0;
        assert!((state.progress_ratio() - 0.0).abs() < EPSILON);
    }

    #[test]
    fn test_traits() {
        fn assert_traits<T: Serialize + for<'de> Deserialize<'de> + Clone + Send + Sync>() {}
        assert_traits::<ResourceId>();
        assert_traits::<Generator>();
        assert_traits::<GameState>();
    }
}
