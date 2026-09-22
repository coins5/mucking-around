#![forbid(unsafe_code)]

use serde::{Deserialize, Deserializer, Serialize};

/// Static configuration for a procrastination activity.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct ActivityConfig {
    /// Unique identifier for the activity.
    pub id: &'static str,
    /// Human-readable display name.
    pub name: &'static str,
    /// Cycle duration in seconds.
    pub duration: f64,
    /// Cost in Sloth Points to unlock (0.0 if initially unlocked).
    pub cost: f64,
    /// Sloth Points awarded upon cycle completion.
    pub reward: f64,
}

impl<'de> Deserialize<'de> for ActivityConfig {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct ConfigHelper {
            id: String,
            name: String,
            duration: f64,
            cost: f64,
            reward: f64,
        }

        let helper = ConfigHelper::deserialize(deserializer)?;
        let id = match helper.id.as_str() {
            "wait_bar" => "wait_bar",
            "stare_void" => "stare_void",
            "doomscroll" => "doomscroll",
            "check_fridge" => "check_fridge",
            "clean_desk" => "clean_desk",
            _ => Box::leak(helper.id.into_boxed_str()),
        };
        let name = match helper.name.as_str() {
            "Esperar a que cargue la barrita" => "Esperar a que cargue la barrita",
            "Mirar a la nada fijamente" => "Mirar a la nada fijamente",
            "Hacer scroll infinito sin ver nada" => "Hacer scroll infinito sin ver nada",
            "Abrir la refri vacía por quinta vez" => "Abrir la refri vacía por quinta vez",
            "Ordenar el escritorio para no trabajar" => "Ordenar el escritorio para no trabajar",
            _ => Box::leak(helper.name.into_boxed_str()),
        };

        Ok(ActivityConfig {
            id,
            name,
            duration: helper.duration,
            cost: helper.cost,
            reward: helper.reward,
        })
    }
}

/// Returns the default roster of procrastination activities.
#[must_use]
pub fn default_roster() -> Vec<ActivityConfig> {
    vec![
        ActivityConfig {
            id: "wait_bar",
            name: "Esperar a que cargue la barrita",
            duration: 5.0,
            cost: 0.0,
            reward: 1.0,
        },
        ActivityConfig {
            id: "stare_void",
            name: "Mirar a la nada fijamente",
            duration: 12.5,
            cost: 5.0,
            reward: 3.5,
        },
        ActivityConfig {
            id: "doomscroll",
            name: "Hacer scroll infinito sin ver nada",
            duration: 25.0,
            cost: 25.0,
            reward: 10.0,
        },
        ActivityConfig {
            id: "check_fridge",
            name: "Abrir la refri vacía por quinta vez",
            duration: 60.0,
            cost: 100.0,
            reward: 36.0,
        },
        ActivityConfig {
            id: "clean_desk",
            name: "Ordenar el escritorio para no trabajar",
            duration: 120.0,
            cost: 350.0,
            reward: 120.0,
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_roster_count_and_order() {
        let roster = default_roster();
        assert_eq!(roster.len(), 5);

        assert_eq!(roster[0].id, "wait_bar");
        assert_eq!(roster[0].cost, 0.0);
        assert_eq!(roster[0].duration, 5.0);
        assert_eq!(roster[0].reward, 1.0);

        assert_eq!(roster[1].id, "stare_void");
        assert_eq!(roster[1].cost, 5.0);

        assert_eq!(roster[2].id, "doomscroll");
        assert_eq!(roster[2].cost, 25.0);

        assert_eq!(roster[3].id, "check_fridge");
        assert_eq!(roster[3].cost, 100.0);

        assert_eq!(roster[4].id, "clean_desk");
        assert_eq!(roster[4].cost, 350.0);
    }
}
