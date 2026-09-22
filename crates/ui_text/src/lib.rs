#![forbid(unsafe_code)]

use game_core::{
    AchievementConfig, ActivityState, DistractionRewardType, ExistentialStats, GameState,
    Generator, OfflineProgressReport, PermanentUpgradeConfig, ProductiveComparison, ResourceId,
};
use std::collections::HashMap;
use std::fmt::Write;
use thiserror::Error;

/// Domain errors for text rendering operations.
#[derive(Error, Debug, PartialEq)]
pub enum UiTextError {
    #[error("Invalid bar width: {0}")]
    InvalidWidth(usize),
}

/// Returns the default visible display name for a given resource.
#[must_use]
pub fn resource_name(id: ResourceId) -> &'static str {
    match id {
        ResourceId::Primary => "Energía",
    }
}

/// Trait to obtain display names for resources.
pub trait ResourceDisplayName {
    fn display_name(&self) -> &str;
}

impl ResourceDisplayName for ResourceId {
    fn display_name(&self) -> &str {
        resource_name(*self)
    }
}

/// Configurable registry for custom resource display names.
#[derive(Debug, Clone, Default)]
pub struct ResourcePresentationRegistry {
    custom_names: HashMap<ResourceId, String>,
}

impl ResourcePresentationRegistry {
    /// Creates an empty registry that falls back to default resource names.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets a custom display name for a specific resource identifier.
    pub fn set_name(&mut self, id: ResourceId, name: impl Into<String>) {
        self.custom_names.insert(id, name.into());
    }

    /// Retrieves the display name for a resource.
    #[must_use]
    pub fn get_name(&self, id: ResourceId) -> &str {
        if let Some(custom) = self.custom_names.get(&id) {
            custom.as_str()
        } else {
            resource_name(id)
        }
    }
}

/// Unicode block elements ordered by eighths: 0/8 through 8/8.
pub const SUB_BLOCKS: [char; 9] = [
    ' ',        // 0/8 (\u{0020})
    '\u{258F}', // 1/8 (▏)
    '\u{258E}', // 2/8 (▎)
    '\u{258D}', // 3/8 (▍)
    '\u{258C}', // 4/8 (▌)
    '\u{258B}', // 5/8 (▋)
    '\u{258A}', // 6/8 (▊)
    '\u{2589}', // 7/8 (▉)
    '\u{2588}', // 8/8 (█)
];

/// Renders a sub-block progress bar using Unicode Block Elements (`U+2588` through `U+258F`).
///
/// Each character represents exactly 1 terminal column with 8 subdivisions of granularity.
/// The resulting string is guaranteed to contain exactly `width_in_chars` characters.
///
/// # Arguments
/// * `current` - Current progress value.
/// * `max` - Maximum target value.
/// * `width_in_chars` - Number of terminal character columns for the bar.
#[must_use]
pub fn render_sub_block_bar(current: f64, max: f64, width_in_chars: usize) -> String {
    if width_in_chars == 0 {
        return String::new();
    }

    let mut output = String::with_capacity(width_in_chars * 4);

    if !max.is_finite() || max <= 0.0 || !current.is_finite() || current <= 0.0 {
        for _ in 0..width_in_chars {
            output.push(' ');
        }
        return output;
    }

    let clamped = current.clamp(0.0, max);
    let ratio = (clamped / max).clamp(0.0, 1.0);
    let total_eighths = width_in_chars * 8;

    let filled_eighths = if clamped >= max {
        total_eighths
    } else {
        (((ratio * total_eighths as f64) + 1e-9).floor() as usize).min(total_eighths)
    };

    let full_blocks = filled_eighths / 8;
    let remainder_eighths = filled_eighths % 8;

    if full_blocks >= width_in_chars {
        for _ in 0..width_in_chars {
            output.push(SUB_BLOCKS[8]);
        }
    } else {
        for _ in 0..full_blocks {
            output.push(SUB_BLOCKS[8]);
        }

        if remainder_eighths > 0 {
            output.push(SUB_BLOCKS[remainder_eighths]);
            for _ in 0..(width_in_chars - full_blocks - 1) {
                output.push(' ');
            }
        } else {
            for _ in 0..(width_in_chars - full_blocks) {
                output.push(' ');
            }
        }
    }

    output
}

/// Unicode lower vertical block elements ordered from 1/8 to 8/8 (`U+2581` through `U+2588`).
pub const VERTICAL_BLOCKS: [char; 8] = [' ', '▂', '▃', '▄', '▅', '▆', '▇', '█'];

/// Renders an oscillating equalizer waveform for activities in turbo mode using vertical block elements.
///
/// Pre-allocates string capacity and generates a smooth, pseudo-harmonic wave combining
/// trigonometric functions of column index and frame count. Guaranteed to contain
/// exactly `width` monospace characters.
#[must_use]
pub fn render_turbo_equalizer(width: usize, frame_seed: u64) -> String {
    if width == 0 {
        return String::new();
    }

    let mut output = String::with_capacity(width * 4);
    let t = (frame_seed as f64) * 0.25;

    for col in 0..width {
        let x = (col as f64) * 0.6;
        let wave = (x + t).sin() * 0.5
            + (x * 1.7 - t * 1.2).sin() * 0.35
            + (x * 0.5 + t * 0.8).cos() * 0.15;
        let normalized = ((wave + 1.0) * 0.5).clamp(0.0, 0.9999);
        let block_index = ((normalized * 8.0).floor() as usize).min(7);
        output.push(VERTICAL_BLOCKS[block_index]);
    }

    output
}

