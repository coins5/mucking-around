#![forbid(unsafe_code)]

pub mod achievements;
pub mod distraction;
pub mod pen;
pub mod persistence;
pub mod prestige;
pub mod roster;
pub mod stats;
pub mod view;

use serde::{Deserialize, Serialize};
use std::collections::HashSet;

pub use achievements::{AchievementCondition, AchievementConfig, default_achievements};
pub use distraction::{
    ActiveDistractionState, ClaimedDistractionFeedback, DistractionConfig, DistractionRewardType,
    DistractionSystemConfig, default_distractions,
};
pub use pen::PenClickConfig;
pub use persistence::{OfflineProgressReport, PersistenceConfig};
pub use prestige::{PermanentUpgradeConfig, PrestigeConfig, default_permanent_upgrades};
pub use roster::{ActivityConfig, Milestone, default_milestones, default_roster};
pub use stats::{ExistentialStats, ProductiveComparison, default_productive_comparisons};
pub use view::ActiveView;

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
        if !dt.is_finite()
            || dt <= 0.0
            || !self.target_duration.is_finite()
            || self.target_duration <= 0.0
        {
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
    /// Divisor applied to base duration (e.g. 1.25 for eternal_sloth -20% duration).
    #[serde(default = "default_duration_divisor")]
    pub duration_divisor: f64,
    /// Exponent applied to cost scaling (e.g. 1.12 for cost_optimization, default 1.15).
    #[serde(default = "default_cost_exponent")]
    pub cost_exponent: f64,
    /// Whether this activity has been discovered/revealed in the UI.
    #[serde(default)]
    pub is_revealed: bool,
}

fn default_duration_divisor() -> f64 {
    1.0
}

fn default_cost_exponent() -> f64 {
    1.15
}

impl ActivityState {
    /// Creates a new `ActivityState` based on its configuration.
    /// An activity starts at level 1 if its cost is 0.0 (unlocked), otherwise level 0 (locked).
    #[must_use]
    pub fn new(config: ActivityConfig) -> Self {
        let level = if config.cost <= 0.0 { 1 } else { 0 };
        let is_revealed = level > 0;
        Self {
            config,
            progress: 0.0,
            level,
            duration_divisor: 1.0,
            cost_exponent: 1.15,
            is_revealed,
        }
    }

    /// Whether this activity is currently active / unlocked.
    #[must_use]
    pub fn is_unlocked(&self) -> bool {
        self.level > 0
    }

    /// Returns the cost for the next upgrade (or initial unlock if level is 0).
    /// - If `level == 0`: returns `config.cost`.
    /// - If `level > 0`: returns `base_cost * cost_exponent^level` (where `base_cost = if cost <= 0.0 { 1.0 } else { cost }`).
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
            let exponent = if self.cost_exponent.is_finite() && self.cost_exponent > 0.0 {
                self.cost_exponent
            } else {
                1.15
            };
            base_cost * exponent.powi(self.level as i32)
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

