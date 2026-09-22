#![forbid(unsafe_code)]

use serde::{Deserialize, Deserializer, Serialize};

/// Existential metrics tracked continuously throughout the simulation.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ExistentialStats {
    /// Total duration in seconds elapsed during active play.
    pub total_seconds_played: f64,
    /// Total manual pen clicks performed.
    pub total_pen_clicks: u64,
    /// Total activity cycles completed.
    pub total_bars_completed: u64,
    /// Total unexpected distractions successfully claimed before decaying.
    pub total_distractions_claimed: u64,
    /// Total existential crises (prestige resets) triggered.
    pub total_prestiges: u32,
    /// Total Sloth Points earned across all sources and prestiges.
    pub total_sloth_points_earned: f64,
}

/// A real-world productive activity used to contrast against wasted time.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ProductiveComparison {
    /// Time in seconds required to complete this productive task.
    pub required_seconds: f64,
    /// Human-readable title of the productive action.
    pub activity_name: &'static str,
    /// Flavor humor / existential comment.
    pub humor_lore: &'static str,
}

impl<'de> Deserialize<'de> for ProductiveComparison {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct ConfigHelper {
            required_seconds: f64,
            activity_name: String,
            humor_lore: String,
        }

        let helper = ConfigHelper::deserialize(deserializer)?;
        let (activity_name, humor_lore) = match helper.activity_name.as_str() {
            "Tomar un vaso con agua" => (
                "Tomar un vaso con agua",
                "Estar hidratado ayuda a pensar con claridad... mejor no.",
            ),
            "Responder ese correo urgente" => (
                "Responder ese correo urgente",
                "Ese cliente puede esperar 4 semanas más.",
            ),
            "Hacer una rutina de estiramientos" => (
                "Hacer una rutina de estiramientos",
                "Tu columna tiene la forma de un signo de interrogación.",
            ),
            "Cocinar comida nutritiva" => (
                "Cocinar comida nutritiva",
                "El delivery de pizza grasosa tarda solo 30 minutos.",
            ),
            "Limpiar a fondo el departamento" => (
                "Limpiar a fondo el departamento",
                "El polvo acumulado le da textura vintage a los muebles.",
            ),
            "Aprender las bases de un nuevo idioma" => (
                "Aprender las bases de un nuevo idioma",
                "Decir 'procrastinar' en español es más que suficiente.",
            ),
            _ => (
                Box::leak(helper.activity_name.into_boxed_str()) as &'static str,
                Box::leak(helper.humor_lore.into_boxed_str()) as &'static str,
            ),
        };

        Ok(ProductiveComparison {
            required_seconds: helper.required_seconds,
            activity_name,
            humor_lore,
        })
    }
}

/// Returns the standard roster of real-world productive comparisons.
#[must_use]
pub fn default_productive_comparisons() -> Vec<ProductiveComparison> {
    vec![
        ProductiveComparison {
            required_seconds: 60.0,
            activity_name: "Tomar un vaso con agua",
            humor_lore: "Estar hidratado ayuda a pensar con claridad... mejor no.",
        },
        ProductiveComparison {
            required_seconds: 300.0,
            activity_name: "Responder ese correo urgente",
            humor_lore: "Ese cliente puede esperar 4 semanas más.",
        },
        ProductiveComparison {
            required_seconds: 900.0,
            activity_name: "Hacer una rutina de estiramientos",
            humor_lore: "Tu columna tiene la forma de un signo de interrogación.",
        },
        ProductiveComparison {
            required_seconds: 3_600.0,
            activity_name: "Cocinar comida nutritiva",
            humor_lore: "El delivery de pizza grasosa tarda solo 30 minutos.",
        },
        ProductiveComparison {
            required_seconds: 14_400.0,
            activity_name: "Limpiar a fondo el departamento",
            humor_lore: "El polvo acumulado le da textura vintage a los muebles.",
        },
        ProductiveComparison {
            required_seconds: 86_400.0,
            activity_name: "Aprender las bases de un nuevo idioma",
            humor_lore: "Decir 'procrastinar' en español es más que suficiente.",
        },
    ]
}