/// Formats a single activity display line according to its state:
///
/// - If locked (`level == 0`): Displays shortcut, status, name, unlock cost, and flavor lore.
/// - If active and normal (`level > 0 && !is_turbo()`): Displays shortcut, name, milestone progress, flavor lore, sub-block progress bar, percentage, remaining time, reward per cycle, and upgrade cost.
/// - If active and turbo (`level > 0 && is_turbo()`): Displays shortcut, name, milestone progress, flavor lore, oscillating equalizer bar, continuous rate (+XX.XX pts/seg), speed multiplier, and upgrade cost.
#[must_use]
pub fn format_activity_line(
    activity: &ActivityState,
    index: usize,
    bar_width: usize,
    frame_seed: u64,
) -> String {
    let key = index + 1;
    if activity.level == 0 {
        let cost = activity.next_cost();
        let mut line =
            String::with_capacity(activity.config.name.len() + activity.config.lore.len() + 120);
        let _ = write!(
            line,
            "[{key}] [BLOQUEADO] {name} -> Desbloquear: Cuesta {cost:.2} pts [Presiona {key}]\n    \"{lore}\"",
            name = activity.config.name,
            lore = activity.config.lore,
        );
        return line;
    }

    let milestone_tag = match activity.next_milestone() {
        Some(next) => format!("[Lvl. {} / Hito: {}]", activity.level, next.level),
        None => format!("[Lvl. {} - MAX]", activity.level),
    };

    let next_level = activity.level + 1;
    let next_cost = activity.next_cost();

    if activity.is_turbo() {
        let equalizer = render_turbo_equalizer(bar_width, frame_seed);
        let pts_per_sec = activity.pts_per_second();
        let speed = activity.speed_multiplier();

        let mut line = String::with_capacity(
            bar_width * 4 + activity.config.name.len() + activity.config.lore.len() + 200,
        );
        let _ = write!(
            line,
            "[{key}] {name} {milestone_tag}\n    \"{lore}\"\n    [{equalizer}] ⚡ TURBO: +{pts_per_sec:.2} pts/seg ({speed:.1}x vel) | Subir a Lvl. {next_level}: Cuesta {next_cost:.2} pts [Presiona {key}]",
            name = activity.config.name,
            lore = activity.config.lore,
        );
        line
    } else {
        let bar = render_sub_block_bar(activity.progress, activity.current_duration(), bar_width);
        let percentage = (activity.progress_ratio() * 100.0).floor() as u32;
        let remaining = activity.remaining_time();
        let reward = activity.current_reward();

        let mut line = String::with_capacity(
            bar_width * 4 + activity.config.name.len() + activity.config.lore.len() + 200,
        );
        let _ = write!(
            line,
            "[{key}] {name} {milestone_tag}\n    \"{lore}\"\n    [{bar}] {percentage}% Faltan {remaining:.2}s | +{reward:.2} pts | Subir a Lvl. {next_level}: Cuesta {next_cost:.2} pts [Presiona {key}]",
            name = activity.config.name,
            lore = activity.config.lore,
        );
        line
    }
}

