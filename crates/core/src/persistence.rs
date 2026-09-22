#![forbid(unsafe_code)]

use serde::{Deserialize, Deserializer, Serialize};

/// Configuration for game state persistence and offline simulation.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct PersistenceConfig {
    /// Interval in seconds between background auto-saves (e.g. 30.0s).
    pub auto_save_interval: f64,
    /// Percentage efficiency applied to offline progress calculations (e.g. 0.50 = 50%).
    pub offline_efficiency: f64,
    /// Maximum hours of offline progress that can accumulate (e.g. 12.0h).
    pub max_offline_hours: f64,
    /// Standard file name for game state persistence (e.g. "save.json").
    pub save_file_name: &'static str,
}

impl Default for PersistenceConfig {
    fn default() -> Self {
        Self {
            auto_save_interval: 30.0,
            offline_efficiency: 0.50,
            max_offline_hours: 12.0,
            save_file_name: "save.json",
        }
    }
}

impl<'de> Deserialize<'de> for PersistenceConfig {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct ConfigHelper {
            #[serde(default = "default_auto_save_interval")]
            auto_save_interval: f64,
            #[serde(default = "default_offline_efficiency")]
            offline_efficiency: f64,
            #[serde(default = "default_max_offline_hours")]
            max_offline_hours: f64,
            #[serde(default = "default_save_file_name_string")]
            save_file_name: String,
        }

        fn default_auto_save_interval() -> f64 {
            30.0
        }
        fn default_offline_efficiency() -> f64 {
            0.50
        }
        fn default_max_offline_hours() -> f64 {
            12.0
        }
        fn default_save_file_name_string() -> String {
            "save.json".to_string()
        }

        let helper = ConfigHelper::deserialize(deserializer)?;
        let save_file_name = if helper.save_file_name == "save.json" {
            "save.json"
        } else {
            Box::leak(helper.save_file_name.into_boxed_str()) as &'static str
        };

        Ok(PersistenceConfig {
            auto_save_interval: helper.auto_save_interval,
            offline_efficiency: helper.offline_efficiency,
            max_offline_hours: helper.max_offline_hours,
            save_file_name,
        })
    }
}

/// Summary report generated when offline progress has been evaluated.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OfflineProgressReport {
    /// Number of real seconds the player was away.
    pub elapsed_seconds: f64,
    /// Ratio of efficiency applied (e.g. 0.50).
    pub offline_efficiency: f64,
    /// Total Sloth Points awarded for the absence.
    pub points_earned: f64,
}
