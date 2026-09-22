#![forbid(unsafe_code)]

use std::io::{IsTerminal, stdout, Write};
use std::time::{Duration, Instant};

use anyhow::Result;
use core::{GameState, ResourceId};
use crossterm::{
    cursor::{Hide, Show},
    event::{self, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{Clear, ClearType, disable_raw_mode, enable_raw_mode},
};

const TARGET_FPS: u64 = 60;
const BAR_WIDTH: usize = 20;

/// RAII Guard that manages terminal state and ensures cursor visibility and
/// terminal modes are safely restored even upon early returns or errors.
struct TerminalGuard {
    is_raw: bool,
}

impl TerminalGuard {
    fn enter() -> Self {
        let is_raw = enable_raw_mode().is_ok();
        let _ = execute!(stdout(), Hide);
        Self { is_raw }
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let _ = execute!(stdout(), Show);
        if self.is_raw {
            let _ = disable_raw_mode();
        }
    }
}

fn main() -> Result<()> {
    let is_tty = std::io::stdin().is_terminal();
    let _guard = TerminalGuard::enter();

    let mut game = GameState::new();
    let frame_duration = Duration::from_nanos(1_000_000_000 / TARGET_FPS);
    let mut last_tick = Instant::now();

    println!("Iniciando ciclo Idle (5.0s ciclo, +0.7 Energía)... Presiona 'q' para salir.\r");

    loop {
        let now = Instant::now();
        let dt = (now - last_tick).as_secs_f64();
        last_tick = now;

        game.tick(dt);

        let status_line = ui_text::format_game_status(&game, BAR_WIDTH);

        let _ = execute!(stdout(), Clear(ClearType::CurrentLine));
        print!("\r{status_line} [q: salir]");
        let _ = stdout().flush();

        if should_exit(is_tty)? {
            break;
        }

        let elapsed = now.elapsed();
        if elapsed < frame_duration {
            std::thread::sleep(frame_duration - elapsed);
        }
    }

    let primary_balance = game.get_resource(ResourceId::Primary);
    println!(
        "\r\nSimulación finalizada. Energía acumulada: {:.2}\r",
        primary_balance
    );

    Ok(())
}

/// Checks non-blockingly for pending user exit commands ('q', 'Q', Esc, or Ctrl-C).
fn should_exit(is_tty: bool) -> Result<bool> {
    if !is_tty {
        return Ok(false);
    }

    while let Ok(true) = event::poll(Duration::from_millis(0)) {
        if let Ok(Event::Key(key_event)) = event::read() {
            let is_quit = matches!(key_event.code, KeyCode::Char('q' | 'Q') | KeyCode::Esc)
                || (key_event.code == KeyCode::Char('c')
                    && key_event.modifiers.contains(KeyModifiers::CONTROL));
            if is_quit {
                return Ok(true);
            }
        }
    }
    Ok(false)
}