/// Renders the complete multi-bar text frame for the procrastination game.
///
/// Header displays balance, Epiphanies, prestige bonus, achievements multiplier, and frenzy status if active.
/// If an unexpected distraction is present, renders an inverse decay bar countdown and claim alert.
/// Following lines display the formatted state of each activity in the roster.
/// Footer displays prestige progress and keyboard shortcuts for all system views.
#[must_use]
pub fn render_game_frame(state: &GameState, bar_width: usize, frame_seed: u64) -> String {
    let estimated_line_len = bar_width * 4 + 200;
    let mut frame = String::with_capacity(state.activities.len() * estimated_line_len + 512);

    let achieve_bonus_pct = ((state.achievements_multiplier() - 1.0) * 100.0 * 10.0).round() / 10.0;
    if state.prestige_revealed {
        let prestige_bonus_pct = ((state.prestige_multiplier() - 1.0) * 100.0).round() as u64;
        let _ = writeln!(
            frame,
            "Puntos: {:.2} | Epifanías Zen: {} (+{}%) | Logros: +{:.1}%",
            state.sloth_points, state.epiphanies, prestige_bonus_pct, achieve_bonus_pct
        );
    } else {
        let _ = writeln!(
            frame,
            "Puntos: {:.2} | Logros: +{:.1}%",
            state.sloth_points, achieve_bonus_pct
        );
    }

    if state.frenzy_timer > 0.0 {
        let frenzy_bar = render_sub_block_bar(
            state.frenzy_timer,
            state.frenzy_max_duration.max(state.frenzy_timer).max(0.001),
            bar_width,
        );
        let _ = writeln!(
            frame,
            "⚡ ¡FRENESÍ DE FLOJERA ACTIVO! [{frenzy_bar}] x{:.1} ({:.1}s restantes)",
            state.frenzy_multiplier, state.frenzy_timer
        );
    }

    if let Some(ref sound) = state.last_pen_sound {
        let _ = writeln!(frame, "🖊️ Lapicero: {sound}");
    }

    if let Some(ref last) = state.last_claimed_distraction {
        let _ = writeln!(
            frame,
            "🎁 Última distracción: {} -> {}",
            last.title, last.effect_summary
        );
    }

    let _ = writeln!(
        frame,
        "------------------------------------------------------------"
    );

    if let Some(ref distraction) = state.active_distraction {
        let _ = writeln!(
            frame,
            "************************************************************"
        );
        let _ = writeln!(
            frame,
            "📱 ¡DISTRACCIÓN INESPERADA!: {}\n    \"{}\"",
            distraction.config.title, distraction.config.description
        );
        let dist_bar = render_sub_block_bar(
            distraction.time_remaining,
            distraction.config.time_to_claim,
            bar_width,
        );
        let effect_str = match &distraction.config.reward {
            DistractionRewardType::Frenzy {
                multiplier,
                duration_secs,
            } => format!("Frenesí x{multiplier:.1} por {duration_secs:.0}s"),
            DistractionRewardType::InstantSloth {
                percentage_of_current,
                min_flat,
            } => format!(
                "+{:.0}% saldo (mín {:.0} pts)",
                percentage_of_current * 100.0,
                min_flat
            ),
            DistractionRewardType::TimeWarp { simulated_seconds } => {
                let mins = (simulated_seconds / 60.0).round() as u64;
                if mins > 0 {
                    format!("Salto temporal de {mins} min ({simulated_seconds:.0}s)")
                } else {
                    format!("Salto temporal de {simulated_seconds:.0}s")
                }
            }
        };
        let _ = writeln!(
            frame,
            "    [{dist_bar}] {time:.1}s restantes | Recompensa: {effect_str}\n    >>> [ESPACIO / D] ¡Reclamar Distracción! <<<",
            time = distraction.time_remaining
        );
        let _ = writeln!(
            frame,
            "************************************************************"
        );
    }

    for (index, activity) in state.activities.iter().enumerate() {
        if activity.is_revealed {
            let line = format_activity_line(activity, index, bar_width, frame_seed);
            let _ = writeln!(frame, "{line}");
        }
    }

    let _ = writeln!(
        frame,
        "------------------------------------------------------------"
    );
    if state.prestige_revealed {
        let _ = writeln!(
            frame,
            "[P] CRISIS EXISTENCIAL -> Reclamar +{} Epifanías (Histórico: {:.2} pts)",
            state.claimable_epiphanies(),
            state.lifetime_sloth_points
        );
    }

    let num_revealed = state
        .activities
        .iter()
        .filter(|a| a.is_revealed)
        .count()
        .max(1);
    let act_shortcut = if num_revealed == 1 {
        "[1] Subir Nivel".to_string()
    } else {
        format!("[1-{num_revealed}] Subir Nivel")
    };
    let prestige_shortcuts = if state.prestige_revealed {
        " | [P] Crisis | [U] Mejoras"
    } else {
        ""
    };
    let _ = writeln!(
        frame,
        "[ESPACIO] Lapicero / Reclamar | {act_shortcut}{prestige_shortcuts} | [S] Estadísticas | [A] Logros | [Q] Salir"
    );

    frame
}

/// Renders the modal dialog for offline progress accumulated while away.
#[must_use]
pub fn render_offline_modal(report: &OfflineProgressReport) -> String {
    let mut modal = String::with_capacity(768);
    let total_secs = report.elapsed_seconds.max(0.0) as u64;
    let hours = total_secs / 3600;
    let minutes = (total_secs % 3600) / 60;
    let seconds = total_secs % 60;
    let efficiency_pct = report.offline_efficiency * 100.0;

    let _ = writeln!(
        modal,
        "=============================================================="
    );
    let _ = writeln!(modal, "                 ¡PROGRESO MIENTRAS DORMÍAS!");
    let _ = writeln!(
        modal,
        "=============================================================="
    );
    let _ = writeln!(
        modal,
        " Estuviste ausente durante: {} horas, {} minutos y {} segundos.",
        hours, minutes, seconds
    );
    let _ = writeln!(modal, " Eficiencia del descanso: {efficiency_pct:.1}%");
    let _ = writeln!(modal);
    let _ = writeln!(
        modal,
        " Tu nivel de desidia es tan alto que incluso desconectado"
    );
    let _ = writeln!(modal, " lograste acumular:");
    let _ = writeln!(modal);
    let _ = writeln!(
        modal,
        "                  +{:.2} Puntos de Flojera",
        report.points_earned
    );
    let _ = writeln!(
        modal,
        "--------------------------------------------------------------"
    );
    let _ = writeln!(modal, "                   [Presiona cualquier tecla]");
    let _ = writeln!(
        modal,
        "=============================================================="
    );

    modal
}