    /// Returns the effective cycle duration considering speed milestones and duration divisor:
    /// `((config.duration / duration_divisor) / speed_multiplier()).max(0.005)`.
    #[must_use]
    pub fn current_duration(&self) -> f64 {
        let mult = self.speed_multiplier();
        let divisor = if self.duration_divisor.is_finite() && self.duration_divisor > 0.0 {
            self.duration_divisor
        } else {
            1.0
        };
        let base_duration = self.config.duration / divisor;
        if !mult.is_finite() || mult <= 0.0 {
            base_duration.max(0.005)
        } else {
            (base_duration / mult).max(0.005)
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
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Action {
    /// Attempt to upgrade/unlock an activity at the given roster index.
    UpgradeActivity(usize),
    /// Legacy alias for unlocking/upgrading an activity.
    UnlockActivity(usize),
    /// Trigger an existential crisis (prestige) to claim Epiphanies.
    TriggerPrestige,
    /// Buy a permanent upgrade by its unique identifier.
    BuyPermanentUpgrade(String),
    /// Open the prestige confirmation dialog.
    OpenPrestigeDialog,
    /// Close the prestige confirmation dialog.
    ClosePrestigeDialog,
    /// Confirm prestige and close the dialog.
    ConfirmPrestige,
    /// Open the permanent upgrade menu.
    OpenUpgradeMenu,
    /// Close the permanent upgrade menu.
    CloseUpgradeMenu,
    /// Toggle the permanent upgrade menu.
    ToggleUpgradeMenu,
    /// Perform an active pen click.
    PenClick,
    /// Claim currently active distraction.
    ClaimDistraction,
    /// Set current active view screen/modal.
    SetView(ActiveView),
    /// Return to main dashboard view.
    CloseView,
}

/// The game state managing procrastination activities, Sloth Points, and Epiphanies.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GameState {
    /// Current balance of Sloth Points ("Puntos de Flojera").
    pub sloth_points: f64,
    /// Lifetime historic Sloth Points accumulated across all prestiges.
    pub lifetime_sloth_points: f64,
    /// Current balance of unspent Epiphanies ("Epifanías").
    pub epiphanies: u32,
    /// Total lifetime Epiphanies earned across all prestiges.
    pub total_epiphanies_earned: u32,
    /// Set of unique identifiers of purchased permanent upgrades.
    pub purchased_permanent_upgrades: HashSet<String>,
    /// Configuration for the prestige scaling formulas and bonus.
    pub prestige_config: PrestigeConfig,
    /// Accumulated delta time in seconds for the autopilot upgrade.
    pub autopilot_timer: f64,
    /// Dynamic state of all activities in the roster.
    pub activities: Vec<ActivityState>,
    /// Indicates whether the Existential Crisis (prestige) confirmation dialog is open.
    #[serde(default)]
    pub in_prestige_dialog: bool,
    /// Indicates whether the Zen Enlightenment (permanent upgrades) menu is open.
    #[serde(default)]
    pub in_upgrade_menu: bool,
    /// Indicates whether the Existential Crisis (prestige) is revealed in the UI.
    #[serde(default)]
    pub prestige_revealed: bool,
    /// Set of unique identifiers of revealed permanent upgrades in the shop.
    #[serde(default)]
    pub revealed_permanent_upgrades: HashSet<String>,
    /// Maximum/initial duration of the currently active frenzy effect.
    #[serde(default)]
    pub frenzy_max_duration: f64,
    /// Feedback report for the last claimed unexpected distraction.
    #[serde(default)]
    pub last_claimed_distraction: Option<ClaimedDistractionFeedback>,

    /// Existential metrics tracked across all actions and ticks.
    #[serde(default)]
    pub existential_stats: ExistentialStats,
    /// Currently active unexpected distraction event, if any.
    #[serde(default)]
    pub active_distraction: Option<ActiveDistractionState>,
    /// Configuration for random distraction spawning.
    #[serde(default)]
    pub distraction_config: DistractionSystemConfig,
    /// Countdown timer in seconds until next distraction spawn attempt.
    #[serde(default = "default_distraction_spawn_timer")]
    pub distraction_spawn_timer: f64,
    /// Temporary multiplier from active frenzy distractions.
    #[serde(default = "default_frenzy_multiplier")]
    pub frenzy_multiplier: f64,
    /// Remaining duration in seconds of the current frenzy effect.
    #[serde(default)]
    pub frenzy_timer: f64,
    /// Set of unique identifiers of unlocked achievements.
    #[serde(default)]
    pub unlocked_achievements: HashSet<String>,
    /// Configuration roster of all available achievements.
    #[serde(default = "default_achievements")]
    pub achievement_roster: Vec<AchievementConfig>,
    /// Configuration for active pen click habit mechanics.
    #[serde(default)]
    pub pen_config: PenClickConfig,
    /// Index tracking the last played onomatopoeia.
    #[serde(default)]
    pub last_pen_sound_index: usize,
    /// Last played onomatopoeia sound text for visual UI feedback.
    #[serde(default)]
    pub last_pen_sound: Option<String>,
    /// Configuration for auto-saving and offline progression.
    #[serde(default)]
    pub persistence_config: PersistenceConfig,
    /// Unix timestamp in seconds of the last recorded save/tick.
    #[serde(default)]
    pub last_save_timestamp: u64,
    /// Current active modal or screen view.
    #[serde(default)]
    pub active_view: ActiveView,
    /// Last generated report for offline absence progress.
    #[serde(default)]
    pub offline_report: Option<OfflineProgressReport>,
    /// Deterministic pseudo-random seed state.
    #[serde(default = "default_prng_seed")]
    pub prng_seed: u64,
}

fn default_distraction_spawn_timer() -> f64 {
    60.0
}

fn default_frenzy_multiplier() -> f64 {
    1.0
}

fn default_prng_seed() -> u64 {
    6364136223846793005
}

fn next_random_f64(seed: &mut u64) -> f64 {
    *seed = seed
        .wrapping_mul(6364136223846793005)
        .wrapping_add(1442695040888963407);
    ((*seed >> 11) as f64) / ((1u64 << 53) as f64)
}

impl Default for GameState {
    fn default() -> Self {
        Self::new()
    }
}

impl GameState {
    /// Initializes a new game state with default configurations and roster.
    /// The first activity starts at level 1 (unlocked), the rest at level 0 (locked).
    #[must_use]
    pub fn new() -> Self {
        let activities = default_roster()
            .into_iter()
            .map(ActivityState::new)
            .collect();
        let mut state = Self {
            sloth_points: 0.0,
            lifetime_sloth_points: 0.0,
            epiphanies: 0,
            total_epiphanies_earned: 0,
            purchased_permanent_upgrades: HashSet::new(),
            prestige_config: PrestigeConfig::default(),
            autopilot_timer: 0.0,
            activities,
            in_prestige_dialog: false,
            in_upgrade_menu: false,
            prestige_revealed: false,
            revealed_permanent_upgrades: HashSet::new(),
            frenzy_max_duration: 0.0,
            last_claimed_distraction: None,

            existential_stats: ExistentialStats::default(),
            active_distraction: None,
            distraction_config: DistractionSystemConfig::default(),
            distraction_spawn_timer: 60.0,
            frenzy_multiplier: 1.0,
            frenzy_timer: 0.0,
            unlocked_achievements: HashSet::new(),
            achievement_roster: default_achievements(),
            pen_config: PenClickConfig::default(),
            last_pen_sound_index: 0,
            last_pen_sound: None,
            persistence_config: PersistenceConfig::default(),
            last_save_timestamp: 0,
            active_view: ActiveView::MainDashboard,
            offline_report: None,
            prng_seed: default_prng_seed(),
        };
        state.reset_distraction_spawn_timer();
        state.check_reveals();
        state
    }

    /// Resets the distraction countdown timer to a deterministic pseudo-random duration.
    pub fn reset_distraction_spawn_timer(&mut self) {
        let min = self.distraction_config.min_spawn_interval;
        let max = self.distraction_config.max_spawn_interval;
        let r = next_random_f64(&mut self.prng_seed);
        self.distraction_spawn_timer = min + r * (max - min).max(0.0);
    }

    /// Evaluates dynamic revelation conditions for activities, prestige, and permanent upgrades.
    pub fn check_reveals(&mut self) {
        // Activity 0 is always revealed
        if let Some(first) = self.activities.get_mut(0) {
            first.is_revealed = true;
        }

        // Any activity with level > 0 is revealed
        for activity in &mut self.activities {
            if activity.level > 0 {
                activity.is_revealed = true;
            }
        }

        // Reveal the first locked & unrevealed activity if current points can afford it
        if let Some(next_to_reveal) = self.activities.iter_mut().find(|a| !a.is_revealed)
            && self.sloth_points >= next_to_reveal.next_cost()
        {
            next_to_reveal.is_revealed = true;
        }

        // Reveal prestige once the player has enough to buy the first permanent upgrade (cost 2)
        if !self.prestige_revealed {
            let min_upgrade_cost = default_permanent_upgrades()
                .iter()
                .map(|u| u.cost_epiphanies)
                .min()
                .unwrap_or(2);
            if (self.epiphanies + self.claimable_epiphanies()) >= min_upgrade_cost
                || self.total_epiphanies_earned > 0
                || self.existential_stats.total_prestiges > 0
            {
                self.prestige_revealed = true;
            }
        }

        // Reveal permanent upgrades: show only those affordable with current epiphanies, or already bought
        let upgrades = default_permanent_upgrades();
        for u in &upgrades {
            if self.epiphanies >= u.cost_epiphanies || self.has_permanent_upgrade(u.id) {
                self.revealed_permanent_upgrades.insert(u.id.to_string());
            }
        }
    }

    /// Returns whether a permanent upgrade has been revealed in the shop.
    #[must_use]
    pub fn is_permanent_upgrade_revealed(&self, id: &str) -> bool {
        self.revealed_permanent_upgrades.contains(id)
    }

    /// Registers an additional distraction configuration into the random spawn pool.
    pub fn register_distraction(&mut self, distraction: DistractionConfig) {
        self.distraction_config.add_distraction(distraction);
    }

    /// Attempts to upgrade (or unlock) the activity at `index`.
    ///
    /// Returns `true` if the activity exists, is revealed, and the player had enough
    /// Sloth Points to pay its next cost. Increments `activity.level` and deducts cost.
    /// Returns `false` otherwise.
    pub fn upgrade_activity(&mut self, index: usize) -> bool {
        self.check_reveals();
        if let Some(activity) = self.activities.get_mut(index) {
            if !activity.is_revealed {
                return false;
            }
            let cost = activity.next_cost();
            if self.sloth_points >= cost {
                self.sloth_points -= cost;
                activity.level += 1;
                self.check_achievements();
                self.check_reveals();
                return true;
            }
        }
        false
    }

    /// Legacy alias for `upgrade_activity`.
    pub fn unlock_activity(&mut self, index: usize) -> bool {
        self.upgrade_activity(index)
    }

    /// Returns the number of Epiphanies that can currently be claimed via prestige.
    ///
    /// Formula:
    /// `total = floor((lifetime_sloth_points / base_cost)^exponent)`
    /// Returns `total.saturating_sub(total_epiphanies_earned)`.
    #[must_use]
    pub fn claimable_epiphanies(&self) -> u32 {
        if !self.lifetime_sloth_points.is_finite()
            || self.lifetime_sloth_points <= 0.0
            || !self.prestige_config.base_cost.is_finite()
            || self.prestige_config.base_cost <= 0.0
        {
            return 0;
        }

        let ratio = self.lifetime_sloth_points / self.prestige_config.base_cost;
        let raw_total = ratio.powf(self.prestige_config.exponent).floor();
        let total = if raw_total.is_finite() && raw_total > 0.0 {
            raw_total.min(u32::MAX as f64) as u32
        } else {
            0
        };

        total.saturating_sub(self.total_epiphanies_earned)
    }

    /// Returns the global production multiplier derived from unspent Epiphanies.
    ///
    /// Checks if the player owns "zen_enlightenment":
    /// - With upgrade: +15% per point (bonus = 0.15)
    /// - Without upgrade: +10% per point (config.default_bonus_per_point)
    ///
    /// Returns `1.0 + (epiphanies * bonus)`.
    #[must_use]
    pub fn prestige_multiplier(&self) -> f64 {
        let bonus = if self.has_permanent_upgrade("zen_enlightenment") {
            0.15
        } else {
            self.prestige_config.default_bonus_per_point
        };
        1.0 + (self.epiphanies as f64 * bonus)
    }

    /// Returns the total cumulative passive bonus granted by unlocked achievements.
    #[must_use]
    pub fn achievements_multiplier(&self) -> f64 {
        let mut bonus = 0.0;
        for ach in &self.achievement_roster {
            if self.unlocked_achievements.contains(ach.id) {
                bonus += ach.bonus_multiplier;
            }
        }
        1.0 + bonus
    }

    /// Returns the active frenzy multiplier if active, otherwise 1.0.
    #[must_use]
    pub fn current_frenzy_multiplier(&self) -> f64 {
        if self.frenzy_timer > 0.0 {
            self.frenzy_multiplier.max(1.0)
        } else {
            1.0
        }
    }

    /// Returns the combined multiplier: Prestige * Achievements * Frenzy.
    #[must_use]
    pub fn total_multiplier(&self) -> f64 {
        self.prestige_multiplier()
            * self.achievements_multiplier()
            * self.current_frenzy_multiplier()
    }

    /// Returns the aggregate base production rate in points per second across all unlocked activities
    /// scaled by prestige and achievements multipliers.
    #[must_use]
    pub fn total_pts_per_second(&self) -> f64 {
        let base_rate: f64 = self
            .activities
            .iter()
            .filter(|a| a.is_unlocked())
            .map(|a| a.pts_per_second())
            .sum();
        base_rate * self.prestige_multiplier() * self.achievements_multiplier()
    }

    /// Returns whether a permanent upgrade with the given ID has been purchased.
    #[must_use]
    pub fn has_permanent_upgrade(&self, id: &str) -> bool {
        self.purchased_permanent_upgrades.contains(id)
    }

    /// Attempts to purchase a permanent upgrade using Epiphanies.
    ///
    /// If the player has sufficient Epiphanies and has not already bought it:
    /// deducts the cost, records the purchase, applies upgrade effects, and returns `true`.
    /// Returns `false` otherwise.
    pub fn buy_permanent_upgrade(&mut self, id: &str) -> bool {
        self.check_reveals();
        if self.has_permanent_upgrade(id) || !self.is_permanent_upgrade_revealed(id) {
            return false;
        }

        let roster = default_permanent_upgrades();
        if let Some(upgrade) = roster
            .iter()
            .find(|u| u.id == id && self.epiphanies >= u.cost_epiphanies)
        {
            self.epiphanies -= upgrade.cost_epiphanies;
            self.purchased_permanent_upgrades.insert(id.to_string());
            self.apply_permanent_upgrade_effects();
            self.check_reveals();
            return true;
        }

        false
    }

    /// Applies active permanent upgrade effects to all activities.
    pub fn apply_permanent_upgrade_effects(&mut self) {
        let duration_divisor = if self.has_permanent_upgrade("eternal_sloth") {
            1.25
        } else {
            1.0
        };
        let cost_exponent = if self.has_permanent_upgrade("cost_optimization") {
            1.12
        } else {
            1.15
        };

        for activity in &mut self.activities {
            activity.duration_divisor = duration_divisor;
            activity.cost_exponent = cost_exponent;
        }
    }

    /// Triggers an Existential Crisis (Prestige reset).
    ///
    /// If `claimable_epiphanies() == 0`, returns `false`.
    /// Otherwise:
    /// - Adds claimed Epiphanies to `epiphanies` and `total_epiphanies_earned`.
    /// - Resets `sloth_points` to 0.0.
    /// - If the player has "muscle_memory", Activity 0 begins at level 10, otherwise 1.
    /// - Remaining activities are reset to level 0 (locked).
    /// - All activity cycle progress is reset to 0.0.
    /// - Resets `autopilot_timer` to 0.0.
    /// - Clears temporary distraction and frenzy states.
    /// - Increments total prestiges counter.
    /// - Re-applies active permanent upgrade effects.
    /// - Returns `true`.
    pub fn trigger_prestige(&mut self) -> bool {
        let to_claim = self.claimable_epiphanies();
        if to_claim == 0 {
            return false;
        }

        self.epiphanies += to_claim;
        self.total_epiphanies_earned += to_claim;
        self.sloth_points = 0.0;
        self.autopilot_timer = 0.0;
        self.frenzy_timer = 0.0;
        self.active_distraction = None;
        self.existential_stats.total_prestiges += 1;

        let first_level = if self.has_permanent_upgrade("muscle_memory") {
            10
        } else {
            1
        };

        for (index, activity) in self.activities.iter_mut().enumerate() {
            activity.level = if index == 0 { first_level } else { 0 };
            activity.progress = 0.0;
        }

        self.apply_permanent_upgrade_effects();
        self.check_achievements();
        self.check_reveals();
        true
    }

    /// Changes the active view mode and synchronizes legacy modal flags.
    pub fn set_view(&mut self, view: ActiveView) {
        self.active_view = view;
        self.in_prestige_dialog = view == ActiveView::PrestigeDialog;
        self.in_upgrade_menu = view == ActiveView::PermanentUpgradesShop;
    }

    /// Opens the Existential Crisis confirmation dialog.
    pub fn open_prestige_dialog(&mut self) {
        self.set_view(ActiveView::PrestigeDialog);
    }

    /// Closes the Existential Crisis confirmation dialog.
    pub fn close_prestige_dialog(&mut self) {
        self.set_view(ActiveView::MainDashboard);
    }

    /// Confirms prestige from the dialog: closes the dialog and triggers prestige.
    pub fn confirm_prestige(&mut self) -> bool {
        self.close_prestige_dialog();
        self.trigger_prestige()
    }

    /// Cancels the prestige dialog without triggering prestige.
    pub fn cancel_prestige(&mut self) {
        self.close_prestige_dialog();
    }

    /// Opens the permanent upgrades shop menu.
    pub fn open_upgrade_menu(&mut self) {
        self.set_view(ActiveView::PermanentUpgradesShop);
    }

    /// Closes the permanent upgrades shop menu.
    pub fn close_upgrade_menu(&mut self) {
        self.set_view(ActiveView::MainDashboard);
    }

    /// Toggles the permanent upgrades shop menu open or closed.
    pub fn toggle_upgrade_menu(&mut self) {
        if self.active_view == ActiveView::PermanentUpgradesShop {
            self.set_view(ActiveView::MainDashboard);
        } else {
            self.set_view(ActiveView::PermanentUpgradesShop);
        }
    }

    /// Executes the active manual "Pen Click" habit action.
    ///
    /// - Adds points scaled by lifetime points, prestige, achievements, and frenzy.
    /// - Advances progress by 2% on the unlocked activity closest to completing.
    /// - Cycles through humorous rotating onomatopoeias.
    /// - Evaluates achievements.
    pub fn pen_click(&mut self) -> (f64, &'static str) {
        let prestige_mult = self.prestige_multiplier();
        let achieve_mult = self.achievements_multiplier();
        let frenzy_mult = self.current_frenzy_multiplier();
        let scaling = 1.0 + self.lifetime_sloth_points * self.pen_config.lifetime_scaling_factor;
        let earned =
            self.pen_config.base_reward * scaling * prestige_mult * achieve_mult * frenzy_mult;

        self.sloth_points += earned;
        self.lifetime_sloth_points += earned;
        self.existential_stats.total_sloth_points_earned += earned;
        self.existential_stats.total_pen_clicks += 1;

        // Advance 2% duration on the unlocked activity closest to completing
        let closest_idx = self
            .activities
            .iter()
            .enumerate()
            .filter(|(_, a)| a.is_unlocked())
            .min_by(|(_, a), (_, b)| a.remaining_time().total_cmp(&b.remaining_time()))
            .map(|(i, _)| i);

        if let Some(idx) = closest_idx {
            let act = &mut self.activities[idx];
            let boost = act.current_duration() * self.pen_config.bar_speedup_percentage;
            act.progress = (act.progress + boost).min(act.current_duration());
        }

        let sound = if !self.pen_config.sound_effects.is_empty() {
            let s = self.pen_config.sound_effects
                [self.last_pen_sound_index % self.pen_config.sound_effects.len()];
            self.last_pen_sound_index =
                (self.last_pen_sound_index + 1) % self.pen_config.sound_effects.len();
            s
        } else {
            "*¡Clic!*"
        };
        self.last_pen_sound = Some(sound.to_string());

        self.check_achievements();
        self.check_reveals();
        (earned, sound)
    }

    /// Claims the currently active distraction event, applying its effect immediately.
    ///
    /// Returns `Some(DistractionRewardType)` if an active distraction was claimed, or `None`.
    pub fn claim_distraction(&mut self) -> Option<DistractionRewardType> {
        let active = self.active_distraction.take()?;
        self.existential_stats.total_distractions_claimed += 1;

        let effect_summary = match &active.config.reward {
            DistractionRewardType::Frenzy {
                multiplier,
                duration_secs,
            } => {
                self.frenzy_multiplier = *multiplier;
                self.frenzy_timer = *duration_secs;
                self.frenzy_max_duration = *duration_secs;
                format!("¡Frenesí x{multiplier:.1} por {duration_secs:.0}s activado!")
            }
            DistractionRewardType::InstantSloth {
                percentage_of_current,
                min_flat,
            } => {
                let pts = (self.sloth_points * percentage_of_current).max(*min_flat);
                self.sloth_points += pts;
                self.lifetime_sloth_points += pts;
                self.existential_stats.total_sloth_points_earned += pts;
                format!("+{pts:.2} Puntos de Flojera al instante")
            }
            DistractionRewardType::TimeWarp { simulated_seconds } => {
                let rate = self.total_pts_per_second();
                let pts = rate * simulated_seconds * self.current_frenzy_multiplier();
                self.sloth_points += pts;
                self.lifetime_sloth_points += pts;
                self.existential_stats.total_sloth_points_earned += pts;
                let mins = (simulated_seconds / 60.0).round() as u64;
                if mins > 0 {
                    format!("+{pts:.2} Puntos de Flojera ({mins} min de producción)")
                } else {
                    format!("+{pts:.2} Puntos de Flojera ({simulated_seconds:.0}s de producción)")
                }
            }
        };

        self.last_claimed_distraction = Some(ClaimedDistractionFeedback {
            title: active.config.title.to_string(),
            description: active.config.description.to_string(),
            effect_summary,
        });

        self.reset_distraction_spawn_timer();
        self.check_achievements();
        self.check_reveals();
        Some(active.config.reward)
    }

    /// Evaluates and applies offline progress deterministically in O(1) time
    /// using external Unix timestamps.
    ///
    /// If more than 60 seconds have elapsed since `last_save_timestamp`, computes
    /// accumulated points up to `max_offline_hours` at `offline_efficiency`, sets
    /// `active_view = ActiveView::WelcomeOfflineModal`, and returns an `OfflineProgressReport`.
    pub fn process_offline_progress(
        &mut self,
        current_timestamp: u64,
    ) -> Option<OfflineProgressReport> {
        if self.last_save_timestamp == 0 {
            self.last_save_timestamp = current_timestamp;
            return None;
        }

        let delta_secs = current_timestamp.saturating_sub(self.last_save_timestamp) as f64;
        self.last_save_timestamp = current_timestamp;

        if delta_secs > 60.0 {
            let rate = self.total_pts_per_second();
            let max_secs = self.persistence_config.max_offline_hours * 3600.0;
            let effective_secs = delta_secs.min(max_secs);
            let pts = rate * effective_secs * self.persistence_config.offline_efficiency;

            self.sloth_points += pts;
            self.lifetime_sloth_points += pts;
            self.existential_stats.total_sloth_points_earned += pts;

            let report = OfflineProgressReport {
                elapsed_seconds: delta_secs,
                offline_efficiency: self.persistence_config.offline_efficiency,
                points_earned: pts,
            };
            self.offline_report = Some(report.clone());
            self.set_view(ActiveView::WelcomeOfflineModal);
            self.check_reveals();
            Some(report)
        } else {
            None
        }
    }

    /// Checks all configured achievements and unlocks any whose conditions have been satisfied.
    /// Returns the list of newly unlocked achievement IDs.
    pub fn check_achievements(&mut self) -> Vec<&'static str> {
        let mut newly_unlocked = Vec::new();
        let num_turbo = self
            .activities
            .iter()
            .filter(|a| a.is_unlocked() && a.is_turbo())
            .count();

        for ach in &self.achievement_roster {
            if self.unlocked_achievements.contains(ach.id) {
                continue;
            }

            let condition_met = match ach.condition {
                AchievementCondition::TotalSlothPoints(req) => self.lifetime_sloth_points >= req,
                AchievementCondition::TotalPenClicks(req) => {
                    self.existential_stats.total_pen_clicks >= req
                }
                AchievementCondition::ReachLevel {
                    activity_index,
                    level,
                } => self
                    .activities
                    .get(activity_index)
                    .is_some_and(|a| a.level >= level),
                AchievementCondition::TotalPrestiges(req) => {
                    self.existential_stats.total_prestiges >= req
                }
                AchievementCondition::SimultaneousTurbo(req) => num_turbo >= req,
            };

            if condition_met {
                newly_unlocked.push(ach.id);
            }
        }

        for id in &newly_unlocked {
            self.unlocked_achievements.insert(id.to_string());
        }

        newly_unlocked
    }

    /// Attempts to buy 1 level of the unlocked activity with the lowest upgrade cost.
    pub fn buy_cheapest_unlocked_activity(&mut self) -> bool {
        let cheapest_index = self
            .activities
            .iter()
            .enumerate()
            .filter(|(_, a)| a.is_unlocked())
            .min_by(|(_, a), (_, b)| a.next_cost().total_cmp(&b.next_cost()))
            .map(|(i, _)| i);

        if let Some(index) = cheapest_index {
            self.upgrade_activity(index)
        } else {
            false
        }
    }

    /// Handles an explicit player action intent.
    pub fn handle_action(&mut self, action: Action) -> bool {
        match action {
            Action::UpgradeActivity(index) | Action::UnlockActivity(index) => {
                self.upgrade_activity(index)
            }
            Action::TriggerPrestige => self.trigger_prestige(),
            Action::BuyPermanentUpgrade(id) => self.buy_permanent_upgrade(&id),
            Action::OpenPrestigeDialog => {
                self.open_prestige_dialog();
                true
            }
            Action::ClosePrestigeDialog => {
                self.close_prestige_dialog();
                true
            }
            Action::ConfirmPrestige => self.confirm_prestige(),
            Action::OpenUpgradeMenu => {
                self.open_upgrade_menu();
                true
            }
            Action::CloseUpgradeMenu => {
                self.close_upgrade_menu();
                true
            }
            Action::ToggleUpgradeMenu => {
                self.toggle_upgrade_menu();
                true
            }
            Action::PenClick => {
                self.pen_click();
                true
            }
            Action::ClaimDistraction => self.claim_distraction().is_some(),
            Action::SetView(view) => {
                self.set_view(view);
                true
            }
            Action::CloseView => {
                self.set_view(ActiveView::MainDashboard);
                true
            }
        }
    }

    /// Advances the simulation by `dt` seconds deterministically.
    ///
    /// - Increments active play time in existential stats.
    /// - Decays temporary frenzy effects.
    /// - Advances active distraction decay or spawns new distraction if timer reaches zero.
    /// - Applies permanent upgrade effects.
    /// - If "autopilot" is owned, advances `autopilot_timer` and buys cheapest unlocked activity every 2.0s.
    /// - Progresses unlocked activities; completed cycles award:
    ///   `current_reward * cycles * prestige_multiplier() * achievements_multiplier() * frenzy_multiplier()`.
    /// - Checks achievements.
    pub fn tick(&mut self, dt: f64) {
        if !dt.is_finite() || dt <= 0.0 {
            return;
        }

        self.existential_stats.total_seconds_played += dt;

        self.apply_permanent_upgrade_effects();

        // Decay frenzy multiplier timer
        if self.frenzy_timer > 0.0 {
            self.frenzy_timer = (self.frenzy_timer - dt).max(0.0);
        }

        // Handle distraction decay or spawn
        if let Some(ref mut distraction) = self.active_distraction {
            distraction.time_remaining -= dt;
            if distraction.time_remaining <= 0.0 {
                self.active_distraction = None;
                self.reset_distraction_spawn_timer();
            }
        } else {
            self.distraction_spawn_timer -= dt;
            if self.distraction_spawn_timer <= 0.0 {
                if !self.distraction_config.roster.is_empty() {
                    let r = next_random_f64(&mut self.prng_seed);
                    let roster_len = self.distraction_config.roster.len();
                    let idx = ((r * roster_len as f64).floor() as usize).min(roster_len - 1);
                    let chosen = self.distraction_config.roster[idx].clone();
                    self.active_distraction = Some(ActiveDistractionState::new(chosen));
                }
                self.reset_distraction_spawn_timer();
            }
        }

        if self.has_permanent_upgrade("autopilot") {
            self.autopilot_timer += dt;
            while self.autopilot_timer >= 2.0 {
                self.autopilot_timer -= 2.0;
                self.buy_cheapest_unlocked_activity();
            }
        } else {
            self.autopilot_timer = 0.0;
        }

        let mult = self.total_multiplier();
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
                let gained = activity.current_reward() * cycles * mult;
                self.sloth_points += gained;
                self.lifetime_sloth_points += gained;
                self.existential_stats.total_sloth_points_earned += gained;
                self.existential_stats.total_bars_completed += cycles as u64;
            }

            // Ensure progress is clamped to prevent negative precision drift or overflow
            activity.progress = activity.progress.clamp(0.0, duration);
        }

