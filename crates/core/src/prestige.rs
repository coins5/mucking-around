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
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PermanentUpgradeConfig {
    /// Unique identifier for the upgrade.
    pub id: &'static str,
    /// Human-readable display name.
    pub name: &'static str,
    /// Detailed description of the mechanical benefit.
    pub description: &'static str,
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
            cost_epiphanies: u32,
        }

        let helper = ConfigHelper::deserialize(deserializer)?;
        let (id, name, description) = match helper.id.as_str() {
            "muscle_memory" => (
                "muscle_memory",
                "Memoria Muscular",
                "La actividad 1 inicia en nivel 10 tras cada Crisis Existencial",
            ),
            "cost_optimization" => (
                "cost_optimization",
                "Optimización del desgano",
                "El factor de escalado de costo de actividades se reduce de 1.15 a 1.12",
            ),
            "zen_enlightenment" => (
                "zen_enlightenment",
                "Iluminación De Sillon",
                "Cada Epifanía no gastada otorga +15% de producción en vez de +10%",
            ),
            "eternal_sloth" => (
                "eternal_sloth",
                "Flojera Eterna",
                "La duración base de todas las actividades se reduce un 20%",
            ),
            "autopilot" => (
                "autopilot",
                "Piloto Automático",
                "Cada 2.0s compra 1 nivel de la actividad desbloqueada más barata",
            ),
            _ => (
                Box::leak(helper.id.into_boxed_str()) as &'static str,
                Box::leak(helper.name.into_boxed_str()) as &'static str,
                Box::leak(helper.description.into_boxed_str()) as &'static str,
            ),
        };

        Ok(PermanentUpgradeConfig {
            id,
            name,
            description,
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
            description: "La actividad 1 inicia en nivel 10 tras cada Crisis Existencial",
            cost_epiphanies: 2,
        },
        PermanentUpgradeConfig {
            id: "cost_optimization",
            name: "Optimización de Costos",
            description: "El factor de escalado de costo de actividades se reduce de 1.15 a 1.12",
            cost_epiphanies: 5,
        },
        PermanentUpgradeConfig {
            id: "zen_enlightenment",
            name: "Iluminación Zen",
            description: "Cada Epifanía no gastada otorga +15% de producción en vez de +10%",
            cost_epiphanies: 10,
        },
        PermanentUpgradeConfig {
            id: "eternal_sloth",
            name: "Flojera Eterna",
            description: "La duración base de todas las actividades se reduce un 20%",
            cost_epiphanies: 20,
        },
        PermanentUpgradeConfig {
            id: "autopilot",
            name: "Piloto Automático",
            description: "Cada 2.0s compra 1 nivel de la actividad desbloqueada más barata",
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

        assert_eq!(upgrades[1].id, "cost_optimization");
        assert_eq!(upgrades[1].cost_epiphanies, 5);

        assert_eq!(upgrades[2].id, "zen_enlightenment");
        assert_eq!(upgrades[2].cost_epiphanies, 10);

        assert_eq!(upgrades[3].id, "eternal_sloth");
        assert_eq!(upgrades[3].cost_epiphanies, 20);

        assert_eq!(upgrades[4].id, "autopilot");
        assert_eq!(upgrades[4].cost_epiphanies, 50);
    }

    #[test]
    fn test_prestige_config_default() {
        let config = PrestigeConfig::default();
        assert_eq!(config.base_cost, 1000.0);
        assert_eq!(config.exponent, 0.50);
        assert_eq!(config.default_bonus_per_point, 0.10);
    }
}