/// Renders the existential statistics screen and real-world productive comparisons.
#[must_use]
pub fn render_stats_frame(
    stats: &ExistentialStats,
    comparisons: &[ProductiveComparison],
) -> String {
    let mut frame = String::with_capacity(1024);
    let total_secs = stats.total_seconds_played.max(0.0) as u64;
    let hours = total_secs / 3600;
    let minutes = (total_secs % 3600) / 60;
    let seconds = total_secs % 60;

    let _ = writeln!(
        frame,
        "================ ESTADÍSTICAS EXISTENCIALES ================"
    );
    let _ = writeln!(
        frame,
        "Tiempo total procrastinado: {}h {}m {}s",
        hours, minutes, seconds
    );
    let _ = writeln!(frame, "Clics con el lapicero: {}", stats.total_pen_clicks);
    let _ = writeln!(
        frame,
        "Barras de ocio completadas: {}",
        stats.total_bars_completed
    );
    let _ = writeln!(
        frame,
        "Distracciones aprovechadas: {}",
        stats.total_distractions_claimed
    );
    let _ = writeln!(
        frame,
        "Crisis existenciales (Prestigios): {}",
        stats.total_prestiges
    );
    let _ = writeln!(
        frame,
        "Puntos de Flojera históricos acumulados: {:.2}",
        stats.total_sloth_points_earned
    );
    let _ = writeln!(
        frame,
        "------------------------------------------------------------"
    );
    let _ = writeln!(frame, "¿QUÉ COSAS REALES PODRÍAS HABER HECHO EN SU LUGAR?");
    let _ = writeln!(
        frame,
        "------------------------------------------------------------"
    );

    for comp in comparisons {
        let times = if comp.required_seconds > 0.0 {
            (stats.total_seconds_played / comp.required_seconds).floor() as u64
        } else {
            0
        };
        let _ = writeln!(frame, "• [{} veces] {}", times, comp.activity_name);
        let _ = writeln!(frame, "    \"{}\"", comp.humor_lore);
    }

    let _ = writeln!(
        frame,
        "------------------------------------------------------------"
    );
    let _ = writeln!(frame, "[S / Esc] Volver al tablero principal");

    frame
}

/// Renders the achievements gallery screen with unlock statuses and passive multipliers.
#[must_use]
pub fn render_achievements_frame(state: &GameState, achievements: &[AchievementConfig]) -> String {
    let mut frame = String::with_capacity(1024);
    let unlocked_count = achievements
        .iter()
        .filter(|a| state.unlocked_achievements.contains(a.id))
        .count();
    let total_bonus_pct = ((state.achievements_multiplier() - 1.0) * 100.0 * 10.0).round() / 10.0;

    let _ = writeln!(
        frame,
        "=================== GALERÍA DE LOGROS ==================="
    );
    let _ = writeln!(
        frame,
        "Logros desbloqueados: {} / {} | Bono pasivo total: +{:.1}%",
        unlocked_count,
        achievements.len(),
        total_bonus_pct
    );
    let _ = writeln!(
        frame,
        "------------------------------------------------------------"
    );

    for ach in achievements {
        let is_unlocked = state.unlocked_achievements.contains(ach.id);
        let bonus_pct = (ach.bonus_multiplier * 100.0 * 10.0).round() / 10.0;
        if is_unlocked {
            let tag = format!("[DESBLOQUEADO (+{bonus_pct:.1}%)]");
            let _ = writeln!(frame, "{} {}", tag, ach.name);
            let _ = writeln!(frame, "    Condición: {}", ach.description);
            let _ = writeln!(frame, "    \"{}\"", ach.lore);
        } else {
            let tag = "[BLOQUEADO]";
            let _ = writeln!(frame, "\x1b[90m{} {}\x1b[0m", tag, ach.name);
            let _ = writeln!(frame, "\x1b[90m    Condición: {}\x1b[0m", ach.description);
            let _ = writeln!(frame, "\x1b[90m    \"{}\"\x1b[0m", ach.lore);
        }
    }

    let _ = writeln!(
        frame,
        "------------------------------------------------------------"
    );
    let _ = writeln!(frame, "[A / Esc] Volver al tablero principal");

    frame
}

/// Renders the modal dialog for the Existential Crisis (Prestige confirmation).
#[must_use]
pub fn render_prestige_dialog(state: &GameState) -> String {
    let mut dialog = String::with_capacity(512);
    let claimable = state.claimable_epiphanies();
    let bonus_rate = if state.has_permanent_upgrade("zen_enlightenment") {
        15
    } else {
        10
    };
    let bonus_pct = claimable * bonus_rate;

    let _ = writeln!(
        dialog,
        "=============================================================="
    );
    let _ = writeln!(dialog, "                 LA CRISIS DE LAS 3:00 AM");
    let _ = writeln!(
        dialog,
        "=============================================================="
    );
    let _ = writeln!(
        dialog,
        " \"Son las 3:00 AM. Te quedas mirando al techo en la oscuridad"
    );
    let _ = writeln!(
        dialog,
        "  mientras la culpa te invade. Te prometes que mañana será"
    );
    let _ = writeln!(
        dialog,
        "  diferente... pero en el fondo sabes que el lunes empiezas.\""
    );
    let _ = writeln!(
        dialog,
        "--------------------------------------------------------------"
    );
    let _ = writeln!(
        dialog,
        " Reclamarás: +{claimable} Epifanías Zen (+{bonus_pct}% de producción permanente)"
    );
    let _ = writeln!(
        dialog,
        " ¿Aceptas tu destino y reinicias? [S: Confirmar / N: Cancelar]"
    );
    let _ = writeln!(
        dialog,
        "=============================================================="
    );

    dialog
}

