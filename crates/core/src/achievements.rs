#![forbid(unsafe_code)]

use serde::{Deserialize, Deserializer, Serialize};

/// Condition required to unlock an achievement.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AchievementCondition {
    /// Total lifetime Sloth Points earned reaches this threshold.
    TotalSlothPoints(f64),
    /// Total manual pen clicks performed reaches this threshold.
    TotalPenClicks(u64),
    /// Activity at the specified roster index reaches this level.
    ReachLevel { activity_index: usize, level: u32 },
    /// Total existential crises (prestiges) triggered reaches this count.
    TotalPrestiges(u32),
    /// Having at least N activities concurrently running in turbo mode.
    SimultaneousTurbo(usize),
}

/// Static configuration for an unlockable achievement.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct AchievementConfig {
    /// Unique identifier for the achievement.
    pub id: &'static str,
    /// Human-readable achievement title.
    pub name: &'static str,
    /// Requirement description shown in menus.
    pub description: &'static str,
    /// Flavor humor / lore text.
    pub lore: &'static str,
    /// Condition logic evaluated during simulation.
    pub condition: AchievementCondition,
    /// Additive bonus multiplier granted once unlocked (e.g. 0.015 = +1.5%).
    pub bonus_multiplier: f64,
}

impl<'de> Deserialize<'de> for AchievementConfig {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct ConfigHelper {
            id: String,
            name: String,
            description: String,
            lore: String,
            condition: AchievementCondition,
            bonus_multiplier: f64,
        }

        let helper = ConfigHelper::deserialize(deserializer)?;
        let (id, name, description, lore) = match helper.id.as_str() {
            "first_drop" => (
                "first_drop",
                "El Comienzo del Fin",
                "Ganar 10 Puntos de Flojera",
                "Cualquier viaje de mil millas empieza sin levantarse del sillón.",
            ),
            "pen_maniac" => (
                "pen_maniac",
                "Síndrome del Resorte",
                "100 clics con el lapicero",
                "Tus compañeros de oficina te miran con odio profundo.",
            ),
            "stare_master" => (
                "stare_master",
                "La Mirada de las 1000 Yardas",
                "Nivel 50 en 'Mirar a la nada'",
                "Tus ojos se secaron por completo, pero lograste el objetivo.",
            ),
            "monday_believer" => (
                "monday_believer",
                "Creyente del Lunes",
                "1 Prestigio (Crisis Existencial)",
                "Juraste que cambiabas. Spoiler: No cambiaste.",
            ),
            "all_turbo" => (
                "all_turbo",
                "Sobrecarga Neuronal",
                "3 actividades en Modo Turbo simultáneo",
                "La máquina vibra con el poder de no hacer nada.",
            ),
            "billionaire_sloth" => (
                "billionaire_sloth",
                "Magnate del Sofá",
                "1,000,000 Puntos Históricos",
                "Eres oficialmente multimillonario en unidades improductivas.",
            ),
            _ => (
                Box::leak(helper.id.into_boxed_str()) as &'static str,
                Box::leak(helper.name.into_boxed_str()) as &'static str,
                Box::leak(helper.description.into_boxed_str()) as &'static str,
                Box::leak(helper.lore.into_boxed_str()) as &'static str,
            ),
        };

        Ok(AchievementConfig {
            id,
            name,
            description,
            lore,
            condition: helper.condition,
            bonus_multiplier: helper.bonus_multiplier,
        })
    }
}

/// Returns the standard default roster of achievements.
#[must_use]
pub fn default_achievements() -> Vec<AchievementConfig> {
    vec![
        AchievementConfig {
            id: "first_drop",
            name: "El Comienzo del Fin",
            description: "Ganar 10 Puntos de Flojera",
            lore: "Cualquier viaje de mil millas empieza sin levantarse del sillón.",
            condition: AchievementCondition::TotalSlothPoints(10.0),
            bonus_multiplier: 0.015,
        },
        AchievementConfig {
            id: "pen_maniac",
            name: "Síndrome del Resorte",
            description: "100 clics con el lapicero",
            lore: "Tus compañeros de oficina te miran con odio profundo.",
            condition: AchievementCondition::TotalPenClicks(100),
            bonus_multiplier: 0.015,
        },
        AchievementConfig {
            id: "stare_master",
            name: "La Mirada de las 1000 Yardas",
            description: "Nivel 50 en 'Mirar a la nada'",
            lore: "Tus ojos se secaron por completo, pero lograste el objetivo.",
            condition: AchievementCondition::ReachLevel {
                activity_index: 1,
                level: 50,
            },
            bonus_multiplier: 0.015,
        },
        AchievementConfig {
            id: "monday_believer",
            name: "Creyente del Lunes",
            description: "1 Prestigio (Crisis Existencial)",
            lore: "Juraste que cambiabas. Spoiler: No cambiaste.",
            condition: AchievementCondition::TotalPrestiges(1),
            bonus_multiplier: 0.015,
        },
        AchievementConfig {
            id: "all_turbo",
            name: "Sobrecarga Neuronal",
            description: "3 actividades en Modo Turbo simultáneo",
            lore: "La máquina vibra con el poder de no hacer nada.",
            condition: AchievementCondition::SimultaneousTurbo(3),
            bonus_multiplier: 0.015,
        },
        AchievementConfig {
            id: "billionaire_sloth",
            name: "Magnate del Sofá",
            description: "1,000,000 Puntos Históricos",
            lore: "Eres oficialmente multimillonario en unidades improductivas.",
            condition: AchievementCondition::TotalSlothPoints(1_000_000.0),
            bonus_multiplier: 0.015,
        },
    ]
}
