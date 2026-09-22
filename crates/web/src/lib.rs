#![forbid(unsafe_code)]

use wasm_bindgen::prelude::*;

use core::{
    AchievementConfig, ActiveView, GameState, PermanentUpgradeConfig, ProductiveComparison,
};

const STORAGE_KEY: &str = "mucking_around_save";

/// WebAssembly bridge managing the game loop, user interactions,
/// rendering, and browser localStorage persistence.
#[wasm_bindgen]
pub struct WebGame {
    state: GameState,
    permanent_upgrades: Vec<PermanentUpgradeConfig>,
    productive_comparisons: Vec<ProductiveComparison>,
    achievements: Vec<AchievementConfig>,
    frame_count: u64,
}

fn get_local_storage() -> Option<web_sys::Storage> {
    #[cfg(target_arch = "wasm32")]
    {
        web_sys::window()?.local_storage().ok()?
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        None
    }
}

#[wasm_bindgen]
impl WebGame {
    /// Initializes the web game.
    /// Attempts to load an existing save from browser localStorage.
    /// If found, evaluates offline progress based on `current_timestamp_secs`.
    /// Otherwise, initializes a new game.
    #[wasm_bindgen(constructor)]
    pub fn new(current_timestamp_secs: f64) -> Self {
        let ts = current_timestamp_secs.max(0.0) as u64;

        let loaded_state = get_local_storage()
            .and_then(|storage| storage.get_item(STORAGE_KEY).ok().flatten())
            .and_then(|raw_json| serde_json::from_str::<GameState>(&raw_json).ok());

        let state = if let Some(mut loaded) = loaded_state {
            loaded.process_offline_progress(ts);
            loaded
        } else {
            let mut new_game = GameState::new();
            new_game.last_save_timestamp = ts;
            new_game
        };

        Self {
            state,
            permanent_upgrades: core::default_permanent_upgrades(),
            productive_comparisons: core::default_productive_comparisons(),
            achievements: core::default_achievements(),
            frame_count: 0,
        }
    }

    /// Loads a game state from an explicit JSON string (used for save file imports).
    pub fn from_save(save_json: &str, current_timestamp_secs: f64) -> Result<WebGame, JsValue> {
        let ts = current_timestamp_secs.max(0.0) as u64;
        let mut loaded = serde_json::from_str::<GameState>(save_json)
            .map_err(|e| JsValue::from_str(&format!("Error deserializing save JSON: {e}")))?;

        loaded.process_offline_progress(ts);

        Ok(Self {
            state: loaded,
            permanent_upgrades: core::default_permanent_upgrades(),
            productive_comparisons: core::default_productive_comparisons(),
            achievements: core::default_achievements(),
            frame_count: 0,
        })
    }

    /// Advances game simulation by delta time `dt` in seconds.
    pub fn tick(&mut self, dt: f64) {
        if dt.is_finite() && dt > 0.0 {
            self.state.tick(dt);
        }
    }

    /// Renders the current frame to a plain text string based on the active view.
    pub fn render_frame(&mut self, bar_width: usize) -> String {
        let frame = match self.state.active_view {
            ActiveView::WelcomeOfflineModal => {
                if let Some(ref report) = self.state.offline_report {
                    ui_text::render_offline_modal(report)
                } else {
                    ui_text::render_game_frame(&self.state, bar_width, self.frame_count)
                }
            }
            ActiveView::PrestigeDialog => ui_text::render_prestige_dialog(&self.state),
            ActiveView::PermanentUpgradesShop => {
                ui_text::render_upgrades_frame(&self.state, &self.permanent_upgrades)
            }
            ActiveView::ExistentialStats => {
                ui_text::render_stats_frame(&self.state.existential_stats, &self.productive_comparisons)
            }
            ActiveView::AchievementsGallery => {
                ui_text::render_achievements_frame(&self.state, &self.achievements)
            }
            ActiveView::MainDashboard => {
                ui_text::render_game_frame(&self.state, bar_width, self.frame_count)
            }
        };

        self.frame_count = self.frame_count.wrapping_add(1);
        frame
    }