        self.check_achievements();
        self.check_reveals();
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
            assert_eq!(
                activity.level, 0,
                "Activity at index {i} should be level 0 initially"
            );
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
            lore: "Test lore",
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

        // Level 500: 64.0x -> duration = 5.0 / 64.0
        state.activities[0].level = 500;
        assert_eq!(state.activities[0].speed_multiplier(), 64.0);
        assert_eq!(state.activities[0].current_duration(), 5.0 / 64.0);

        // Level 1000: 128.0x -> duration = 5.0 / 128.0 = 0.0390625s (Turbo mode active)
        state.activities[0].level = 1000;
        assert_eq!(state.activities[0].speed_multiplier(), 128.0);
        assert_eq!(state.activities[0].current_duration(), 5.0 / 128.0);
        assert!(state.activities[0].is_turbo());

        // Level 5000: 256.0x -> duration = 5.0 / 256.0 = 0.01953125s
        state.activities[0].level = 5000;
        assert_eq!(state.activities[0].speed_multiplier(), 256.0);
        assert_eq!(state.activities[0].current_duration(), 0.01953125);

        // Level 9999: 1024.0x -> (5.0 / 1024.0).max(0.005) = 0.005s (.max(0.005))
        state.activities[0].level = 9999;
        assert_eq!(state.activities[0].speed_multiplier(), 1024.0);
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
            lore: "Test lore",
            duration: 1.0,
            cost: 0.0,
            reward: 5.0,
            milestones: vec![Milestone {
                level: 10,
                speed_multiplier: 10.0, // duration becomes 0.1s <= 0.15s (Turbo)
            }],
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

