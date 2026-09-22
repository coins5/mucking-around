#![forbid(unsafe_code)]

use std::collections::HashMap;
use std::fmt::Write;
use game_core::{GameState, Generator, ResourceId};
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

    /// Retrieves the display name for a resource, returning the custom name if configured
    /// or falling back to the default display name.
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
///
/// # Safety and Determinism
/// - Preallocates `String::with_capacity(width_in_chars * 4)` to avoid reallocations.
/// - If `width_in_chars == 0`, returns an empty string immediately.
/// - If `max <= 0.0`, `current <= 0.0`, or either is NaN/infinite, safely returns `width_in_chars` spaces without panicking.
/// - Inputs are clamped to `[0.0, max]` to eliminate visual overflow.
#[must_use]
pub fn render_sub_block_bar(current: f64, max: f64, width_in_chars: usize) -> String {
    if width_in_chars == 0 {
        return String::new();
    }

    // Pre-allocate buffer: 4 bytes per char ensures zero dynamic reallocations
    // as UTF-8 block elements occupy 3 bytes each and spaces occupy 1 byte.
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

    // Small epsilon to prevent float rounding down on exact multiples
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

/// Formats a complete progress status line with a sub-block bar, percentage, and resource display.
///
/// Example output: `[████████▌      ] 54% | Energía: 1.40`
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

    // Pre-allocate buffer capacity to prevent dynamic reallocations during frame renders.
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

/// Formats a status line directly from the current `GameState`.
#[must_use]
pub fn format_game_status(state: &GameState, bar_width: usize) -> String {
    let label = resource_name(state.generator.output_resource);
    let amount = state.get_resource(state.generator.output_resource);
    format_status_line(
        state.generator.progress,
        state.generator.target_duration,
        label,
        amount,
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
        // 10 chars = 80 eighths.
        // 5.0 * (43 / 80) = 2.6875 -> 43 eighths = 5 full blocks (40 eighths) + remainder 3 ('▍')
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
        // 2.7 / 5.0 = 54%
        // In 16-char bar: 16 * 8 = 128 eighths. 128 * 0.54 = 69.12 eighths -> 8 full blocks (64 eighths) + remainder 5 ('▋') + 7 spaces
        let status = format_status_line(2.7, 5.0, "Energía", 1.40, 16);
        assert_eq!(status, "[████████▋       ] 54% | Energía: 1.40");
    }

    #[test]
    fn test_format_game_status() {
        let mut state = GameState::new();
        state.generator.progress = 2.5;
        state.add_resource(ResourceId::Primary, 0.7);

        let status = format_game_status(&state, 10);
        assert_eq!(status, "[█████     ] 50% | Energía: 0.70");
    }

    #[test]
    fn test_format_generator_status() {
        let generator = Generator::default();
        let status = format_generator_status(&generator, 0.0, 10);
        assert_eq!(status, "[          ] 0% | Energía: 0.00");
    }
}