    /// Returns a contextual controls hint string for the active screen.
    pub fn get_controls_hint(&self) -> String {
        match self.state.active_view {
            ActiveView::WelcomeOfflineModal => {
                "Controles: [Cualquier tecla / Esc / Espacio] Continuar al juego".to_string()
            }
            ActiveView::PrestigeDialog => {
                "Controles: [S] Confirmar Crisis | [N / Esc] Cancelar".to_string()
            }
            ActiveView::PermanentUpgradesShop => {
                let count = self
                    .permanent_upgrades
                    .iter()
                    .filter(|u| self.state.is_permanent_upgrade_revealed(u.id))
                    .count()
                    .max(1);
                let keys = if count == 1 {
                    "[1] Comprar Mejora".to_string()
                } else {
                    format!("[1-{count}] Comprar Mejora")
                };
                format!("Controles: {keys} | [U / Esc / q] Volver al juego")
            }
            ActiveView::ExistentialStats => "Controles: [S / Esc / q] Volver al juego".to_string(),
            ActiveView::AchievementsGallery => {
                "Controles: [A / Esc / q] Volver al juego".to_string()
            }
            ActiveView::MainDashboard => {
                let num_revealed = self
                    .state
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
                let prestige_keys = if self.state.prestige_revealed {
                    " | [P] Crisis | [U] Mejoras"
                } else {
                    ""
                };
                format!(
                    "Controles: [Espacio] Lapicero / Reclamar | {act_keys}{prestige_keys} | [S] Estadísticas | [A] Logros"
                )
            }
        }
    }

    /// Processes keyboard events, mirroring the native CLI shortcuts.
    pub fn handle_key(&mut self, key: &str) -> bool {
        match self.state.active_view {
            ActiveView::WelcomeOfflineModal => {
                self.state.set_view(ActiveView::MainDashboard);
                true
            }
            ActiveView::PrestigeDialog => match key {
                "s" | "S" => {
                    let _ = self.state.confirm_prestige();
                    true
                }
                "n" | "N" | "Escape" | "Esc" => {
                    self.state.close_prestige_dialog();
                    true
                }
                _ => false,
            },
            ActiveView::PermanentUpgradesShop => match key {
                "1" | "2" | "3" | "4" | "5" => {
                    if let Ok(num) = key.parse::<usize>()
                        && num >= 1
                    {
                        let index = num - 1;
                        if let Some(upgrade) = self.permanent_upgrades.get(index)
                            && self.state.is_permanent_upgrade_revealed(upgrade.id)
                        {
                            let _ = self.state.buy_permanent_upgrade(upgrade.id);
                            return true;
                        }
                    }
                    false
                }
                "u" | "U" | "Escape" | "Esc" | "q" | "Q" => {
                    self.state.set_view(ActiveView::MainDashboard);
                    true
                }
                _ => false,
            },
            ActiveView::ExistentialStats => match key {
                "s" | "S" | "Escape" | "Esc" | "q" | "Q" => {
                    self.state.set_view(ActiveView::MainDashboard);
                    true
                }
                _ => false,
            },
            ActiveView::AchievementsGallery => match key {
                "a" | "A" | "Escape" | "Esc" | "q" | "Q" => {
                    self.state.set_view(ActiveView::MainDashboard);
                    true
                }
                _ => false,
            },
            ActiveView::MainDashboard => match key {
                " " | "Space" => {
                    if self.state.active_distraction.is_some() {
                        let _ = self.state.claim_distraction();
                    } else {
                        let _ = self.state.pen_click();
                    }
                    true
                }
                "d" | "D" => {
                    let _ = self.state.claim_distraction();
                    true
                }
                "1" | "2" | "3" | "4" | "5" => {
                    if let Ok(num) = key.parse::<usize>()
                        && num >= 1
                    {
                        let index = num - 1;
                        let _ = self.state.upgrade_activity(index);
                        return true;
                    }
                    false
                }
                "p" | "P" => {
                    if self.state.prestige_revealed {
                        self.state.open_prestige_dialog();
                        true
                    } else {
                        false
                    }
                }
                "u" | "U" => {
                    if self.state.prestige_revealed {
                        self.state.open_upgrade_menu();
                        true
                    } else {
                        false
                    }
                }
                "s" | "S" => {
                    self.state.set_view(ActiveView::ExistentialStats);
                    true
                }
                "a" | "A" => {
                    self.state.set_view(ActiveView::AchievementsGallery);
                    true
                }
                _ => false,
            },
        }
    }

