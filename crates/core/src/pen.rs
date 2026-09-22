#![forbid(unsafe_code)]

use serde::{Deserialize, Deserializer, Serialize};

/// Configuration for the active pen click habit mechanic.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct PenClickConfig {
    /// Base points awarded per manual click (e.g. 0.25).
    pub base_reward: f64,
    /// Multiplier scaling based on historic lifetime sloth points (e.g. 0.0001).
    pub lifetime_scaling_factor: f64,
    /// Percentage progression speedup granted to the nearest completing bar (e.g. 0.02 = 2%).
    pub bar_speedup_percentage: f64,
    /// Comic rotating onomatopoeias.
    pub sound_effects: Vec<&'static str>,
}

impl Default for PenClickConfig {
    fn default() -> Self {
        Self {
            base_reward: 0.25,
            lifetime_scaling_factor: 0.0001,
            bar_speedup_percentage: 0.02,
            sound_effects: vec![
                "*¡Tac!*",
                "*¡Clic!*",
                "*¡Tac-tac-tac!*",
                "*¡Crack! (casi rompes el resorte)*",
            ],
        }
    }
}

impl<'de> Deserialize<'de> for PenClickConfig {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct ConfigHelper {
            base_reward: f64,
            lifetime_scaling_factor: f64,
            bar_speedup_percentage: f64,
            #[serde(default = "default_sound_effects")]
            sound_effects: Vec<String>,
        }

        fn default_sound_effects() -> Vec<String> {
            vec![
                "*¡Tac!*".to_string(),
                "*¡Clic!*".to_string(),
                "*¡Tac-tac-tac!*".to_string(),
                "*¡Crack! (casi rompes el resorte)*".to_string(),
            ]
        }

        let helper = ConfigHelper::deserialize(deserializer)?;
        let sound_effects = helper
            .sound_effects
            .into_iter()
            .map(|s| match s.as_str() {
                "*¡Tac!*" => "*¡Tac!*",
                "*¡Clic!*" => "*¡Clic!*",
                "*¡Tac-tac-tac!*" => "*¡Tac-tac-tac!*",
                "*¡Crack! (casi rompes el resorte)*" => "*¡Crack! (casi rompes el resorte)*",
                _ => Box::leak(s.into_boxed_str()) as &'static str,
            })
            .collect();

        Ok(PenClickConfig {
            base_reward: helper.base_reward,
            lifetime_scaling_factor: helper.lifetime_scaling_factor,
            bar_speedup_percentage: helper.bar_speedup_percentage,
            sound_effects,
        })
    }
}
