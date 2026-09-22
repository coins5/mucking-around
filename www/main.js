import init, { WebGame } from './pkg/web.js';

const BAR_WIDTH = 20;
const AUTO_SAVE_INTERVAL_MS = 30_000;

let game = null;
let lastTime = performance.now();
let lastAutoSave = performance.now();
let frameCounter = 0;
let fpsTimer = performance.now();

// Elementos del DOM
const screenElem = document.getElementById('screen');
const controlsHintElem = document.getElementById('controls-hint');
const saveStatusElem = document.getElementById('save-status');
const fpsDisplayElem = document.getElementById('fps-display');
const spaceLabelElem = document.getElementById('space-label');

const dashboardPanel = document.getElementById('dashboard-controls');
const prestigePanel = document.getElementById('prestige-controls');
const shopPanel = document.getElementById('shop-controls');
const modalPanel = document.getElementById('modal-controls');

const btnSpace = document.getElementById('btn-space');
const btnPrestige = document.getElementById('btn-prestige');
const btnUpgrades = document.getElementById('btn-upgrades');
const btnStats = document.getElementById('btn-stats');
const btnAchievements = document.getElementById('btn-achievements');

const btnConfirmPrestige = document.getElementById('btn-confirm-prestige');
const btnCancelPrestige = document.getElementById('btn-cancel-prestige');
const btnShopBack = document.getElementById('btn-shop-back');
const btnModalBack = document.getElementById('btn-modal-back');

const btnExport = document.getElementById('btn-export');
const btnImport = document.getElementById('btn-import');
const btnReset = document.getElementById('btn-reset');
const fileInput = document.getElementById('file-input');

function currentTimestamp() {
  return Date.now() / 1000.0;
}

function flashSaveStatus(message = '💾 Guardado') {
  if (!saveStatusElem) return;
  saveStatusElem.textContent = message;
  saveStatusElem.classList.add('saved');
  setTimeout(() => {
    saveStatusElem.classList.remove('saved');
  }, 1500);
}

function triggerSave() {
  if (!game) return;
  try {
    const success = game.save_to_storage(currentTimestamp());
    if (success) {
      flashSaveStatus('💾 Guardado');
    }
  } catch (err) {
    console.error('Error guardando en localStorage:', err);
  }
}

// Actualiza los controles dinámicos respetando la Niebla de Guerra y la pantalla activa
function updateDynamicControls() {
  if (!game) return;

  const activeView = game.active_view_name();

  // Ocultar todos los paneles contextuales
  if (dashboardPanel) dashboardPanel.style.display = 'none';
  if (prestigePanel) prestigePanel.style.display = 'none';
  if (shopPanel) shopPanel.style.display = 'none';
  if (modalPanel) modalPanel.style.display = 'none';

  if (activeView === 'MainDashboard') {
    if (dashboardPanel) dashboardPanel.style.display = 'block';

    // 1. Botón primario: Distracción vs Lapicero
    if (game.has_active_distraction()) {
      if (spaceLabelElem) spaceLabelElem.textContent = '⚡ ¡RECLAMAR DISTRACCIÓN!';
      if (btnSpace) {
        btnSpace.style.backgroundColor = '#d29922';
        btnSpace.style.borderColor = '#e3b341';
      }
    } else {
      if (spaceLabelElem) spaceLabelElem.textContent = 'Clic Lapicero';
      if (btnSpace) {
        btnSpace.style.backgroundColor = '';
        btnSpace.style.borderColor = '';
      }
    }

    // 2. Actividades: NIEBLA DE GUERRA (Fog of War)
    // Solo se muestran los botones de las actividades descubiertas en GameState
    for (let i = 1; i <= 5; i++) {
      const btn = document.getElementById(`btn-act-${i}`);
      if (btn) {
        const isRevealed = game.is_activity_revealed(i - 1);
        btn.style.display = isRevealed ? 'inline-flex' : 'none';
      }
    }

    // 3. Botones de navegación según progresión
    const prestigeRevealed = game.is_prestige_revealed();
    if (btnPrestige) btnPrestige.style.display = prestigeRevealed ? 'inline-flex' : 'none';
    if (btnUpgrades) btnUpgrades.style.display = prestigeRevealed ? 'inline-flex' : 'none';

  } else if (activeView === 'PrestigeDialog') {
    if (prestigePanel) prestigePanel.style.display = 'flex';

  } else if (activeView === 'PermanentUpgradesShop') {
    if (shopPanel) {
      shopPanel.style.display = 'flex';

      // Mejoras de la tienda: Niebla de guerra para mejoras reveladas
      for (let i = 1; i <= 5; i++) {
        const btn = document.getElementById(`btn-upg-${i}`);
        if (btn) {
          const isRevealed = game.is_upgrade_revealed(i - 1);
          btn.style.display = isRevealed ? 'inline-flex' : 'none';
        }
      }
    }

  } else {
    // WelcomeOfflineModal, ExistentialStats, AchievementsGallery
    if (modalPanel) {
      modalPanel.style.display = 'flex';
      if (btnModalBack) {
        if (activeView === 'WelcomeOfflineModal') {
          btnModalBack.textContent = '✨ Continuar al juego';
        } else {
          btnModalBack.textContent = '↩️ [Esc] Volver al juego';
        }
      }
    }
  }
}