    /// Direct action: Click pen or claim distraction if active.
    pub fn click_pen(&mut self) {
        if self.state.active_distraction.is_some() {
            let _ = self.state.claim_distraction();
        } else {
            let _ = self.state.pen_click();
        }
    }

    /// Direct action: Claim active distraction.
    pub fn claim_distraction(&mut self) {
        let _ = self.state.claim_distraction();
    }

    /// Direct action: Upgrade activity by 0-based index.
    pub fn upgrade_activity(&mut self, index: usize) -> bool {
        self.state.upgrade_activity(index)
    }

    /// Direct action: Open prestige dialog.
    pub fn open_prestige_dialog(&mut self) {
        if self.state.prestige_revealed {
            self.state.open_prestige_dialog();
        }
    }

    /// Direct action: Confirm prestige.
    pub fn confirm_prestige(&mut self) -> bool {
        self.state.confirm_prestige()
    }

    /// Direct action: Cancel prestige dialog.
    pub fn cancel_prestige(&mut self) {
        self.state.close_prestige_dialog();
    }

    /// Direct action: Open permanent upgrades shop.
    pub fn open_upgrade_shop(&mut self) {
        if self.state.prestige_revealed {
            self.state.open_upgrade_menu();
        }
    }

    /// Direct action: Buy permanent upgrade by 0-based index.
    pub fn buy_permanent_upgrade(&mut self, index: usize) -> bool {
        if let Some(upgrade) = self.permanent_upgrades.get(index) {
            self.state.buy_permanent_upgrade(upgrade.id)
        } else {
            false
        }
    }

    /// Direct action: Open existential stats.
    pub fn open_stats(&mut self) {
        self.state.set_view(ActiveView::ExistentialStats);
    }

    /// Direct action: Open achievements gallery.
    pub fn open_achievements(&mut self) {
        self.state.set_view(ActiveView::AchievementsGallery);
    }

    /// Direct action: Close current modal/view and return to dashboard.
    pub fn close_view(&mut self) {
        self.state.set_view(ActiveView::MainDashboard);
    }