        // Test action TriggerPrestige
        state.lifetime_sloth_points = 1000.0;
        assert_eq!(state.claimable_epiphanies(), 1);
        assert!(state.handle_action(Action::TriggerPrestige));
        assert_eq!(state.epiphanies, 1);

        // Test action BuyPermanentUpgrade
        assert!(!state.handle_action(Action::BuyPermanentUpgrade("muscle_memory".to_string())));
        state.epiphanies = 2;
        assert!(state.handle_action(Action::BuyPermanentUpgrade("muscle_memory".to_string())));
        assert!(state.has_permanent_upgrade("muscle_memory"));

        // Test dialog and menu actions
        assert!(!state.in_prestige_dialog);
        assert!(state.handle_action(Action::OpenPrestigeDialog));
        assert!(state.in_prestige_dialog);
        assert!(state.handle_action(Action::ClosePrestigeDialog));
        assert!(!state.in_prestige_dialog);

        assert!(!state.in_upgrade_menu);
        assert!(state.handle_action(Action::OpenUpgradeMenu));
        assert!(state.in_upgrade_menu);
        assert!(state.handle_action(Action::CloseUpgradeMenu));
        assert!(!state.in_upgrade_menu);

        assert!(state.handle_action(Action::ToggleUpgradeMenu));
        assert!(state.in_upgrade_menu);
        assert!(state.handle_action(Action::ToggleUpgradeMenu));
        assert!(!state.in_upgrade_menu);

