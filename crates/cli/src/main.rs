#![forbid(unsafe_code)]

use std::io::{IsTerminal, stdout, Write};
use std::time::{Duration, Instant};

use anyhow::Result;
use core::GameState;
use crossterm::{
    cursor::{self, Hide, Show},
    event::{self, Event, KeyCode, KeyEventKind, KeyModifiers},
    execute,
    terminal::{self, Clear, ClearType, disable_raw_mode, enable_raw_mode},
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
        let _ = execute!(stdout(), Hide, Clear(ClearType::All), cursor::MoveTo(0, 0));
        Self { is_raw }
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let _ = execute!(stdout(), Show);
        if self.is_raw {
            let _ = disable_raw_mode();
        }
        println!();
    }
}

fn main() -> Result<()> {
    let is_tty = std::io::stdin().is_terminal();
    let _guard = TerminalGuard::enter();

    let mut game = GameState::new();
    let frame_duration = Duration::from_nanos(1_000_000_000 / TARGET_FPS);
    let mut last_tick = Instant::now();
    let mut frame_count = 0u64;

    loop {
        let now = Instant::now();
        let dt = (now - last_tick).as_secs_f64();
        last_tick = now;

        game.tick(dt);

        // Process non-blocking keyboard input
        if is_tty {
            while event::poll(Duration::ZERO)? {
                if let Event::Key(key_event) = event::read()? {
                    // Ignore key releases on platforms that emit them
                    if key_event.kind == KeyEventKind::Release {
                        continue;
                    }

                    // Check for exit
                    let is_exit = matches!(key_event.code, KeyCode::Char('q' | 'Q') | KeyCode::Esc)
                        || (key_event.code == KeyCode::Char('c')
                            && key_event.modifiers.contains(KeyModifiers::CONTROL));
                    if is_exit {
                        print_exit_summary(&game)?;
                        return Ok(());
                    }

                    // Check for upgrading / unlocking activities (1-5)
                    if let KeyCode::Char(ch @ '1'..='5') = key_event.code {
                        let index = (ch as usize) - ('1' as usize);
                        let _ = game.upgrade_activity(index);
                    }
                }
            }
        }

        // Render current frame
        let frame = ui_text::render_game_frame(&game, BAR_WIDTH, frame_count);
        frame_count = frame_count.wrapping_add(1);
        execute!(stdout(), cursor::MoveTo(0, 0))?;
        for line in frame.lines() {
            execute!(stdout(), terminal::Clear(ClearType::CurrentLine))?;
            println!("{line}\r");
        }
        execute!(stdout(), terminal::Clear(ClearType::CurrentLine))?;
        println!("\r\nControles: [1-5] Mejorar / Desbloquear actividad | [q / Esc] Salir\r");
        stdout().flush()?;

        // If non-interactive environment, avoid infinite loop
        if !is_tty {
            break;
        }

        let elapsed = now.elapsed();
        if elapsed < frame_duration {
            std::thread::sleep(frame_duration - elapsed);
        }
    }

    print_exit_summary(&game)?;
    Ok(())
}

fn print_exit_summary(game: &GameState) -> Result<()> {
    println!(
        "\r\nSimulación finalizada. Puntos de Flojera acumulados: {:.2}\r",
        game.sloth_points
    );
    Ok(())
}