/// Renders the permanent upgrades shop frame for Epiphanies.
///
/// Displays current unspent Epiphanies balance, total earned, current production multiplier,
/// and list of permanent upgrades with their status, mechanical effect, and flavor lore:
/// - `[COMPRADA]` if already owned.
/// - `[COMPRAR - Presiona {key}]` if purchasable.
/// - `[BLOQUEADO - Faltan {n} Epifanías]` if insufficient Epiphanies.
#[must_use]
pub fn render_upgrades_frame(state: &GameState, upgrades: &[PermanentUpgradeConfig]) -> String {
    let mut frame = String::with_capacity(upgrades.len() * 256 + 256);
    let bonus_pct = ((state.prestige_multiplier() - 1.0) * 100.0).round() as u64;

    let _ = writeln!(
        frame,
        "=== MENÚ DE ILUMINACIÓN ZEN (MEJORAS PERMANENTES) ==="
    );
    let _ = writeln!(
        frame,
        "Epifanías disponibles: {} | Total ganadas: {} | Bono Producción: +{}%",
        state.epiphanies, state.total_epiphanies_earned, bonus_pct
    );
    let _ = writeln!(
        frame,
        "--------------------------------------------------------------------------------"
    );

    let mut any_revealed = false;
    for (index, upgrade) in upgrades.iter().enumerate() {
        if !state.is_permanent_upgrade_revealed(upgrade.id) {
            continue;
        }
        any_revealed = true;
        let key = index + 1;
        let status = if state.has_permanent_upgrade(upgrade.id) {
            "[COMPRADA]".to_string()
        } else if state.epiphanies >= upgrade.cost_epiphanies {
            format!("[COMPRAR - Presiona {key}]")
        } else {
            let needed = upgrade.cost_epiphanies.saturating_sub(state.epiphanies);
            format!("[BLOQUEADO - Faltan {needed} Epifanías]")
        };

        let _ = writeln!(
            frame,
            "[{key}] {name} - Costo: {cost} Epifanías {status}",
            name = upgrade.name,
            cost = upgrade.cost_epiphanies
        );
        let _ = writeln!(frame, "    Efecto: {desc}", desc = upgrade.description);
        let _ = writeln!(frame, "    \"{lore}\"", lore = upgrade.lore);
    }

    if !any_revealed {
        let _ = writeln!(
            frame,
            "    (Reúne más Epifanías en una Crisis Existencial para descubrir mejoras permanentes)"
        );
    }

    let _ = writeln!(
        frame,
        "--------------------------------------------------------------------------------"
    );
    let _ = writeln!(frame, "[U] Volver al juego principal");

    frame
}

/// Legacy helper for formatting a game state status.
#[must_use]
pub fn format_game_status(state: &GameState, bar_width: usize) -> String {
    render_game_frame(state, bar_width, 0)
}

/// Formats a single progress status line with a sub-block bar, percentage, and resource display.
#[must_use]
pub fn format_status_line(
    progress: f64,
    target_duration: f64,
    resource_label: &str,
    resource_amount: f64,
    bar_width: usize,
) -> String {
    let bar = render_sub_block_bar(progress, target_duration, bar_width);
    let ratio = if !target_duration.is_finite()
        || target_duration <= 0.0
        || !progress.is_finite()
        || progress <= 0.0
    {
        0.0
    } else {
        (progress / target_duration).clamp(0.0, 1.0)
    };
    let percentage = (ratio * 100.0).floor() as u32;

    let mut output = String::with_capacity(bar_width * 4 + resource_label.len() + 32);
    let _ = write!(
        output,
        "[{bar}] {percentage}% | {resource_label}: {resource_amount:.2}"
    );
    output
}

