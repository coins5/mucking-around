#![forbid(unsafe_code)]

use std::collections::HashMap;
use std::fmt::Write;
use game_core::{ActivityState, GameState, Generator, ResourceId};
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
        let wave = (x + t).sin() * 0.5 + (x * 1.7 - t * 1.2).sin() * 0.35 + (x * 0.5 + t * 0.8).cos() * 0.15;
        let normalized = ((wave + 1.0) * 0.5).clamp(0.0, 0.9999);
        let block_index = ((normalized * 8.0).floor() as usize).min(7);
        output.push(VERTICAL_BLOCKS[block_index]);
    }

    output
}

/// Formats a single activity display line according to its state:
///
/// - If locked (`level == 0`): `[BLOQUEADO] Nombre -> Desbloquear [Tecla]: Cuesta X.XX pts`.
/// - If active and normal (`level > 0 && !is_turbo()`): Displays milestone progress, sub-block progress bar, percentage, remaining time, reward per cycle, and upgrade cost.
/// - If active and turbo (`level > 0 && is_turbo()`): Displays milestone progress, oscillating equalizer bar, continuous rate (+XX.XX pts/seg), speed multiplier, and upgrade cost.
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
        let mut line = String::with_capacity(activity.config.name.len() + 80);
        let _ = write!(
            line,
            "[BLOQUEADO] {name} -> Desbloquear [{key}]: Cuesta {cost:.2} pts",
            name = activity.config.name,
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

        let mut line = String::with_capacity(bar_width * 4 + activity.config.name.len() + 140);
        let _ = write!(
            line,
            "{milestone_tag} {name} [{equalizer}] ⚡ TURBO: +{pts_per_sec:.2} pts/seg ({speed:.1}x vel) | Subir a Lvl. {next_level}: Cuesta {next_cost:.2} pts [Presiona {key}]",
            name = activity.config.name,
        );
        line
    } else {
        let bar = render_sub_block_bar(activity.progress, activity.current_duration(), bar_width);
        let percentage = (activity.progress_ratio() * 100.0).floor() as u32;
        let remaining = activity.remaining_time();
        let reward = activity.current_reward();

        let mut line = String::with_capacity(bar_width * 4 + activity.config.name.len() + 140);
        let _ = write!(
            line,
            "{milestone_tag} {name} [{bar}] {percentage}% Faltan {remaining:.2}s | +{reward:.2} pts | Subir a Lvl. {next_level}: Cuesta {next_cost:.2} pts [Presiona {key}]",
            name = activity.config.name,
        );
        line
    }
}

/// Renders the complete multi-bar text frame for the procrastination game.
///
/// Header displays total balance: `Puntos de Flojera: XX.XX`.
/// Following lines display the formatted state of each activity in the roster.
#[must_use]
pub fn render_game_frame(state: &GameState, bar_width: usize, frame_seed: u64) -> String {
    let estimated_line_len = bar_width * 4 + 140;
    let mut frame = String::with_capacity(state.activities.len() * estimated_line_len + 128);

    let _ = writeln!(frame, "Puntos de Flojera: {:.2}", state.sloth_points);
    let _ = writeln!(frame, "------------------------------------------------------------");

    for (index, activity) in state.activities.iter().enumerate() {
        let line = format_activity_line(activity, index, bar_width, frame_seed);
        let _ = writeln!(frame, "{line}");
    }

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
    let ratio = if !target_duration.is_finite() || target_duration <= 0.0 || !progress.is_finite() || progress <= 0.0 {
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
            assert_eq!(bar.chars().next().unwrap(), exp_char, "Failed at eighth {i}");
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
        assert!(line.contains("[Lvl. 1 / Hito: 25] Esperar a que cargue la barrita"));
        assert!(line.contains("[          ]"));
        assert!(line.contains("0% Faltan 5.00s"));
        assert!(line.contains("+1.00 pts"));
        assert!(line.contains("Subir a Lvl. 2: Cuesta 1.15 pts [Presiona 1]"));
    }

    #[test]
    fn test_format_activity_line_locked() {
        let state = GameState::new();
        let line = format_activity_line(&state.activities[1], 1, 10, 0);
        assert_eq!(
            line,
            "[BLOQUEADO] Mirar a la nada fijamente -> Desbloquear [2]: Cuesta 5.00 pts"
        );
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
        // Set activity 0 to level 1000 where speed_multiplier is 64x -> duration = 5.0 / 64.0 = 0.078125s (Turbo)
        state.activities[0].level = 1000;
        assert!(state.activities[0].is_turbo());

        let line = format_activity_line(&state.activities[0], 0, 12, 42);
        assert!(line.contains("[Lvl. 1000 / Hito: 5000]"));
        assert!(line.contains("Esperar a que cargue la barrita"));
        assert!(line.contains("⚡ TURBO:"));
        assert!(line.contains("pts/seg"));
        assert!(line.contains("64.0x vel"));
        assert!(line.contains("Subir a Lvl. 1001:"));
    }

    #[test]
    fn test_format_activity_line_max_milestone() {
        let mut state = GameState::new();
        state.activities[0].level = 9999;
        assert!(state.activities[0].next_milestone().is_none());

        let line = format_activity_line(&state.activities[0], 0, 12, 0);
        assert!(line.contains("[Lvl. 9999 - MAX]"));
        assert!(line.contains("⚡ TURBO:"));
        assert!(line.contains("1000.0x vel"));
    }

    #[test]
    fn test_render_game_frame() {
        let mut state = GameState::new();
        state.sloth_points = 12.5;
        state.activities[0].progress = 2.5;

        let frame = render_game_frame(&state, 10, 0);
        let lines: Vec<&str> = frame.lines().collect();

        assert_eq!(lines[0], "Puntos de Flojera: 12.50");
        assert_eq!(lines[1], "------------------------------------------------------------");
        assert_eq!(
            lines[2],
            "[Lvl. 1 / Hito: 25] Esperar a que cargue la barrita [█████     ] 50% Faltan 2.50s | +1.00 pts | Subir a Lvl. 2: Cuesta 1.15 pts [Presiona 1]"
        );
        assert_eq!(
            lines[3],
            "[BLOQUEADO] Mirar a la nada fijamente -> Desbloquear [2]: Cuesta 5.00 pts"
        );
        assert_eq!(
            lines[4],
            "[BLOQUEADO] Hacer scroll infinito sin ver nada -> Desbloquear [3]: Cuesta 25.00 pts"
        );
        assert_eq!(
            lines[5],
            "[BLOQUEADO] Abrir la refri vacía por quinta vez -> Desbloquear [4]: Cuesta 100.00 pts"
        );
        assert_eq!(
            lines[6],
            "[BLOQUEADO] Ordenar el escritorio para no trabajar -> Desbloquear [5]: Cuesta 350.00 pts"
        );
    }
}
