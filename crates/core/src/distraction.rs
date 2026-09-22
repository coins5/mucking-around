#![forbid(unsafe_code)]

use serde::{Deserialize, Deserializer, Serialize};

/// Type of reward awarded when claiming an unexpected distraction.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DistractionRewardType {
    /// Multiplies all production for a specific duration in seconds.
    Frenzy { multiplier: f64, duration_secs: f64 },
    /// Grants a percentage of current balance or a minimum flat amount (whichever is greater).
    InstantSloth { percentage_of_current: f64, min_flat: f64 },
    /// Simulates the instantaneous passage of N seconds of production across active bars.
    TimeWarp { simulated_seconds: f64 },
}

/// Static configuration for an unexpected distraction event.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct DistractionConfig {
    /// Unique identifier for this distraction event.
    pub id: &'static str,
    /// Alert / Display title.
    pub title: &'static str,
    /// Flavor lore / humor description.
    pub description: &'static str,
    /// Window of opportunity in seconds before the distraction decays away.
    pub time_to_claim: f64,
    /// The reward awarded upon claiming.
    pub reward: DistractionRewardType,
}

impl<'de> Deserialize<'de> for DistractionConfig {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct ConfigHelper {
            id: String,
            title: String,
            description: String,
            time_to_claim: f64,
            reward: DistractionRewardType,
        }

        let helper = ConfigHelper::deserialize(deserializer)?;
        let (id, title, description) = match helper.id.as_str() {
            "medieval_pan" => (
                "medieval_pan",
                "Video de Restauración",
                "Te apareció un video de 45 min sobre cómo restaurar una sartén de hierro fundido.",
            ),
            "whatsapp_meme" => (
                "whatsapp_meme",
                "Meme del Grupo",
                "Tu amigo mandó un meme de gatos al grupo. Es obligatorio reaccionar.",
            ),
            "bread_quiz" => (
                "bread_quiz",
                "Test de Personalidad",
                "Descubre qué tipo de pan dulce eres según tu signo zodiacal.",
            ),
            "zillow_dream" => (
                "zillow_dream",
                "Casas Inalcanzables",
                "Mirando departamentos de 2 millones de dólares que jamás podrás pagar.",
            ),
            _ => (
                Box::leak(helper.id.into_boxed_str()) as &'static str,
                Box::leak(helper.title.into_boxed_str()) as &'static str,
                Box::leak(helper.description.into_boxed_str()) as &'static str,
            ),
        };

        Ok(DistractionConfig {
            id,
            title,
            description,
            time_to_claim: helper.time_to_claim,
            reward: helper.reward,
        })
    }
}

/// Dynamic active state for a distraction currently ticking down on screen.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ActiveDistractionState {
    pub config: DistractionConfig,
    pub time_remaining: f64,
}

impl ActiveDistractionState {
    #[must_use]
    pub fn new(config: DistractionConfig) -> Self {
        let time_remaining = config.time_to_claim;
        Self {
            config,
            time_remaining,
        }
    }

    /// Ratio remaining from 1.0 down to 0.0.
    #[must_use]
    pub fn progress_ratio(&self) -> f64 {
        if !self.config.time_to_claim.is_finite() || self.config.time_to_claim <= 0.0 {
            0.0
        } else {
            (self.time_remaining / self.config.time_to_claim).clamp(0.0, 1.0)
        }
    }
}

/// Global system configuration for random distraction spawning.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DistractionSystemConfig {
    /// Minimum time in seconds between distraction spawns (e.g. 60.0s).
    pub min_spawn_interval: f64,
    /// Maximum time in seconds between distraction spawns (e.g. 180.0s).
    pub max_spawn_interval: f64,
    /// The pool of distraction events that can be spawned.
    pub roster: Vec<DistractionConfig>,
}

impl Default for DistractionSystemConfig {
    fn default() -> Self {
        Self {
            min_spawn_interval: 60.0,
            max_spawn_interval: 180.0,
            roster: default_distractions(),
        }
    }
}

/// Returns the standard default roster of distraction events.
#[must_use]
pub fn default_distractions() -> Vec<DistractionConfig> {
    vec![
        DistractionConfig {
            id: "medieval_pan",
            title: "Video de Restauración",
            description: "Te apareció un video de 45 min sobre cómo restaurar una sartén de hierro fundido.",
            time_to_claim: 7.0,
            reward: DistractionRewardType::Frenzy {
                multiplier: 7.0,
                duration_secs: 25.0,
            },
        },
        DistractionConfig {
            id: "whatsapp_meme",
            title: "Meme del Grupo",
            description: "Tu amigo mandó un meme de gatos al grupo. Es obligatorio reaccionar.",
            time_to_claim: 6.0,
            reward: DistractionRewardType::InstantSloth {
                percentage_of_current: 0.20,
                min_flat: 50.0,
            },
        },
        DistractionConfig {
            id: "bread_quiz",
            title: "Test de Personalidad",
            description: "Descubre qué tipo de pan dulce eres según tu signo zodiacal.",
            time_to_claim: 8.0,
            reward: DistractionRewardType::TimeWarp {
                simulated_seconds: 90.0,
            },
        },
        DistractionConfig {
            id: "zillow_dream",
            title: "Casas Inalcanzables",
            description: "Mirando departamentos de 2 millones de dólares que jamás podrás pagar.",
            time_to_claim: 5.0,
            reward: DistractionRewardType::Frenzy {
                multiplier: 15.0,
                duration_secs: 10.0,
            },
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_distractions_roster() {
        let roster = default_distractions();
        assert_eq!(roster.len(), 4);
        assert_eq!(roster[0].id, "medieval_pan");
        assert_eq!(roster[1].id, "whatsapp_meme");
        assert_eq!(roster[2].id, "bread_quiz");
        assert_eq!(roster[3].id, "zillow_dream");
    }

    #[test]
    fn test_active_distraction_ratio() {
        let config = default_distractions()[0].clone();
        let mut active = ActiveDistractionState::new(config);
        assert!((active.progress_ratio() - 1.0).abs() < 1e-9);

        active.time_remaining = 3.5;
        assert!((active.progress_ratio() - 0.5).abs() < 1e-9);

        active.time_remaining = 0.0;
        assert!((active.progress_ratio() - 0.0).abs() < 1e-9);
    }
}
