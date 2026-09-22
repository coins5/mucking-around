#![forbid(unsafe_code)]

use serde::{Deserialize, Deserializer, Serialize};

/// Configurable speed milestone reached at a specific activity level.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Milestone {
    pub level: u32,
    pub speed_multiplier: f64,
}

/// Returns the standard default milestones for an activity.
#[must_use]
pub fn default_milestones() -> Vec<Milestone> {
    vec![
        Milestone {
            level: 25,
            speed_multiplier: 2.0,
        },
        Milestone {
            level: 50,
            speed_multiplier: 4.0,
        },
        Milestone {
            level: 100,
            speed_multiplier: 8.0,
        },
        Milestone {
            level: 200,
            speed_multiplier: 16.0,
        },

        Milestone {
            level: 300,
            speed_multiplier: 24.0,
        },
        Milestone {
            level: 400,
            speed_multiplier: 32.0,
        },

        Milestone {
            level: 500,
            speed_multiplier: 64.0,
        },

        Milestone {
            level: 1000,
            speed_multiplier: 128.0,
        },
        Milestone {
            level: 5000,
            speed_multiplier: 256.0,
        },
        Milestone {
            level: 9999,
            speed_multiplier: 1024.0,
        },
    ]
}

/// Static configuration for a procrastination activity.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ActivityConfig {
    /// Unique identifier for the activity.
    pub id: &'static str,
    /// Human-readable display name.
    pub name: &'static str,
    /// Flavor text / lore describing this procrastination activity.
    pub lore: &'static str,
    /// Cycle duration in seconds.
    pub duration: f64,
    /// Cost in Sloth Points to unlock (0.0 if initially unlocked).
    pub cost: f64,
    /// Sloth Points awarded upon cycle completion.
    pub reward: f64,
    /// Configurable speed milestones for this activity.
    pub milestones: Vec<Milestone>,
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
            #[serde(default)]
            lore: Option<String>,
            duration: f64,
            cost: f64,
            reward: f64,
            #[serde(default = "default_milestones")]
            milestones: Vec<Milestone>,
        }

        let helper = ConfigHelper::deserialize(deserializer)?;
        let (id, name, lore) = match helper.id.as_str() {
            "wait_bar" => (
                "wait_bar",
                "Esperar a que cargue la barrita",
                "La vida se mide en barras de carga que sospechosamente se quedan en 99%.",
            ),
            "stare_void" => (
                "stare_void",
                "Mirar a la nada fijamente",
                "Si miras fijamente a la nada, la nada te exige que te pongas a trabajar.",
            ),
            "doomscroll" => (
                "doomscroll",
                "Hacer scroll infinito sin ver nada",
                "Solo cinco minutitos más... susurró hace cuatro horas y media.",
            ),
            "check_fridge" => (
                "check_fridge",
                "Abrir la refri vacía por quinta vez",
                "Quizás apareció una pizza por generación espontánea en los últimos 3 minutos.",
            ),
            "clean_desk" => (
                "clean_desk",
                "Ordenar el escritorio para no trabajar",
                "Increíble cómo organizar cables se vuelve prioridad cuando hay pendientes.",
            ),
            _ => (
                Box::leak(helper.id.into_boxed_str()) as &'static str,
                Box::leak(helper.name.into_boxed_str()) as &'static str,
                match helper.lore {
                    Some(s) => Box::leak(s.into_boxed_str()) as &'static str,
                    None => "",
                },
            ),
        };

        Ok(ActivityConfig {
            id,
            name,
            lore,
            duration: helper.duration,
            cost: helper.cost,
            reward: helper.reward,
            milestones: helper.milestones,
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
            lore: "La vida se mide en barras de carga que sospechosamente se quedan en 99%.",
            duration: 5.0,
            cost: 0.0,
            reward: 1.0,
            milestones: default_milestones(),
        },
        ActivityConfig {
            id: "stare_void",
            name: "Mirar a la nada fijamente",
            lore: "Si miras fijamente a la nada, la nada te exige que te pongas a trabajar.",
            duration: 12.5,
            cost: 5.0,
            reward: 3.5,
            milestones: default_milestones(),
        },
        ActivityConfig {
            id: "doomscroll",
            name: "Hacer scroll infinito sin ver nada",
            lore: "Solo cinco minutitos más... susurró hace cuatro horas y media.",
            duration: 25.0,
            cost: 25.0,
            reward: 10.0,
            milestones: default_milestones(),
        },
        ActivityConfig {
            id: "check_fridge",
            name: "Abrir la refri vacía por quinta vez",
            lore: "Quizás apareció una pizza por generación espontánea en los últimos 3 minutos.",
            duration: 60.0,
            cost: 100.0,
            reward: 36.0,
            milestones: default_milestones(),
        },
        ActivityConfig {
            id: "clean_desk",
            name: "Ordenar el escritorio para no trabajar",
            lore: "Increíble cómo organizar cables se vuelve prioridad cuando hay pendientes.",
            duration: 120.0,
            cost: 350.0,
            reward: 120.0,
            milestones: default_milestones(),
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
        assert_eq!(
            roster[0].lore,
            "La vida se mide en barras de carga que sospechosamente se quedan en 99%."
        );

        assert_eq!(roster[1].id, "stare_void");
        assert_eq!(roster[1].cost, 5.0);
        assert_eq!(
            roster[1].lore,
            "Si miras fijamente a la nada, la nada te exige que te pongas a trabajar."
        );

        assert_eq!(roster[2].id, "doomscroll");
        assert_eq!(roster[2].cost, 25.0);
        assert_eq!(
            roster[2].lore,
            "Solo cinco minutitos más... susurró hace cuatro horas y media."
        );

        assert_eq!(roster[3].id, "check_fridge");
        assert_eq!(roster[3].cost, 100.0);
        assert_eq!(
            roster[3].lore,
            "Quizás apareció una pizza por generación espontánea en los últimos 3 minutos."
        );

        assert_eq!(roster[4].id, "clean_desk");
        assert_eq!(roster[4].cost, 350.0);
        assert_eq!(
            roster[4].lore,
            "Increíble cómo organizar cables se vuelve prioridad cuando hay pendientes."
        );
    }

    #[test]
    fn test_default_milestones() {
        let milestones = default_milestones();
        assert_eq!(milestones.len(), 10);
        assert_eq!(milestones[0].level, 25);
        assert_eq!(milestones[0].speed_multiplier, 2.0);
        assert_eq!(milestones[1].level, 50);
        assert_eq!(milestones[1].speed_multiplier, 4.0);
        assert_eq!(milestones[2].level, 100);
        assert_eq!(milestones[2].speed_multiplier, 8.0);
        assert_eq!(milestones[3].level, 200);
        assert_eq!(milestones[3].speed_multiplier, 16.0);
        assert_eq!(milestones[4].level, 300);
        assert_eq!(milestones[4].speed_multiplier, 24.0);
        assert_eq!(milestones[5].level, 400);
        assert_eq!(milestones[5].speed_multiplier, 32.0);
        assert_eq!(milestones[6].level, 500);
        assert_eq!(milestones[6].speed_multiplier, 64.0);
        assert_eq!(milestones[7].level, 1000);
        assert_eq!(milestones[7].speed_multiplier, 128.0);
        assert_eq!(milestones[8].level, 5000);
        assert_eq!(milestones[8].speed_multiplier, 256.0);
        assert_eq!(milestones[9].level, 9999);
        assert_eq!(milestones[9].speed_multiplier, 1024.0);
    }
}
