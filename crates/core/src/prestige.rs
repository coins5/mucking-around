#![forbid(unsafe_code)]

use serde::{Deserialize, Deserializer, Serialize};

/// Configuration for the existential crisis (prestige) system.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PrestigeConfig {
    /// Base historic Sloth Points required to begin claiming Epiphanies (e.g. 1000.0).
    pub base_cost: f64,
    /// Scaling exponent for epiphany formula calculation (e.g. 0.50 for square root).
    pub exponent: f64,
    /// Base production multiplier awarded per unspent Epiphany (e.g. 0.10 for +10%).
    pub default_bonus_per_point: f64,
}

impl Default for PrestigeConfig {
    fn default() -> Self {
        Self {
            base_cost: 1000.0,
            exponent: 0.50,
            default_bonus_per_point: 0.10,
        }
    }
}

/// Static configuration for a permanent upgrade purchasable with Epiphanies.
/// Static configuration for a permanent upgrade purchasable with Epiphanies.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PermanentUpgradeConfig {
    /// Unique identifier for the upgrade.
    pub id: &'static str,
    /// Human-readable display name.
    pub name: &'static str,
    /// Detailed description of the mechanical benefit.
    pub description: &'static str,
    /// Flavor text / lore describing this permanent breakthrough.
    pub lore: &'static str,
    /// Cost in Epiphanies to unlock.
    pub cost_epiphanies: u32,
}

impl<'de> Deserialize<'de> for PermanentUpgradeConfig {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct ConfigHelper {
            id: String,
            name: String,
            description: String,
            #[serde(default)]
            lore: Option<String>,
            cost_epiphanies: u32,
        }

        let helper = ConfigHelper::deserialize(deserializer)?;
        let (id, name, description, lore) = match helper.id.as_str() {
            "muscle_memory" => (
                "muscle_memory",
                "Memoria Muscular",
                "La primera actividad inicia en Nivel 10 tras reiniciar.",
                "Tu mano ya abre pestañas de ocio por reflejo involuntario.",
            ),
            "cost_optimization" => (
                "cost_optimization",
                "Optimización del Desgano",
                "Los niveles de actividades escalan con costo 1.12 en vez de 1.15.",
                "Descubriste métodos para rendir aún menos con menor esfuerzo.",
            ),
            "zen_enlightenment" => (
                "zen_enlightenment",
                "Iluminación de Sillón",
                "Cada Epifanía libre otorga +15% global en lugar de +10%.",
                "Aceptas el vacío existencial con una taza de café frío.",
            ),
            "eternal_sloth" => (
                "eternal_sloth",
                "Inercia Pura",
                "Todas las actividades son permanentemente un 25% más rápidas.",
                "La física demuestra que el tiempo vuela cuando ignoras tus deberes.",
            ),
            "autopilot" => (
                "autopilot",
                "Piloto Automático",
                "Compra 1 nivel de la actividad más barata cada 2 segundos automáticamente.",
                "La flojera se ejecuta sola. Ya ni para procrastinar te esfuerzas.",
            ),
            _ => (
                Box::leak(helper.id.into_boxed_str()) as &'static str,
                Box::leak(helper.name.into_boxed_str()) as &'static str,
                Box::leak(helper.description.into_boxed_str()) as &'static str,
                match helper.lore {
                    Some(s) => Box::leak(s.into_boxed_str()) as &'static str,
                    None => "",
                },
            ),
        };

        Ok(PermanentUpgradeConfig {
            id,
            name,
            description,
            lore,
            cost_epiphanies: helper.cost_epiphanies,
        })
    }
}

/// Returns the standard default permanent upgrades.
#[must_use]
pub fn default_permanent_upgrades() -> Vec<PermanentUpgradeConfig> {
    vec![
        PermanentUpgradeConfig {
            id: "muscle_memory",
            name: "Memoria Muscular",
            description: "La primera actividad inicia en Nivel 10 tras reiniciar.",
            lore: "Tu mano ya abre pestañas de ocio por reflejo involuntario.",
            cost_epiphanies: 2,
        },
        PermanentUpgradeConfig {
            id: "cost_optimization",
            name: "Optimización del Desgano",
            description: "Los niveles de actividades escalan con costo 1.12 en vez de 1.15.",
            lore: "Descubriste métodos para rendir aún menos con menor esfuerzo.",
            cost_epiphanies: 5,
        },
        PermanentUpgradeConfig {
            id: "zen_enlightenment",
            name: "Iluminación de Sillón",
            description: "Cada Epifanía libre otorga +15% global en lugar de +10%.",
            lore: "Aceptas el vacío existencial con una taza de café frío.",
            cost_epiphanies: 10,
        },
        PermanentUpgradeConfig {
            id: "eternal_sloth",
            name: "Inercia Pura",
            description: "Todas las actividades son permanentemente un 25% más rápidas.",
            lore: "La física demuestra que el tiempo vuela cuando ignoras tus deberes.",
            cost_epiphanies: 20,
        },
        PermanentUpgradeConfig {
            id: "autopilot",
            name: "Piloto Automático",
            description: "Compra 1 nivel de la actividad más barata cada 2 segundos automáticamente.",
            lore: "La flojera se ejecuta sola. Ya ni para procrastinar te esfuerzas.",
            cost_epiphanies: 50,
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_permanent_upgrades() {
        let upgrades = default_permanent_upgrades();
        assert_eq!(upgrades.len(), 5);

        assert_eq!(upgrades[0].id, "muscle_memory");
        assert_eq!(upgrades[0].cost_epiphanies, 2);
        assert_eq!(upgrades[0].name, "Memoria Muscular");
        assert_eq!(
            upgrades[0].lore,
            "Tu mano ya abre pestañas de ocio por reflejo involuntario."
        );

        assert_eq!(upgrades[1].id, "cost_optimization");
        assert_eq!(upgrades[1].cost_epiphanies, 5);
        assert_eq!(upgrades[1].name, "Optimización del Desgano");
        assert_eq!(
            upgrades[1].lore,
            "Descubriste métodos para rendir aún menos con menor esfuerzo."
        );

        assert_eq!(upgrades[2].id, "zen_enlightenment");
        assert_eq!(upgrades[2].cost_epiphanies, 10);
        assert_eq!(upgrades[2].name, "Iluminación de Sillón");
        assert_eq!(
            upgrades[2].lore,
            "Aceptas el vacío existencial con una taza de café frío."
        );

        assert_eq!(upgrades[3].id, "eternal_sloth");
        assert_eq!(upgrades[3].cost_epiphanies, 20);
        assert_eq!(upgrades[3].name, "Inercia Pura");
        assert_eq!(
            upgrades[3].lore,
            "La física demuestra que el tiempo vuela cuando ignoras tus deberes."
        );

        assert_eq!(upgrades[4].id, "autopilot");
        assert_eq!(upgrades[4].cost_epiphanies, 50);
        assert_eq!(upgrades[4].name, "Piloto Automático");
        assert_eq!(
            upgrades[4].lore,
            "La flojera se ejecuta sola. Ya ni para procrastinar te esfuerzas."
        );
    }

    #[test]
    fn test_prestige_config_default() {
        let config = PrestigeConfig::default();
        assert_eq!(config.base_cost, 1000.0);
        assert_eq!(config.exponent, 0.50);
        assert_eq!(config.default_bonus_per_point, 0.10);
    }
}
