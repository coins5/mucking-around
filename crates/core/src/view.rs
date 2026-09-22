#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};

/// Active visual modal/screen state within the terminal application.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum ActiveView {
    /// Main dashboard with activity progress bars, pen click status, and distractions.
    #[default]
    MainDashboard,
    /// Confirmation dialog for triggering an existential crisis (prestige).
    PrestigeDialog,
    /// Zen Enlightenment permanent upgrade store [U].
    PermanentUpgradesShop,
    /// Existential statistics and real-world productive comparisons [S].
    ExistentialStats,
    /// Achievements gallery and passive multipliers [A].
    AchievementsGallery,
    /// Welcome modal displaying offline progress accumulated while away.
    WelcomeOfflineModal,
}