// Bucle principal a 60 FPS con requestAnimationFrame
function gameLoop(now) {
  const dt = Math.min((now - lastTime) / 1000.0, 0.25);
  lastTime = now;

  if (game) {
    game.tick(dt);

    // Renderizar cuadro de texto
    const frame = game.render_frame(BAR_WIDTH);
    if (screenElem) screenElem.textContent = frame;

    const hint = game.get_controls_hint();
    if (controlsHintElem) controlsHintElem.textContent = hint;

    // Sincronizar controles con el estado del juego
    updateDynamicControls();
  }

  // Contador de FPS
  frameCounter++;
  if (now - fpsTimer >= 500) {
    const fps = Math.round((frameCounter * 1000) / (now - fpsTimer));
    if (fpsDisplayElem) fpsDisplayElem.textContent = `${fps} FPS`;
    frameCounter = 0;
    fpsTimer = now;
  }

  // Autoguardado periódico (cada 30s)
  if (now - lastAutoSave >= AUTO_SAVE_INTERVAL_MS) {
    triggerSave();
    lastAutoSave = now;
  }

  requestAnimationFrame(gameLoop);
}

// Configuración de listeners de teclado
function setupKeyboardListeners() {
  window.addEventListener('keydown', (e) => {
    if (!game) return;

    // Prevenir scroll en barra espaciadora
    if (e.key === ' ' || e.code === 'Space') {
      e.preventDefault();
      game.handle_key(' ');
      return;
    }

    const handled = game.handle_key(e.key);
    if (handled) {
      e.preventDefault();
    }
  });
}

// Configuración de controles táctiles / botones auxiliares
function setupTouchListeners() {
  if (btnSpace) {
    btnSpace.addEventListener('click', () => {
      if (game) game.click_pen();
    });
  }

  // Botones de actividades 1 a 5
  for (let i = 1; i <= 5; i++) {
    const btn = document.getElementById(`btn-act-${i}`);
    if (btn) {
      btn.addEventListener('click', () => {
        if (game) game.handle_key(`${i}`);
      });
    }
  }

  // Botones de navegación del Dashboard
  if (btnPrestige) {
    btnPrestige.addEventListener('click', () => {
      if (game) game.handle_key('p');
    });
  }

  if (btnUpgrades) {
    btnUpgrades.addEventListener('click', () => {
      if (game) game.handle_key('u');
    });
  }

  if (btnStats) {
    btnStats.addEventListener('click', () => {
      if (game) game.handle_key('s');
    });
  }

  if (btnAchievements) {
    btnAchievements.addEventListener('click', () => {
      if (game) game.handle_key('a');
    });
  }

  // Botones del diálogo de Crisis Existencial (Prestigio)
  if (btnConfirmPrestige) {
    btnConfirmPrestige.addEventListener('click', () => {
      if (game) game.handle_key('s');
    });
  }

  if (btnCancelPrestige) {
    btnCancelPrestige.addEventListener('click', () => {
      if (game) game.handle_key('Escape');
    });
  }

  // Botones de la tienda de Mejoras Permanentes
  for (let i = 1; i <= 5; i++) {
    const btn = document.getElementById(`btn-upg-${i}`);
    if (btn) {
      btn.addEventListener('click', () => {
        if (game) game.handle_key(`${i}`);
      });
    }
  }

  if (btnShopBack) {
    btnShopBack.addEventListener('click', () => {
      if (game) game.handle_key('Escape');
    });
  }

  // Botón de vistas modales (Stats, Logros, Offline)
  if (btnModalBack) {
    btnModalBack.addEventListener('click', () => {
      if (game) game.handle_key('Escape');
    });
  }

  // Guardar antes de cerrar o cambiar de pestaña
  window.addEventListener('visibilitychange', () => {
    if (document.hidden) {
      triggerSave();
    }
  });

  window.addEventListener('beforeunload', () => {
    triggerSave();
  });

  // Exportar partida a JSON
  if (btnExport) {
    btnExport.addEventListener('click', () => {
      if (!game) return;
      try {
        const json = game.export_save_json(currentTimestamp());
        const blob = new Blob([json], { type: 'application/json' });
        const url = URL.createObjectURL(blob);
        const a = document.createElement('a');
        a.href = url;
        a.download = 'save.json';
        document.body.appendChild(a);
        a.click();
        document.body.removeChild(a);
        URL.revokeObjectURL(url);
        flashSaveStatus('📥 Exportado');
      } catch (err) {
        alert(`Error al exportar partida: ${err}`);
      }
    });
  }

  // Importar partida desde JSON
  if (btnImport && fileInput) {
    btnImport.addEventListener('click', () => {
      fileInput.click();
    });

    fileInput.addEventListener('change', (e) => {
      const file = e.target.files[0];
      if (!file) return;

      const reader = new FileReader();
      reader.onload = (event) => {
        try {
          const content = event.target.result;
          const importedGame = WebGame.from_save(content, currentTimestamp());
          game = importedGame;
          triggerSave();
          flashSaveStatus('📤 Importado con éxito');
        } catch (err) {
          alert(`Error al importar archivo save.json: ${err}`);
        }
      };
      reader.readAsText(file);
      fileInput.value = '';
    });
  }

  // Reiniciar partida limpia
  if (btnReset) {
    btnReset.addEventListener('click', () => {
      if (confirm('¿Seguro que deseas reiniciar tu partida desde cero? Todo tu progreso actual será borrado.')) {
        if (game) {
          game.reset_game(currentTimestamp());
          flashSaveStatus('⚠️ Reiniciado');
        }
      }
    });
  }
}

// Inicialización
async function main() {
  try {
    await init();
    game = new WebGame(currentTimestamp());
    setupKeyboardListeners();
    setupTouchListeners();
    lastTime = performance.now();
    requestAnimationFrame(gameLoop);
  } catch (err) {
    console.error('Error al inicializar WebAssembly:', err);
    if (screenElem) screenElem.textContent = `Error al cargar WebAssembly:\n${err}`;
  }
}

main();