/// Formats a status line directly from a `Generator` component and a resource balance.
#[must_use]
pub fn format_generator_status(
    generator: &Generator,
    resource_amount: f64,
    bar_width: usize,
) -> String {
    let label = resource_name(generator.output_resource);
    format_status_line(
        generator.progress,
        generator.target_duration,
        label,
        resource_amount,
        bar_width,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resource_naming() {
        assert_eq!(resource_name(ResourceId::Primary), "Energía");
        assert_eq!(ResourceId::Primary.display_name(), "Energía");

        let mut registry = ResourcePresentationRegistry::new();
        assert_eq!(registry.get_name(ResourceId::Primary), "Energía");

        registry.set_name(ResourceId::Primary, "Ciclos");
        assert_eq!(registry.get_name(ResourceId::Primary), "Ciclos");
    }

    #[test]
    fn test_zero_width() {
        assert_eq!(render_sub_block_bar(2.5, 5.0, 0), "");
    }

    #[test]
    fn test_empty_bar() {
        let bar = render_sub_block_bar(0.0, 5.0, 10);
        assert_eq!(bar, "          ");
        assert_eq!(bar.chars().count(), 10);
    }

    #[test]
    fn test_full_bar() {
        let bar = render_sub_block_bar(5.0, 5.0, 10);
        assert_eq!(bar, "██████████");
        assert_eq!(bar.chars().count(), 10);
    }

    #[test]
    fn test_overflow_clamping() {
        let bar = render_sub_block_bar(10.0, 5.0, 10);
        assert_eq!(bar, "██████████");
        assert_eq!(bar.chars().count(), 10);
    }

    #[test]
    fn test_sub_block_increments_single_char() {
        let max = 8.0;
        let expected = [' ', '▏', '▎', '▍', '▌', '▋', '▊', '▉', '█'];
        for (i, &exp_char) in expected.iter().enumerate() {
            let bar = render_sub_block_bar(i as f64, max, 1);
            assert_eq!(bar.chars().count(), 1);
            assert_eq!(
                bar.chars().next().unwrap(),
                exp_char,
                "Failed at eighth {i}"
            );
        }
    }

    #[test]
    fn test_half_bar() {
        let bar = render_sub_block_bar(2.5, 5.0, 10);
        assert_eq!(bar, "█████     ");
        assert_eq!(bar.chars().count(), 10);
    }

    #[test]
    fn test_fractional_sub_blocks() {
        let current = 5.0 * (43.0 / 80.0);
        let bar = render_sub_block_bar(current, 5.0, 10);
        assert_eq!(bar, "█████▍    ");
        assert_eq!(bar.chars().count(), 10);
    }

    #[test]
    fn test_safe_handling_of_nan_and_negative() {
        assert_eq!(render_sub_block_bar(-1.0, 5.0, 5), "     ");
        assert_eq!(render_sub_block_bar(f64::NAN, 5.0, 5), "     ");
        assert_eq!(render_sub_block_bar(2.0, 0.0, 5), "     ");
        assert_eq!(render_sub_block_bar(2.0, -5.0, 5), "     ");
        assert_eq!(render_sub_block_bar(2.0, f64::NAN, 5), "     ");
        assert_eq!(render_sub_block_bar(2.0, f64::INFINITY, 5), "     ");
    }

    #[test]
    fn test_strict_character_count_across_range() {
        let width = 20;
        for i in 0..=100 {
            let current = i as f64 / 100.0 * 5.0;
            let bar = render_sub_block_bar(current, 5.0, width);
            assert_eq!(bar.chars().count(), width, "Failed char count at step {i}");
        }
    }

    #[test]
    fn test_format_status_line() {
        let status = format_status_line(2.7, 5.0, "Energía", 1.40, 16);
        assert_eq!(status, "[████████▋       ] 54% | Energía: 1.40");
    }

    #[test]
    fn test_format_generator_status() {
        let generator = Generator::default();
        let status = format_generator_status(&generator, 0.0, 10);
        assert_eq!(status, "[          ] 0% | Energía: 0.00");
    }

    #[test]
    fn test_format_activity_line_unlocked() {
        let state = GameState::new();
        let line = format_activity_line(&state.activities[0], 0, 10, 0);
        assert!(line.contains("[1] Esperar a que cargue la barrita [Lvl. 1 / Hito: 25]"));
        assert!(line.contains(
            "\"La vida se mide en barras de carga que sospechosamente se quedan en 99%.\""
        ));
        assert!(line.contains("[          ]"));
        assert!(line.contains("0% Faltan 5.00s"));
        assert!(line.contains("+1.00 pts"));
        assert!(line.contains("Subir a Lvl. 2: Cuesta 1.15 pts [Presiona 1]"));
    }

    #[test]
    fn test_format_activity_line_locked() {
        let state = GameState::new();
        let line = format_activity_line(&state.activities[1], 1, 10, 0);
        assert!(line.contains(
            "[2] [BLOQUEADO] Mirar a la nada fijamente -> Desbloquear: Cuesta 5.00 pts [Presiona 2]"
        ));
        assert!(line.contains(
            "\"Si miras fijamente a la nada, la nada te exige que te pongas a trabajar.\""
        ));
    }

    #[test]
    fn test_render_turbo_equalizer() {
        assert_eq!(render_turbo_equalizer(0, 0), "");

        let eq1 = render_turbo_equalizer(16, 0);
        assert_eq!(eq1.chars().count(), 16);
        for ch in eq1.chars() {
            assert!(
                VERTICAL_BLOCKS.contains(&ch),
                "Character {ch} not in VERTICAL_BLOCKS"
            );
        }

        let eq2 = render_turbo_equalizer(16, 15);
        assert_eq!(eq2.chars().count(), 16);
        // Waves should shift with different frame seeds
        assert_ne!(eq1, eq2);
    }

    #[test]
    fn test_format_activity_line_turbo() {
        let mut state = GameState::new();
        // Set activity 0 to level 1000 where speed_multiplier is 128x -> duration = 5.0 / 128.0 = 0.0390625s (Turbo)
        state.activities[0].level = 1000;
        assert!(state.activities[0].is_turbo());

        let line = format_activity_line(&state.activities[0], 0, 12, 42);
        assert!(line.contains("[1] Esperar a que cargue la barrita [Lvl. 1000 / Hito: 5000]"));
        assert!(line.contains(
            "\"La vida se mide en barras de carga que sospechosamente se quedan en 99%.\""
        ));
        assert!(line.contains("⚡ TURBO:"));
        assert!(line.contains("pts/seg"));
        assert!(line.contains("128.0x vel"));
        assert!(line.contains("Subir a Lvl. 1001:"));
    }

    #[test]
    fn test_format_activity_line_max_milestone() {
        let mut state = GameState::new();
        state.activities[0].level = 9999;
        assert!(state.activities[0].next_milestone().is_none());

        let line = format_activity_line(&state.activities[0], 0, 12, 0);
        assert!(line.contains("[1] Esperar a que cargue la barrita [Lvl. 9999 - MAX]"));
        assert!(line.contains(
            "\"La vida se mide en barras de carga que sospechosamente se quedan en 99%.\""
        ));
        assert!(line.contains("⚡ TURBO:"));
        assert!(line.contains("1024.0x vel"));
    }

    #[test]
    fn test_render_game_frame() {
        let mut state = GameState::new();
        state.sloth_points = 12.5;
        state.activities[0].progress = 2.5;
        state.check_reveals();

        let frame = render_game_frame(&state, 10, 0);
        // Prestige is not revealed yet: header has points & achievements, no epiphanies
        assert!(frame.contains("Puntos: 12.50 | Logros: +0.0%"));
        assert!(!frame.contains("Epifanías Zen:"));

        // Activity 0 and 1 are revealed, activities 2-4 are hidden
        assert!(frame.contains("[1] Esperar a que cargue la barrita [Lvl. 1 / Hito: 25]"));
        assert!(frame.contains("[█████     ] 50% Faltan 2.50s | +1.00 pts"));
        assert!(frame.contains(
            "[2] [BLOQUEADO] Mirar a la nada fijamente -> Desbloquear: Cuesta 5.00 pts [Presiona 2]"
        ));
        assert!(!frame.contains("[3] [BLOQUEADO]"));
        assert!(!frame.contains("[4] [BLOQUEADO]"));
        assert!(!frame.contains("[5] [BLOQUEADO]"));

        // Prestige prompt and menu shortcuts are hidden
        assert!(!frame.contains("[P] CRISIS EXISTENCIAL"));
        assert!(frame.contains("[ESPACIO] Lapicero / Reclamar | [1-2] Subir Nivel | [S] Estadísticas | [A] Logros | [Q] Salir"));

        // Now earn enough for prestige (4000 historic points = 2 Epiphanies)
        state.lifetime_sloth_points = 4000.0;
        state.check_reveals();
        let frame_with_prestige = render_game_frame(&state, 10, 0);
        assert!(frame_with_prestige.contains("Epifanías Zen: 0 (+0%)"));
        assert!(frame_with_prestige.contains("[P] CRISIS EXISTENCIAL -> Reclamar +2 Epifanías"));
        assert!(frame_with_prestige.contains("[P] Crisis | [U] Mejoras"));
    }

    #[test]
    fn test_render_game_frame_with_distraction_and_frenzy() {
        let mut state = GameState::new();
        state.frenzy_multiplier = 7.0;
        state.frenzy_timer = 18.5;
        state.frenzy_max_duration = 25.0;
        state.last_pen_sound = Some("*¡Crack!*".to_string());
        state.last_claimed_distraction = Some(game_core::ClaimedDistractionFeedback {
            title: "Meme del Grupo".to_string(),
            description: "Desc".to_string(),
            effect_summary: "+200.00 Puntos de Flojera al instante".to_string(),
        });
        state.active_distraction = Some(game_core::ActiveDistractionState::new(
            game_core::default_distractions()[0].clone(),
        ));

        let frame = render_game_frame(&state, 20, 0);
        assert!(frame.contains("⚡ ¡FRENESÍ DE FLOJERA ACTIVO! ["));
        assert!(frame.contains("x7.0 (18.5s restantes)"));
        assert!(frame.contains("🖊️ Lapicero: *¡Crack!*"));
        assert!(frame.contains(
            "🎁 Última distracción: Meme del Grupo -> +200.00 Puntos de Flojera al instante"
        ));
        assert!(frame.contains("📱 ¡DISTRACCIÓN INESPERADA!: Video de Restauración"));
        assert!(frame.contains("Frenesí x7.0 por 25s"));
        assert!(frame.contains(">>> [ESPACIO / D] ¡Reclamar Distracción! <<<"));
    }

    #[test]
    fn test_render_offline_modal() {
        let report = OfflineProgressReport {
            elapsed_seconds: 26530.0, // 7h 22m 10s
            offline_efficiency: 0.50,
            points_earned: 84320.50,
        };
        let modal = render_offline_modal(&report);

        assert!(modal.contains("¡PROGRESO MIENTRAS DORMÍAS!"));
        assert!(modal.contains("Estuviste ausente durante: 7 horas, 22 minutos y 10 segundos."));
        assert!(modal.contains("Eficiencia del descanso: 50.0%"));
        assert!(modal.contains("+84320.50 Puntos de Flojera"));
        assert!(modal.contains("[Presiona cualquier tecla]"));
    }

    #[test]
    fn test_render_stats_frame() {
        let mut stats = ExistentialStats::default();
        stats.total_seconds_played = 7325.0; // 2h 2m 5s
        stats.total_pen_clicks = 150;
        stats.total_bars_completed = 45;
        stats.total_distractions_claimed = 8;
        stats.total_prestiges = 2;
        stats.total_sloth_points_earned = 12500.0;

        let comparisons = game_core::default_productive_comparisons();
        let frame = render_stats_frame(&stats, &comparisons);

        assert!(frame.contains("================ ESTADÍSTICAS EXISTENCIALES ================"));
        assert!(frame.contains("Tiempo total procrastinado: 2h 2m 5s"));
        assert!(frame.contains("Clics con el lapicero: 150"));
        assert!(frame.contains("Barras de ocio completadas: 45"));
        assert!(frame.contains("Distracciones aprovechadas: 8"));
        assert!(frame.contains("Crisis existenciales (Prestigios): 2"));
        assert!(frame.contains("Puntos de Flojera históricos acumulados: 12500.00"));
        assert!(frame.contains("• [122 veces] Tomar un vaso con agua"));
        assert!(frame.contains("\"Estar hidratado ayuda a pensar con claridad... mejor no.\""));
        assert!(frame.contains("[S / Esc] Volver al tablero principal"));
    }

    #[test]
    fn test_render_achievements_frame() {
        let mut state = GameState::new();
        state.unlocked_achievements.insert("first_drop".to_string());
        let achievements = game_core::default_achievements();

        let frame = render_achievements_frame(&state, &achievements);
        assert!(frame.contains("=================== GALERÍA DE LOGROS ==================="));
        assert!(frame.contains("Logros desbloqueados: 1 / 6"));
        assert!(frame.contains("[DESBLOQUEADO (+1.5%)] El Comienzo del Fin"));
        assert!(
            frame.contains("\"Cualquier viaje de mil millas empieza sin levantarse del sillón.\"")
        );
        assert!(frame.contains("[BLOQUEADO] Síndrome del Resorte"));
        assert!(frame.contains("\x1b[90m[BLOQUEADO] Síndrome del Resorte\x1b[0m"));
        assert!(frame.contains("[A / Esc] Volver al tablero principal"));
    }

    #[test]
    fn test_render_prestige_dialog() {
        let mut state = GameState::new();
        state.lifetime_sloth_points = 4000.0;
        let dialog = render_prestige_dialog(&state);

        assert!(dialog.contains("LA CRISIS DE LAS 3:00 AM"));
        assert!(dialog.contains("\"Son las 3:00 AM. Te quedas mirando al techo en la oscuridad"));
        assert!(dialog.contains("mientras la culpa te invade. Te prometes que mañana será"));
        assert!(dialog.contains("diferente... pero en el fondo sabes que el lunes empiezas.\""));
        assert!(dialog.contains("Reclamarás: +2 Epifanías Zen (+20% de producción permanente)"));
        assert!(dialog.contains("¿Aceptas tu destino y reinicias? [S: Confirmar / N: Cancelar]"));

        // With zen_enlightenment upgrade
        state
            .purchased_permanent_upgrades
            .insert("zen_enlightenment".to_string());
        let dialog_zen = render_prestige_dialog(&state);
        assert!(
            dialog_zen.contains("Reclamarás: +2 Epifanías Zen (+30% de producción permanente)")
        );
    }

    #[test]
    fn test_render_upgrades_frame() {
        let mut state = GameState::new();
        state.epiphanies = 2;
        state.total_epiphanies_earned = 2;
        state.check_reveals();
        let upgrades = game_core::default_permanent_upgrades();

        let frame = render_upgrades_frame(&state, &upgrades);
        assert!(frame.contains("=== MENÚ DE ILUMINACIÓN ZEN (MEJORAS PERMANENTES) ==="));
        assert!(
            frame.contains("Epifanías disponibles: 2 | Total ganadas: 2 | Bono Producción: +20%")
        );
        // muscle_memory costs 2: should be revealed and available to buy
        assert!(frame.contains("[1] Memoria Muscular - Costo: 2 Epifanías [COMPRAR - Presiona 1]"));
        assert!(frame.contains("Efecto: La primera actividad inicia en Nivel 10 tras reiniciar."));
        assert!(frame.contains("\"Tu mano ya abre pestañas de ocio por reflejo involuntario.\""));

        // cost_optimization costs 5: should NOT be revealed yet
        assert!(!frame.contains("Optimización del Desgano"));

        // Now earn 5 Epiphanies -> cost_optimization is revealed
        state.epiphanies = 5;
        state.check_reveals();
        let frame_with_5 = render_upgrades_frame(&state, &upgrades);
        assert!(frame_with_5.contains("Optimización del Desgano"));

        // Buy muscle_memory (cost 2) -> 3 Epiphanies remain
        assert!(state.buy_permanent_upgrade("muscle_memory"));
        assert_eq!(state.epiphanies, 3);
        let frame_after = render_upgrades_frame(&state, &upgrades);
        assert!(frame_after.contains("[1] Memoria Muscular - Costo: 2 Epifanías [COMPRADA]"));
        // cost_optimization was previously revealed, so it STAYS revealed as blocked
        assert!(frame_after.contains(
            "[2] Optimización del Desgano - Costo: 5 Epifanías [BLOQUEADO - Faltan 2 Epifanías]"
        ));
    }
}