        state.lifetime_sloth_points = 9000.0;
        state.open_prestige_dialog();
        assert!(state.in_prestige_dialog);
        assert!(state.handle_action(Action::ConfirmPrestige));
        assert!(!state.in_prestige_dialog);
        assert_eq!(state.epiphanies, 2);
    }

    #[test]
    fn test_prestige_dialog_and_menu_navigation() {
        let mut state = GameState::new();
        assert!(!state.in_prestige_dialog);
        assert!(!state.in_upgrade_menu);

        state.open_prestige_dialog();
        assert!(state.in_prestige_dialog);
        assert!(!state.in_upgrade_menu);

        state.open_upgrade_menu();
        assert!(!state.in_prestige_dialog);
        assert!(state.in_upgrade_menu);

        state.close_upgrade_menu();
        assert!(!state.in_upgrade_menu);

        state.open_prestige_dialog();
        state.cancel_prestige();
        assert!(!state.in_prestige_dialog);

        state.lifetime_sloth_points = 1000.0;
        state.open_prestige_dialog();
        assert!(state.confirm_prestige());
        assert!(!state.in_prestige_dialog);
        assert_eq!(state.epiphanies, 1);
    }

    #[test]
    fn test_claimable_epiphanies_math() {
        let mut state = GameState::new();
        // default config: base_cost = 1000.0, exponent = 0.50
        assert_eq!(state.claimable_epiphanies(), 0);

        state.lifetime_sloth_points = 999.0;
        assert_eq!(state.claimable_epiphanies(), 0);

        state.lifetime_sloth_points = 1000.0;
        assert_eq!(state.claimable_epiphanies(), 1);

        state.lifetime_sloth_points = 3999.0;
        assert_eq!(state.claimable_epiphanies(), 1);

        state.lifetime_sloth_points = 4000.0;
        assert_eq!(state.claimable_epiphanies(), 2);

        state.lifetime_sloth_points = 9000.0;
        assert_eq!(state.claimable_epiphanies(), 3);

        // Deduct already earned epiphanies
        state.total_epiphanies_earned = 2;
        assert_eq!(state.claimable_epiphanies(), 1);

        state.total_epiphanies_earned = 3;
        assert_eq!(state.claimable_epiphanies(), 0);

        state.total_epiphanies_earned = 5;
        assert_eq!(state.claimable_epiphanies(), 0);
    }

    #[test]
    fn test_prestige_multiplier() {
        let mut state = GameState::new();
        assert!((state.prestige_multiplier() - 1.0).abs() < EPSILON);

        state.epiphanies = 10;
        // Default bonus = 0.10: 1.0 + 10 * 0.10 = 2.0 (+100%)
        assert!((state.prestige_multiplier() - 2.0).abs() < EPSILON);

        // With zen_enlightenment bonus = 0.15: 1.0 + 10 * 0.15 = 2.5 (+150%)
        state
            .purchased_permanent_upgrades
            .insert("zen_enlightenment".to_string());
        assert!((state.prestige_multiplier() - 2.5).abs() < EPSILON);
    }

    #[test]
    fn test_buy_permanent_upgrade() {
        let mut state = GameState::new();
        state.epiphanies = 5;

        // muscle_memory costs 2
        assert!(!state.has_permanent_upgrade("muscle_memory"));
        assert!(state.buy_permanent_upgrade("muscle_memory"));
        assert!(state.has_permanent_upgrade("muscle_memory"));
        assert_eq!(state.epiphanies, 3);

        // Cannot buy already purchased upgrade
        assert!(!state.buy_permanent_upgrade("muscle_memory"));
        assert_eq!(state.epiphanies, 3);

        // Cannot buy upgrade without enough epiphanies (cost_optimization costs 5)
        assert!(!state.buy_permanent_upgrade("cost_optimization"));
        assert!(!state.has_permanent_upgrade("cost_optimization"));
        assert_eq!(state.epiphanies, 3);

        // Unknown upgrade id returns false
        assert!(!state.buy_permanent_upgrade("non_existent"));
    }

    #[test]
    fn test_trigger_prestige() {
        let mut state = GameState::new();
        state.sloth_points = 500.0;
        state.lifetime_sloth_points = 500.0;

        // Claimable is 0 (< 1000.0) -> trigger_prestige returns false and does nothing
        assert!(!state.trigger_prestige());
        assert!((state.sloth_points - 500.0).abs() < EPSILON);
        assert_eq!(state.epiphanies, 0);

        // Level up activity 0 and 1
        state.activities[0].level = 5;
        state.activities[0].progress = 2.0;
        state.activities[1].level = 3;
        state.activities[1].progress = 1.0;

        state.lifetime_sloth_points = 4000.0; // gives 2 epiphanies
        assert_eq!(state.claimable_epiphanies(), 2);

        assert!(state.trigger_prestige());
        assert_eq!(state.epiphanies, 2);
        assert_eq!(state.total_epiphanies_earned, 2);
        assert_eq!(state.sloth_points, 0.0);
        assert_eq!(state.lifetime_sloth_points, 4000.0);

        // Activity 0 resets to level 1, activity 1 resets to 0 (locked), all progress to 0.0
        assert_eq!(state.activities[0].level, 1);
        assert_eq!(state.activities[0].progress, 0.0);
        assert_eq!(state.activities[1].level, 0);
        assert_eq!(state.activities[1].progress, 0.0);
    }

    #[test]
    fn test_muscle_memory_effect() {
        let mut state = GameState::new();
        state.lifetime_sloth_points = 1000.0;
        state
            .purchased_permanent_upgrades
            .insert("muscle_memory".to_string());

        assert!(state.trigger_prestige());
        // Activity 0 should start at level 10 with muscle_memory
        assert_eq!(state.activities[0].level, 10);
        assert_eq!(state.activities[1].level, 0);
    }

    #[test]
    fn test_cost_optimization_effect() {
        let mut state = GameState::new();
        state.activities[0].level = 2;

        // Base cost is 1.0, level 2 cost without optimization: 1.0 * 1.15^2 = 1.3225
        assert!((state.activities[0].next_cost() - 1.3225).abs() < EPSILON);

        // Buy cost_optimization
        state.epiphanies = 10;
        assert!(state.buy_permanent_upgrade("cost_optimization"));

        // Level 2 cost with optimization: 1.0 * 1.12^2 = 1.2544
        assert_eq!(state.activities[0].cost_exponent, 1.12);
        assert!((state.activities[0].next_cost() - 1.2544).abs() < EPSILON);
    }

    #[test]
    fn test_eternal_sloth_effect() {
        let mut state = GameState::new();
        // Activity 0 duration is 5.0s
        assert_eq!(state.activities[0].current_duration(), 5.0);

        // Buy eternal_sloth (reduces duration by 20%, divide by 1.25 -> 5.0 / 1.25 = 4.0s)
        state.epiphanies = 20;
        assert!(state.buy_permanent_upgrade("eternal_sloth"));

        assert_eq!(state.activities[0].duration_divisor, 1.25);
        assert_eq!(state.activities[0].current_duration(), 4.0);
    }

    #[test]
    fn test_autopilot_effect() {
        let mut state = GameState::new();
        state.epiphanies = 50;
        assert!(state.buy_permanent_upgrade("autopilot"));

        // Activity 0 is level 1 (cost 1.15)
        state.sloth_points = 10.0;

        // Tick 1.0s: autopilot_timer = 1.0 (< 2.0), no purchase yet
        state.tick(1.0);
        assert!((state.autopilot_timer - 1.0).abs() < EPSILON);
        assert_eq!(state.activities[0].level, 1);

        // Tick another 1.0s: autopilot_timer reaches 2.0, triggers purchase of cheapest unlocked (activity 0)
        state.tick(1.0);
        assert_eq!(state.activities[0].level, 2);
        assert!((state.autopilot_timer - 0.0).abs() < EPSILON);
    }

    #[test]
    fn test_prestige_multiplies_rewards() {
        let mut state = GameState::new();
        state.epiphanies = 10; // multiplier = 1.0 + 10 * 0.10 = 2.0

        // Activity 0 takes 5.0s to complete, base reward = 1.0
        // When completed with 2.0x multiplier, awards 2.0 points to sloth_points and lifetime
        state.tick(5.0);
        assert!((state.sloth_points - 2.0).abs() < EPSILON);
        assert!((state.lifetime_sloth_points - 2.0).abs() < EPSILON);
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
        assert_traits::<PrestigeConfig>();
        assert_traits::<PermanentUpgradeConfig>();
        assert_traits::<Action>();
        assert_traits::<GameState>();
        assert_traits::<DistractionConfig>();
        assert_traits::<DistractionSystemConfig>();
        assert_traits::<ActiveDistractionState>();
        assert_traits::<OfflineProgressReport>();
        assert_traits::<PersistenceConfig>();
        assert_traits::<PenClickConfig>();
        assert_traits::<ExistentialStats>();
        assert_traits::<ProductiveComparison>();
        assert_traits::<AchievementConfig>();
        assert_traits::<ActiveView>();
        assert_traits::<ClaimedDistractionFeedback>();
    }

    #[test]
    fn test_distraction_spawn_and_decay() {
        let mut state = GameState::new();
        state.distraction_spawn_timer = 2.0;
        assert!(state.active_distraction.is_none());

        // Tick 1.0s: still counting down
        state.tick(1.0);
        assert!(state.active_distraction.is_none());
        assert!((state.distraction_spawn_timer - 1.0).abs() < EPSILON);

        // Tick 1.0s: reaches 0 -> spawns distraction
        state.tick(1.0);
        assert!(state.active_distraction.is_some());
        let initial_remaining = state.active_distraction.as_ref().unwrap().time_remaining;
        assert!(initial_remaining > 0.0);

        // Tick 1.0s: time remaining decreases
        state.tick(1.0);
        assert!(state.active_distraction.is_some());
        let new_remaining = state.active_distraction.as_ref().unwrap().time_remaining;
        assert!((new_remaining - (initial_remaining - 1.0)).abs() < EPSILON);

        // Advance past expiration: distraction decays and disappears
        state.tick(10.0);
        assert!(state.active_distraction.is_none());
    }

    #[test]
    fn test_distraction_claim_frenzy() {
        let mut state = GameState::new();
        state.active_distraction = Some(ActiveDistractionState::new(DistractionConfig {
            id: "test_frenzy",
            title: "Test Frenzy",
            description: "Test description",
            time_to_claim: 5.0,
            reward: DistractionRewardType::Frenzy {
                multiplier: 7.0,
                duration_secs: 25.0,
            },
        }));

        assert_eq!(state.current_frenzy_multiplier(), 1.0);
        let reward = state.claim_distraction();
        assert!(matches!(reward, Some(DistractionRewardType::Frenzy { .. })));
        assert!(state.active_distraction.is_none());
        assert_eq!(state.existential_stats.total_distractions_claimed, 1);
        assert_eq!(state.frenzy_multiplier, 7.0);
        assert_eq!(state.frenzy_timer, 25.0);
        assert_eq!(state.current_frenzy_multiplier(), 7.0);

        // During frenzy, activity 0 completes a 5.0s cycle and yields 1.0 * 7.0 = 7.0 pts
        state.tick(5.0);
        assert!((state.sloth_points - 7.0).abs() < EPSILON);
        assert!((state.frenzy_timer - 20.0).abs() < EPSILON);

        // After frenzy expires (tick 20.0s)
        state.tick(20.0);
        assert_eq!(state.frenzy_timer, 0.0);
        assert_eq!(state.current_frenzy_multiplier(), 1.0);
    }

    #[test]
    fn test_distraction_claim_instant_sloth_and_timewarp() {
        let mut state = GameState::new();
        state.sloth_points = 1000.0;
        state.active_distraction = Some(ActiveDistractionState::new(DistractionConfig {
            id: "test_sloth",
            title: "Meme",
            description: "Desc",
            time_to_claim: 5.0,
            reward: DistractionRewardType::InstantSloth {
                percentage_of_current: 0.20,
                min_flat: 50.0,
            },
        }));

        // 20% of 1000 = 200.0 (> min_flat 50.0)
        assert!(state.claim_distraction().is_some());
        assert!((state.sloth_points - 1200.0).abs() < EPSILON);

        // TimeWarp: Activity 0 generates 1.0 pt / 5.0s = 0.20 pts/s (scaled by achievements)
        let rate_before = state.total_pts_per_second();
        state.active_distraction = Some(ActiveDistractionState::new(DistractionConfig {
            id: "test_warp",
            title: "Warp",
            description: "Desc",
            time_to_claim: 5.0,
            reward: DistractionRewardType::TimeWarp {
                simulated_seconds: 90.0,
            },
        }));
        let points_before = state.sloth_points;
        assert!(state.claim_distraction().is_some());
        let expected_warp = rate_before * 90.0;
        assert!((state.sloth_points - (points_before + expected_warp)).abs() < EPSILON);
    }

    #[test]
    fn test_offline_progress_o1_calculation() {
        let mut state = GameState::new();
        // Activity 0: 1.0 pt / 5.0s = 0.20 pts/sec
        // Initial setup: timestamp at t=1000
        assert!(state.process_offline_progress(1000).is_none());
        assert_eq!(state.last_save_timestamp, 1000);

        // Under 60 seconds (delta = 45s): ignored
        let report_short = state.process_offline_progress(1045);
        assert!(report_short.is_none());
        assert_eq!(state.last_save_timestamp, 1045);
        assert_eq!(state.sloth_points, 0.0);

        // Delta = 3600s (1 hour away):
        // Production rate = 0.20 pts/s
        // Efficiency = 50% (0.50)
        // Expected points = 0.20 * 3600 * 0.50 = 360.0 pts
        let report = state.process_offline_progress(1045 + 3600);
        assert!(report.is_some());
        let rep = report.unwrap();
        assert_eq!(rep.elapsed_seconds, 3600.0);
        assert_eq!(rep.offline_efficiency, 0.50);
        assert!((rep.points_earned - 360.0).abs() < EPSILON);
        assert!((state.sloth_points - 360.0).abs() < EPSILON);
        assert_eq!(state.active_view, ActiveView::WelcomeOfflineModal);

        // Capping at max_offline_hours (12 hours = 43,200s):
        // 24 hours away (86,400s) should cap effective seconds at 43,200s
        let report_capped = state.process_offline_progress(state.last_save_timestamp + 86_400);
        assert!(report_capped.is_some());
        let rep_capped = report_capped.unwrap();
        assert_eq!(rep_capped.elapsed_seconds, 86_400.0);
        // Expected points: 0.20 * 43200 * 0.50 = 4320.0 pts
        assert!((rep_capped.points_earned - 4320.0).abs() < EPSILON);
    }

    #[test]
    fn test_serde_json_save_load_roundtrip() {
        let mut state = GameState::new();
        state.sloth_points = 543.21;
        state.lifetime_sloth_points = 1234.56;
        state.epiphanies = 3;
        state.existential_stats.total_pen_clicks = 42;
        state.activities[0].level = 5;
        state.unlocked_achievements.insert("first_drop".to_string());
        state.last_save_timestamp = 1700000000;

        let json = serde_json::to_string(&state).expect("Serialization failed");
        let loaded: GameState = serde_json::from_str(&json).expect("Deserialization failed");

        assert!((loaded.sloth_points - 543.21).abs() < EPSILON);
        assert!((loaded.lifetime_sloth_points - 1234.56).abs() < EPSILON);
        assert_eq!(loaded.epiphanies, 3);
        assert_eq!(loaded.existential_stats.total_pen_clicks, 42);
        assert_eq!(loaded.activities[0].level, 5);
        assert!(loaded.unlocked_achievements.contains("first_drop"));
        assert_eq!(loaded.last_save_timestamp, 1700000000);
    }

    #[test]
    fn test_pen_click_scaling_and_speedup() {
        let mut state = GameState::new();
        state.lifetime_sloth_points = 10_000.0;
        // base = 0.25, factor = 0.0001 -> scaling = 1.0 + 10000 * 0.0001 = 2.0 -> earned = 0.25 * 2.0 = 0.50
        assert_eq!(state.activities[0].progress, 0.0);
        assert_eq!(state.activities[0].current_duration(), 5.0);

        let (earned, sound) = state.pen_click();
        assert!((earned - 0.50).abs() < EPSILON);
        assert_eq!(state.existential_stats.total_pen_clicks, 1);
        assert_eq!(sound, "*¡Tac!*");

        // Nearest activity (activity 0) progressed by 2% of 5.0s = 0.10s
        assert!((state.activities[0].progress - 0.10).abs() < EPSILON);

        // Next click cycles onomatopoeia to "*¡Clic!*"
        let (_, sound2) = state.pen_click();
        assert_eq!(sound2, "*¡Clic!*");
        assert_eq!(state.existential_stats.total_pen_clicks, 2);
        assert!((state.activities[0].progress - 0.20).abs() < EPSILON);
    }

    #[test]
    fn test_existential_stats_tracking() {
        let mut state = GameState::new();
        state.tick(12.5);
        assert!((state.existential_stats.total_seconds_played - 12.5).abs() < EPSILON);

        // 12.5s on activity 0 (duration 5.0s) completes 2 full cycles
        assert_eq!(state.existential_stats.total_bars_completed, 2);

        // Productive comparisons list
        let comparisons = default_productive_comparisons();
        assert_eq!(comparisons.len(), 6);
        assert_eq!(comparisons[0].required_seconds, 60.0);
        assert_eq!(comparisons[0].activity_name, "Tomar un vaso con agua");
    }

    #[test]
    fn test_achievements_unlock_and_passive_multiplier() {
        let mut state = GameState::new();
        assert_eq!(state.achievements_multiplier(), 1.0);
        assert!(state.unlocked_achievements.is_empty());

        // Accumulate 10 sloth points -> unlocks "first_drop" (+1.5%)
        state.sloth_points = 10.0;
        state.lifetime_sloth_points = 10.0;
        let unlocked = state.check_achievements();
        assert!(unlocked.contains(&"first_drop"));
        assert!(state.unlocked_achievements.contains("first_drop"));
        assert!((state.achievements_multiplier() - 1.015).abs() < EPSILON);

        // Perform 100 pen clicks -> unlocks "pen_maniac" (+1.5% -> 1.030)
        state.existential_stats.total_pen_clicks = 100;
        let unlocked2 = state.check_achievements();
        assert!(unlocked2.contains(&"pen_maniac"));
        assert!((state.achievements_multiplier() - 1.030).abs() < EPSILON);

        // 3 activities in turbo mode simultaneously -> unlocks "all_turbo"
        state.activities[0].level = 1000;
        state.activities[1].level = 1000;
        state.activities[2].level = 5000;
        assert!(state.activities[0].is_turbo());
        assert!(state.activities[1].is_turbo());
        assert!(state.activities[2].is_turbo());
        let unlocked3 = state.check_achievements();
        assert!(unlocked3.contains(&"all_turbo"));
        assert!(unlocked3.contains(&"stare_master"));
        // 4 achievements unlocked (first_drop, pen_maniac, stare_master, all_turbo): 1.0 + 4 * 0.015 = 1.060
        assert!((state.achievements_multiplier() - 1.060).abs() < EPSILON);
    }

    #[test]
    fn test_active_view_navigation() {
        let mut state = GameState::new();
        assert_eq!(state.active_view, ActiveView::MainDashboard);

        assert!(state.handle_action(Action::SetView(ActiveView::ExistentialStats)));
        assert_eq!(state.active_view, ActiveView::ExistentialStats);

        assert!(state.handle_action(Action::CloseView));
        assert_eq!(state.active_view, ActiveView::MainDashboard);

        assert!(state.handle_action(Action::SetView(ActiveView::AchievementsGallery)));
        assert_eq!(state.active_view, ActiveView::AchievementsGallery);

        assert!(state.handle_action(Action::PenClick));
        assert_eq!(state.existential_stats.total_pen_clicks, 1);
    }

    #[test]
    fn test_progressive_activity_revelation() {
        let mut state = GameState::new();
        // Initially, only activity 0 is revealed
        assert!(state.activities[0].is_revealed);
        for i in 1..state.activities.len() {
            assert!(
                !state.activities[i].is_revealed,
                "Activity {i} should be hidden initially"
            );
        }

        // Cannot upgrade locked & hidden activity 1 even if key is pressed
        assert!(!state.upgrade_activity(1));

        // Earn 4.9 points (cost is 5.0) -> still hidden
        state.sloth_points = 4.9;
        state.check_reveals();
        assert!(!state.activities[1].is_revealed);

        // Earn 5.0 points -> activity 1 is revealed!
        state.sloth_points = 5.0;
        state.check_reveals();
        assert!(state.activities[1].is_revealed);
        assert!(!state.activities[2].is_revealed);

        // Spend all points down to 0 -> activity 1 STAYS revealed
        state.sloth_points = 0.0;
        state.check_reveals();
        assert!(state.activities[1].is_revealed);

        // Can now buy activity 1 once points are re-accumulated
        state.sloth_points = 5.0;
        assert!(state.upgrade_activity(1));
        assert_eq!(state.activities[1].level, 1);

        // Now next to reveal is activity 2 (cost 25.0)
        state.sloth_points = 25.0;
        state.check_reveals();
        assert!(state.activities[2].is_revealed);
    }

    #[test]
    fn test_prestige_and_upgrade_revelation_gating() {
        let mut state = GameState::new();
        // Initially, prestige is NOT revealed
        assert!(!state.prestige_revealed);
        assert!(state.revealed_permanent_upgrades.is_empty());

        // Accumulate 1000 points (1 Epiphany claimable, but min permanent upgrade costs 2)
        state.lifetime_sloth_points = 1000.0;
        assert_eq!(state.claimable_epiphanies(), 1);
        state.check_reveals();
        assert!(
            !state.prestige_revealed,
            "Prestige should not reveal with only 1 claimable Epiphany"
        );

        // Accumulate 4000 points (2 Epiphanies claimable, enough for muscle_memory)
        state.lifetime_sloth_points = 4000.0;
        assert_eq!(state.claimable_epiphanies(), 2);
        state.check_reveals();
        assert!(
            state.prestige_revealed,
            "Prestige should reveal once 2 Epiphanies can be claimed"
        );

        // Trigger prestige
        assert!(state.trigger_prestige());
        assert_eq!(state.epiphanies, 2);
        // Prestige remains revealed forever
        assert!(state.prestige_revealed);

        // With 2 Epiphanies, only muscle_memory (cost 2) is revealed
        assert!(state.is_permanent_upgrade_revealed("muscle_memory"));
        assert!(!state.is_permanent_upgrade_revealed("cost_optimization")); // costs 5
        assert!(!state.is_permanent_upgrade_revealed("zen_enlightenment")); // costs 10

        // Buy muscle_memory -> Epiphanies drop to 0, but muscle_memory STAYS revealed
        assert!(state.buy_permanent_upgrade("muscle_memory"));
        assert_eq!(state.epiphanies, 0);
        assert!(state.has_permanent_upgrade("muscle_memory"));
        assert!(state.is_permanent_upgrade_revealed("muscle_memory"));
    }

    #[test]
    fn test_distraction_feedback_and_frenzy_bar_duration() {
        let mut state = GameState::new();
        assert!(state.last_claimed_distraction.is_none());
        assert_eq!(state.frenzy_max_duration, 0.0);

        // Claim Frenzy
        state.active_distraction = Some(ActiveDistractionState::new(DistractionConfig::frenzy(
            "test_frenzy",
            "Frenesí Test",
            "Lore",
            5.0,
            7.0,
            25.0,
        )));
        assert!(state.claim_distraction().is_some());
        assert_eq!(state.frenzy_multiplier, 7.0);
        assert_eq!(state.frenzy_timer, 25.0);
        assert_eq!(state.frenzy_max_duration, 25.0);

        let feedback = state.last_claimed_distraction.as_ref().unwrap();
        assert_eq!(feedback.title, "Frenesí Test");
        assert!(feedback.effect_summary.contains("Frenesí x7.0 por 25s"));

        // Claim TimeWarp (Pereza Instantánea: 900s)
        state.activities[0].level = 5;
        state.active_distraction = Some(ActiveDistractionState::new(DistractionConfig::time_warp(
            "pereza_instantanea",
            "Pereza Instantánea",
            "Lore",
            8.0,
            900.0,
        )));
        assert!(state.claim_distraction().is_some());
        let warp_feedback = state.last_claimed_distraction.as_ref().unwrap();
        assert_eq!(warp_feedback.title, "Pereza Instantánea");
        assert!(
            warp_feedback
                .effect_summary
                .contains("15 min de producción")
        );
        assert!(warp_feedback.effect_summary.contains("Puntos de Flojera"));
    }
}
