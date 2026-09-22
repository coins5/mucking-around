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

/// Active view screen in the terminal CLI.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ViewMode {
    Main,
    Upgrades,
}

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
    let permanent_upgrades = core::default_permanent_upgrades();
    let mut view_mode = ViewMode::Main;

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

                    // Check for Ctrl+C exit
                    let is_ctrl_c = key_event.code == KeyCode::Char('c')
                        && key_event.modifiers.contains(KeyModifiers::CONTROL);
                    if is_ctrl_c {
                        print_exit_summary(&game)?;
                        return Ok(());
                    }

                    // Check for view toggle with 'u' or 'U'
                    if matches!(key_event.code, KeyCode::Char('u' | 'U')) {
                        view_mode = match view_mode {
                            ViewMode::Main => ViewMode::Upgrades,
                            ViewMode::Upgrades => ViewMode::Main,
                        };
                        execute!(stdout(), Clear(ClearType::All))?;
                        continue;
                    }

                    // Check for quit or back
                    let is_exit_key = matches!(key_event.code, KeyCode::Char('q' | 'Q') | KeyCode::Esc);
                    if is_exit_key {
                        match view_mode {
                            ViewMode::Upgrades => {
                                view_mode = ViewMode::Main;
                                execute!(stdout(), Clear(ClearType::All))?;
                                continue;
                            }
                            ViewMode::Main => {
                                print_exit_summary(&game)?;
                                return Ok(());
                            }
                        }
                    }

                    match view_mode {
                        ViewMode::Main => {
                            // Prestige trigger: 'p' or 'P'
                            if matches!(key_event.code, KeyCode::Char('p' | 'P')) {
                                let _ = game.trigger_prestige();
                            }

                            // Check for upgrading / unlocking activities (1-5)
                            if let KeyCode::Char(ch @ '1'..='5') = key_event.code {
                                let index = (ch as usize) - ('1' as usize);
                                let _ = game.upgrade_activity(index);
                            }
                        }
                        ViewMode::Upgrades => {
                            // Buy permanent upgrades (1-5)
                            if let KeyCode::Char(ch @ '1'..='5') = key_event.code {
                                let index = (ch as usize) - ('1' as usize);
                                if let Some(upgrade) = permanent_upgrades.get(index) {
                                    let _ = game.buy_permanent_upgrade(upgrade.id);
                                }
                            }
                        }
                    }
                }
            }
        }

        // Render current frame
        let frame = match view_mode {
            ViewMode::Main => ui_text::render_game_frame(&game, BAR_WIDTH, frame_count),
            ViewMode::Upgrades => ui_text::render_upgrades_frame(&game, &permanent_upgrades),
        };
        frame_count = frame_count.wrapping_add(1);
        execute!(stdout(), cursor::MoveTo(0, 0))?;
        for line in frame.lines() {
            execute!(stdout(), terminal::Clear(ClearType::CurrentLine))?;
            println!("{line}\r");
        }
        execute!(stdout(), terminal::Clear(ClearType::CurrentLine))?;

        let controls = match view_mode {
            ViewMode::Main => "\r\nControles: [1-5] Mejorar | [P] Crisis Existencial | [U] Mejoras Permanentes | [q / Esc] Salir\r",
            ViewMode::Upgrades => "\r\nControles: [1-5] Comprar Mejora | [U / Esc] Volver al juego | [q] Volver\r",
        };
        println!("{controls}");
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
        "\r\nSimulación finalizada. Puntos: {:.2} | Epifanías Zen: {} (Histórico: {:.2} pts)\r",
        game.sloth_points, game.epiphanies, game.lifetime_sloth_points
    );
    Ok(())
}
