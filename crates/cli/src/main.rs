#![forbid(unsafe_code)]

use std::io::{IsTerminal, Write, stdout};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use anyhow::Result;
use core::{ActiveView, GameState};
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

/// Retrieves the current system Unix timestamp in seconds.
fn current_unix_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Saves the game state to disk using `serde_json`.
fn save_game(game: &mut GameState) -> Result<()> {
    game.last_save_timestamp = current_unix_timestamp();
    let data = serde_json::to_string_pretty(game)?;
    std::fs::write(game.persistence_config.save_file_name, data)?;
    Ok(())
}

/// Loads the game state from disk if available, otherwise initializes a new state.
fn load_or_init_game() -> GameState {
    let default_file = "save.json";
    let loaded = std::fs::read_to_string(default_file)
        .ok()
        .and_then(|content| serde_json::from_str::<GameState>(&content).ok());

    if let Some(mut game) = loaded {
        let now = current_unix_timestamp();
        game.process_offline_progress(now);
        game
    } else {
        let mut new_game = GameState::new();
        new_game.last_save_timestamp = current_unix_timestamp();
        new_game
    }
}

fn main() -> Result<()> {
    let is_tty = std::io::stdin().is_terminal();
    let _guard = TerminalGuard::enter();

    let mut game = load_or_init_game();
    let permanent_upgrades = core::default_permanent_upgrades();
    let productive_comparisons = core::default_productive_comparisons();
    let achievements = core::default_achievements();

    let frame_duration = Duration::from_nanos(1_000_000_000 / TARGET_FPS);
    let mut last_tick = Instant::now();
    let mut last_save = Instant::now();
    let mut frame_count = 0u64;

    loop {
        let now = Instant::now();
        let dt = (now - last_tick).as_secs_f64();
        last_tick = now;

        game.tick(dt);

        // Background auto-save check
        if last_save.elapsed().as_secs_f64() >= game.persistence_config.auto_save_interval {
            let _ = save_game(&mut game);
            last_save = Instant::now();
        }

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
                        let _ = save_game(&mut game);
                        print_exit_summary(&game)?;
                        return Ok(());
                    }

                    match game.active_view {
                        ActiveView::WelcomeOfflineModal => {
                            // Any key dismisses welcome modal and returns to main dashboard
                            game.set_view(ActiveView::MainDashboard);
                            execute!(stdout(), Clear(ClearType::All))?;
                        }
                        ActiveView::PrestigeDialog => match key_event.code {
                            KeyCode::Char('s' | 'S') => {
                                let _ = game.confirm_prestige();
                                execute!(stdout(), Clear(ClearType::All))?;
                            }
                            KeyCode::Char('n' | 'N') | KeyCode::Esc => {
                                game.close_prestige_dialog();
                                execute!(stdout(), Clear(ClearType::All))?;
                            }
                            _ => {}
                        },
                        ActiveView::PermanentUpgradesShop => match key_event.code {
                            KeyCode::Char(ch @ '1'..='5') => {
                                let index = (ch as usize) - ('1' as usize);
                                if let Some(upgrade) = permanent_upgrades.get(index)
                                    && game.is_permanent_upgrade_revealed(upgrade.id)
                                {
                                    let _ = game.buy_permanent_upgrade(upgrade.id);
                                }
                            }
                            KeyCode::Char('u' | 'U') | KeyCode::Esc | KeyCode::Char('q' | 'Q') => {
                                game.set_view(ActiveView::MainDashboard);
                                execute!(stdout(), Clear(ClearType::All))?;
                            }
                            _ => {}
                        },
                        ActiveView::ExistentialStats => match key_event.code {
                            KeyCode::Char('s' | 'S') | KeyCode::Esc | KeyCode::Char('q' | 'Q') => {
                                game.set_view(ActiveView::MainDashboard);
                                execute!(stdout(), Clear(ClearType::All))?;
                            }
                            _ => {}
                        },
                        ActiveView::AchievementsGallery => match key_event.code {
                            KeyCode::Char('a' | 'A') | KeyCode::Esc | KeyCode::Char('q' | 'Q') => {
                                game.set_view(ActiveView::MainDashboard);
                                execute!(stdout(), Clear(ClearType::All))?;
                            }
                            _ => {}
                        },
                        ActiveView::MainDashboard => match key_event.code {
                            KeyCode::Char(' ') => {
                                // Claim distraction if active, otherwise click pen
                                if game.active_distraction.is_some() {
                                    let _ = game.claim_distraction();
                                } else {
                                    let _ = game.pen_click();
                                }
                            }
                            KeyCode::Char('d' | 'D') => {
                                let _ = game.claim_distraction();
                            }
                            KeyCode::Char(ch @ '1'..='5') => {
                                let index = (ch as usize) - ('1' as usize);
                                let _ = game.upgrade_activity(index);
                            }
                            KeyCode::Char('p' | 'P') => {
                                if game.prestige_revealed {
                                    game.open_prestige_dialog();
                                    execute!(stdout(), Clear(ClearType::All))?;
                                }
                            }
                            KeyCode::Char('u' | 'U') => {
                                if game.prestige_revealed {
                                    game.open_upgrade_menu();
                                    execute!(stdout(), Clear(ClearType::All))?;
                                }
                            }
                            KeyCode::Char('s' | 'S') => {
                                game.set_view(ActiveView::ExistentialStats);
                                execute!(stdout(), Clear(ClearType::All))?;
                            }
                            KeyCode::Char('a' | 'A') => {
                                game.set_view(ActiveView::AchievementsGallery);
                                execute!(stdout(), Clear(ClearType::All))?;
                            }
                            KeyCode::Char('q' | 'Q') | KeyCode::Esc => {
                                let _ = save_game(&mut game);
                                print_exit_summary(&game)?;
                                return Ok(());
                            }
                            _ => {}
                        },
                    }
                }
            }
        }

        // Render frame based on active view state machine
        let frame = match game.active_view {
            ActiveView::WelcomeOfflineModal => {
                if let Some(ref report) = game.offline_report {
                    ui_text::render_offline_modal(report)
                } else {
                    ui_text::render_game_frame(&game, BAR_WIDTH, frame_count)
                }
            }
            ActiveView::PrestigeDialog => ui_text::render_prestige_dialog(&game),
            ActiveView::PermanentUpgradesShop => {
                ui_text::render_upgrades_frame(&game, &permanent_upgrades)
            }
            ActiveView::ExistentialStats => {
                ui_text::render_stats_frame(&game.existential_stats, &productive_comparisons)
            }
            ActiveView::AchievementsGallery => {
                ui_text::render_achievements_frame(&game, &achievements)
            }
            ActiveView::MainDashboard => ui_text::render_game_frame(&game, BAR_WIDTH, frame_count),
        };

        frame_count = frame_count.wrapping_add(1);
        execute!(stdout(), cursor::MoveTo(0, 0))?;
        for line in frame.lines() {
            execute!(stdout(), terminal::Clear(ClearType::CurrentLine))?;
            println!("{line}\r");
        }
        execute!(stdout(), terminal::Clear(ClearType::CurrentLine))?;

        let controls = match game.active_view {
            ActiveView::WelcomeOfflineModal => {
                "\r\nControles: [Cualquier tecla / Esc / Espacio] Continuar al juego\r".to_string()
            }
            ActiveView::PrestigeDialog => {
                "\r\nControles: [S] Confirmar | [N / Esc] Cancelar\r".to_string()
            }
            ActiveView::PermanentUpgradesShop => {
                let count = permanent_upgrades
                    .iter()
                    .filter(|u| game.is_permanent_upgrade_revealed(u.id))
                    .count()
                    .max(1);
                let keys = if count == 1 {
                    "[1] Comprar Mejora".to_string()
                } else {
                    format!("[1-{count}] Comprar Mejora")
                };
                format!("\r\nControles: {keys} | [U / Esc / q] Volver al juego\r")
            }
            ActiveView::ExistentialStats => {
                "\r\nControles: [S / Esc / q] Volver al juego\r".to_string()
            }
            ActiveView::AchievementsGallery => {
                "\r\nControles: [A / Esc / q] Volver al juego\r".to_string()
            }
            ActiveView::MainDashboard => {
                let num_revealed = game
                    .activities
                    .iter()
                    .filter(|a| a.is_revealed)
                    .count()
                    .max(1);
                let act_keys = if num_revealed == 1 {
                    "[1] Mejorar".to_string()
                } else {
                    format!("[1-{num_revealed}] Mejorar")
                };
                let prestige_keys = if game.prestige_revealed {
                    " | [P] Crisis | [U] Mejoras"
                } else {
                    ""
                };
                format!(
                    "\r\nControles: [Espacio] Lapicero / Reclamar | {act_keys}{prestige_keys} | [S] Estadísticas | [A] Logros | [q / Esc] Salir\r"
                )
            }
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

    let _ = save_game(&mut game);
    print_exit_summary(&game)?;
    Ok(())
}

fn print_exit_summary(game: &GameState) -> Result<()> {
    println!(
        "\r\nSimulación guardada y finalizada. Puntos: {:.2} | Epifanías Zen: {} (Histórico: {:.2} pts)\r",
        game.sloth_points, game.epiphanies, game.lifetime_sloth_points
    );
    Ok(())
}