    /// Saves the current game state to browser localStorage.
    pub fn save_to_storage(&mut self, current_timestamp_secs: f64) -> Result<bool, JsValue> {
        self.state.last_save_timestamp = current_timestamp_secs.max(0.0) as u64;
        let serialized = serde_json::to_string_pretty(&self.state)
            .map_err(|e| JsValue::from_str(&format!("Serialization error: {e}")))?;

        if let Some(storage) = get_local_storage() {
            storage
                .set_item(STORAGE_KEY, &serialized)
                .map_err(|e| JsValue::from_str(&format!("localStorage error: {e:?}")))?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Exports the current game state as a formatted JSON string.
    pub fn export_save_json(&mut self, current_timestamp_secs: f64) -> Result<String, JsValue> {
        self.state.last_save_timestamp = current_timestamp_secs.max(0.0) as u64;
        serde_json::to_string_pretty(&self.state)
            .map_err(|e| JsValue::from_str(&format!("Export error: {e}")))
    }

    /// Clears the save from localStorage and resets to a clean game.
    pub fn reset_game(&mut self, current_timestamp_secs: f64) {
        if let Some(storage) = get_local_storage() {
            let _ = storage.remove_item(STORAGE_KEY);
        }
        let mut new_game = GameState::new();
        new_game.last_save_timestamp = current_timestamp_secs.max(0.0) as u64;
        self.state = new_game;
        self.frame_count = 0;
    }

    /// Returns the name of the current active view.
    pub fn active_view_name(&self) -> String {
        format!("{:?}", self.state.active_view)
    }

    /// Returns current balance of Sloth Points.
    pub fn sloth_points(&self) -> f64 {
        self.state.sloth_points
    }

    /// Returns current balance of unspent Epiphanies.
    pub fn epiphanies(&self) -> u32 {
        self.state.epiphanies
    }

    /// Returns total lifetime Sloth Points.
    pub fn lifetime_sloth_points(&self) -> f64 {
        self.state.lifetime_sloth_points
    }

    /// Whether there is an active distraction available to claim.
    pub fn has_active_distraction(&self) -> bool {
        self.state.active_distraction.is_some()
    }

    /// Whether the prestige system is unlocked/revealed.
    pub fn is_prestige_revealed(&self) -> bool {
        self.state.prestige_revealed
    }

    /// Number of revealed activities.
    pub fn revealed_activities_count(&self) -> usize {
        self.state.activities.iter().filter(|a| a.is_revealed).count()
    }

    /// Number of revealed permanent upgrades in the shop.
    pub fn revealed_upgrades_count(&self) -> usize {
        self.permanent_upgrades
            .iter()
            .filter(|u| self.state.is_permanent_upgrade_revealed(u.id))
            .count()
    }

    /// Whether an activity at the given 0-based index is currently revealed (Fog of War).
    pub fn is_activity_revealed(&self, index: usize) -> bool {
        self.state.activities.get(index).is_some_and(|a| a.is_revealed)
    }

    /// Whether a permanent upgrade at the given 0-based index is currently revealed in the shop.
    pub fn is_upgrade_revealed(&self, index: usize) -> bool {
        self.permanent_upgrades
            .get(index)
            .is_some_and(|u| self.state.is_permanent_upgrade_revealed(u.id))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_web_game_initialization() {
        let mut game = WebGame::new(1000.0);
        assert_eq!(game.sloth_points(), 0.0);
        assert_eq!(game.epiphanies(), 0);
        assert_eq!(game.active_view_name(), "MainDashboard");
        assert_eq!(game.revealed_activities_count(), 1);

        let frame = game.render_frame(20);
        assert!(frame.contains("Puntos:"));
        assert!(frame.contains("Esperar a que cargue la barrita"));

        // Fog of War: only activity 0 is revealed initially
        assert!(game.is_activity_revealed(0));
        assert!(!game.is_activity_revealed(1));
        assert!(!game.is_activity_revealed(2));
        assert!(!game.is_activity_revealed(3));
        assert!(!game.is_activity_revealed(4));
        assert!(!game.is_prestige_revealed());
    }

    #[test]
    fn test_web_game_tick_and_pen_click() {
        let mut game = WebGame::new(1000.0);
        game.click_pen();
        assert!(game.sloth_points() >= 0.25);

        game.tick(10.0);
        assert!(game.sloth_points() > 1.0);
    }

    #[test]
    fn test_web_game_key_handling() {
        let mut game = WebGame::new(1000.0);
        // Pressing 's' switches to ExistentialStats
        assert!(game.handle_key("s"));
        assert_eq!(game.active_view_name(), "ExistentialStats");

        // Pressing 'Escape' returns to MainDashboard
        assert!(game.handle_key("Escape"));
        assert_eq!(game.active_view_name(), "MainDashboard");

        // Pressing 'a' switches to AchievementsGallery
        assert!(game.handle_key("a"));
        assert_eq!(game.active_view_name(), "AchievementsGallery");

        // Pressing 'q' returns to MainDashboard
        assert!(game.handle_key("q"));
        assert_eq!(game.active_view_name(), "MainDashboard");
    }

    #[test]
    fn test_web_game_export_and_import() {
        let mut game = WebGame::new(1000.0);
        game.click_pen();
        let pts_before = game.sloth_points();

        let exported = game.export_save_json(1050.0).expect("export should succeed");
        assert!(exported.contains("sloth_points"));

        let imported = WebGame::from_save(&exported, 1050.0).expect("import should succeed");
        assert_eq!(imported.sloth_points(), pts_before);
    }
}

