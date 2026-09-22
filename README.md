# Mucking Around — El Arte de la Procrastinación Zen 🦥

Un juego incremental / *idle game* desarrollado en **Rust**, presentado en la terminal con gráficos de texto ASCII y barras de progreso fluidas usando caracteres de sub-bloques Unicode (con 8 niveles de granularidad por celda).

El objetivo es simple: acumular **Puntos de Flojera** (*Sloth Points*), desbloquear y optimizar actividades de procrastinación cotidiana, hacer clic compulsivo en un lapicero, reclamar distracciones espontáneas y, eventualmente, alcanzar una **Crisis Existencial** para renacer con **Epifanías** que potencian permanentemente tu desgano productivo.

---

## Índice

1. [Arquitectura del Workspace](#arquitectura-del-workspace)
2. [Compilación, Ejecución y Makefile](#compilación-ejecución-y-makefile)
3. [Controles y Modos de Juego](#controles-y-modos-de-juego)
   - [Sincronización de Niebla de Guerra (Terminal ↔ Web)](#sincronización-de-niebla-de-guerra-terminal--web)
4. [Mecánicas del Juego](#mecánicas-del-juego)
5. [Guía de Configuración Detallada](#guía-de-configuración-detallada)
   - [¿Dónde se configura el juego?](#dónde-se-configura-el-juego)
   - [El Exponente de Costo de Actividades (`cost_exponent`)](#el-exponente-de-costo-de-actividades-cost_exponent)
   - [Configuración de Actividades e Hitos](#configuración-de-actividades-e-hitos)
   - [Configuración del Lapicero (Clicker)](#configuración-del-lapicero-clicker)
   - [Configuración de Crisis Existencial (Prestigio)](#configuración-de-crisis-existencial-prestigio)
   - [Configuración de Distracciones y Frenesí](#configuración-de-distracciones-y-frenesí)
   - [Configuración de Persistencia y Progreso Offline](#configuración-de-persistencia-y-progreso-offline)
6. [Pruebas y Verificación de Calidad](#pruebas-y-verificación-de-calidad)

---

## Arquitectura del Workspace

El repositorio está organizado como un **Cargo Workspace** con separación estricta de responsabilidades:

```text
├── Cargo.toml                  # Manifiesto del Workspace
├── save.json                   # Archivo de persistencia de partida CLI
├── www/                        # Aplicación web estática (HTML5, CSS retro, JS ES6)
│   ├── index.html              # Pantalla terminal y controles táctiles
│   ├── styles.css              # Estilos CRT retro-modernos con tipografía monoespaciada
│   ├── main.js                 # Bucle 60 FPS, puente WASM, localStorage y eventos
│   └── pkg/                    # Paquete compilado WebAssembly (generado por wasm-pack)
├── crates/
│   ├── core/                  # Lógica pura del juego (estado, simulación determinista, fórmulas, persistencia serde)
│   ├── ui_text/               # Renderizado agnóstico a texto, barras sub-bloque Unicode, marcos de interfaz
│   ├── cli/                   # Binario de terminal interactivo (Crossterm, bucle a 60 FPS, manejo de eventos)
│   └── web/                   # Bindings WebAssembly (wasm-bindgen, localStorage, puente JS)
```

- **`core`**: Totalmente puro y desacoplado del sistema operativo y de terminales. No importa `crossterm` ni `std::io::stdout`. Las actualizaciones de estado se procesan mediante `tick(dt: f64)` recibiendo el delta de tiempo en segundos o acciones explícitas.
- **`ui_text`**: Transforma estados en buffers de `String` y caracteres Unicode (`U+2588` a `U+258F`), dibujable de manera idéntica en terminal nativa o en navegador web.
- **`cli`**: Gestiona el terminal crudo (*raw mode*), teclado no bloqueante, bucle a 60 FPS y persistencia en `save.json`.
- **`web`**: Compila a WebAssembly (`wasm32-unknown-unknown`), conectando la simulación determinista y el renderizado Unicode con el navegador web y `window.localStorage`.

---

## Compilación, Ejecución y Makefile

El proyecto incluye un archivo `Makefile` con atajos cómodos para todas las operaciones cotidianas:

| Comando | Acción |
| :--- | :--- |
| `make help` | Muestra la lista de comandos disponibles con sus descripciones. |
| `make run` / `make cli` | Compila e inicia el juego en la terminal nativa. |
| `make web` | Compila el paquete WASM y levanta el servidor web en `http://localhost:8080`. |
| `make web-build` | Compila únicamente el crate `crates/web` a WebAssembly con `wasm-pack`. |
| `make web-serve` | Levanta el servidor local HTTP en el puerto 8080 para la carpeta `www`. |
| `make test` | Ejecuta toda la suite de pruebas unitarias (`cargo test --workspace`). |
| `make check` | Comprobación rápida de sintaxis y tipos en todos los crates. |
| `make clippy` / `make lint` | Ejecuta el linter Clippy con `-D warnings`. |
| `make fmt` / `make fmt-fix` | Verifica o aplica el formateo de código con `rustfmt`. |
| `make clean` | Limpia los artefactos de compilación (`target/` y `www/pkg/`). |
| `make reset-save` | Elimina `save.json` para reiniciar la partida de la terminal. |

---

### Iniciar en Terminal (CLI nativo):
```bash
make run
# O directamente: cargo run -p cli
```

### Iniciar en Navegador Web (WebAssembly):
Requisito: tener instalado [`wasm-pack`](https://rustwasm.github.io/wasm-pack/installer/) (`cargo install wasm-pack`).

```bash
make web
# O manualmente:
# wasm-pack build crates/web --target web --out-dir ../../www/pkg
# python3 -m http.server 8080 --directory www
```
Abre tu navegador en `http://localhost:8080`.

> [!TIP]
> **Persistencia en Web e Interoperabilidad:**
> - La versión web guarda automáticamente tu progreso en `window.localStorage` cada 30 segundos y al cerrar o cambiar de pestaña.
> - Al volver a entrar tras horas ausente, el juego calcula automáticamente tu **progreso offline**.
> - Mediante los botones **📥 Exportar** y **📤 Importar**, puedes descargar tu partida en formato `save.json` o subir tu archivo de la terminal para continuar tu partida en el navegador (¡y viceversa!).

---

## Controles y Modos de Juego

### Panel Principal (Main Dashboard)
| Tecla | Acción |
| :--- | :--- |
| `[Espacio]` | **Hacer clic en el lapicero** (genera puntos y acelera la barra más cercana en 2%) o **reclamar distracción** si hay una activa. |
| `[D]` | **Reclamar distracción activa** directamente sin hacer clic en el lapicero. |
| `[1]` – `[5]` | **Desbloquear o subir de nivel** una actividad descubierta. |
| `[P]` | **Abrir diálogo de Crisis Existencial** (Prestigio) *(se desbloquea al acumular 1,000 puntos históricos)*. |
| `[U]` | **Abrir tienda de Mejoras Permanentes** *(requiere Epifanías obtenidas en una Crisis)*. |
| `[S]` | **Ver Estadísticas Existenciales** (tiempo procrastinado, clics, crisis, comparativas cómicas con la vida real). |
| `[A]` | **Ver Galería de Logros** desbloqueados y sus bonus pasivos. |
| `[q]` / `[Esc]` | **Guardar y salir limpiamente** del juego. |

---

### Sincronización de Niebla de Guerra (Terminal ↔ Web)

Siguiendo las reglas de arquitectura de `AGENTS.md`, la versión web respeta al 100% las mecánicas y el estado de la simulación:

1. **Niebla de Guerra en Actividades:**
   - En una partida nueva, **únicamente el botón `[1]` está visible**, ya que es la única actividad desbloqueada.
   - Los botones `[2]`, `[3]`, `[4]` y `[5]` permanecen ocultos hasta que acumulas suficientes Puntos de Flojera para revelar la siguiente actividad en el motor de juego (`GameState.activities[i].is_revealed`).
2. **Niebla de Guerra en Prestigio:**
   - Los botones `[P] Crisis` y `[U] Mejoras` solo se muestran en pantalla una vez que alcanzas el umbral de puntos históricos (1,000 pts) y el prestigio queda revelado (`prestige_revealed`).
3. **Controles Contextuales según Pantalla Activa:**
   - **En la Crisis Existencial (`ActiveView::PrestigeDialog`):** La barra de actividades se oculta y aparecen los botones de decisión `[S] Confirmar Crisis` y `[N / Esc] Cancelar`.
   - **En la Tienda de Mejoras (`ActiveView::PermanentUpgradesShop`):** Solo se muestran los botones de las mejoras permanentes que hayan sido descubiertas, junto con `[U / Esc] Volver al juego`.
   - **En Estadísticas y Logros:** Se ofrece un botón directo para volver al juego.

---

## Mecánicas del Juego

1. **Actividades de Procrastinación**:
   - Cada ciclo completo de la barra de progreso otorga Puntos de Flojera.
   - Las actividades suben de nivel con `[1-5]`, aumentando linealmente sus recompensas.
   - **Hitos de velocidad (Milestones)**: Al alcanzar los niveles **25, 50, 100, 200, 300, 400, 500, 1000, 5000 y 9999**, la duración base de la actividad se divide por multiplicadores exponenciales (2x, 4x, 8x, 16x...).
   - **Modo ⚡ TURBO**: Cuando la duración efectiva de una actividad baja de `0.1s`, el motor conmuta la actividad a modo continuo, mostrando un ecualizador oscilante animado y sumando puntos fluidamente en tiempo real (+XX.XX pts/seg).

2. **Lapicero Anti-Estrés**:
   - Hacer clic en el lapicero (`[Espacio]`) genera puntos base más una bonificación escalada por los puntos históricos de flojera.
   - Cada clic le regala un **2% de avance instantáneo** a la barra activa más cercana a completarse.

3. **Distracciones Inesperadas**:
   - De forma aleatoria (entre 60 y 180 segundos), surge una distracción en pantalla (un video de sartenes de hierro, un meme en WhatsApp, un quiz de pan dulce...).
   - Si la reclamas a tiempo, puedes activar un **Frenesí Multiplicador** (ej. 7x producción por 25s), recibir **Flojera Instantánea** o experimentar un **Salto en el Tiempo** (*Time Warp*).

4. **Crisis Existencial (Prestigio) y Epifanías**:
   - Reinicia los puntos actuales y niveles de actividades a cambio de **Epifanías**.
   - Cada Epifanía no gastada otorga un **+10% de producción global acumulativo**.
   - Las Epifanías se pueden gastar en mejoras permanentes en la tienda (`[U]`).

5. **Progreso Offline**:
   - Al cerrar el juego y volver a entrar, se calcula matemáticamente en tiempo $O(1)$ la producción obtenida mientras estabas ausente (a un 50% de eficiencia, hasta un máximo de 12 horas).

---

## Guía de Configuración Detallada

### ¿Dónde se configura el juego?

El juego se puede configurar a través de dos mecanismos:

1. **En tiempo de ejecución / Partida guardada (`save.json`)**:
   - Ideal para ajustar valores en una partida en curso sin necesidad de recompilar Rust.
   - **IMPORTANTE**: Cierra el juego antes de editar `save.json` para que el autoguardado de 30 segundos no sobreescriba tus modificaciones.
2. **En el código fuente Rust (`crates/core/src/`)**:
   - Ideal para cambiar los valores por defecto de partidas nuevas, modificar la lógica de cálculo o redefinir el balance general. Requiere recompilar con `cargo run -p cli`.

---

### El Exponente de Costo de Actividades (`cost_exponent`)

> [!NOTE]
> **¿Por qué no aparece el exponente de costo en `ActivityConfig` dentro de `roster.rs`?**
>
> En `crates/core/src/roster.rs` solo se define la configuración inicial fija de cada actividad (`id`, `name`, `lore`, `duration`, `cost` base, `reward`, `milestones`).
>
> El exponente de costo reside en **`ActivityState.cost_exponent`** en [`crates/core/src/lib.rs`](crates/core/src/lib.rs) porque **es una propiedad de estado dinámica que puede ser alterada en tiempo de juego por el sistema de prestigio** (específicamente por la mejora permanente *"Optimización del Desgano"* / `cost_optimization`).

#### La Fórmula Matemática de Costo

El costo del siguiente nivel se calcula mediante:

$$\text{Costo}(\text{nivel}) = \begin{cases} \text{costo\_desbloqueo} & \text{si nivel} = 0 \\ \text{costo\_base} \times (\text{cost\_exponent})^{\text{nivel}} & \text{si nivel} \ge 1 \end{cases}$$

*(donde `costo_base` es `1.0` si el costo inicial era `0.0`, o el valor de `cost` de la actividad).*

- **Valor inicial estándar**: `1.15` (cada nivel cuesta un 15% más que el anterior).
- **Con la mejora "Optimización del Desgano"**: Se reduce a `1.12` (cada nivel cuesta solo un 12% más).

#### ¿Cómo configurarlo?

##### Opción A: Modificarlo en una partida activa (`save.json`)
1. Cierra el juego.
2. Abre `save.json` en tu editor.
3. Dentro del arreglo `"activities"`, localiza la propiedad `"cost_exponent"` de cada actividad que desees ajustar:

```json
{
  "activities": [
    {
      "config": {
        "id": "wait_bar",
        "name": "Esperar a que cargue la barrita",
        ...
      },
      "progress": 0.0,
      "level": 30,
      "duration_divisor": 1.0,
      "cost_exponent": 1.15,   <--- Cambia este valor (ej. 1.08 para escalar más lento)
      "is_revealed": true
    },
    ...
  ]
}
```
4. Guarda el archivo e inicia el juego con `cargo run -p cli`.

> [!WARNING]
> Ten en cuenta que si en tu partida compras la mejora permanente *"Optimización del Desgano"* o ejecutas un reinicio por *Crisis Existencial*, el método `GameState::apply_permanent_upgrade_effects()` en Rust actualizará automáticamente el `cost_exponent` de todas las actividades a `1.12` (si tienes la mejora) o `1.15` (si no la tienes). Si deseas cambiar el comportamiento permanente, usa la Opción B.

##### Opción B: Modificarlo en el Código Fuente Rust (Para nuevas partidas y mejoras)
Si deseas cambiar permanentemente el exponente para todas las partidas futuras y asegurar que no sea sobreescrito por el prestigio:

1. Abre [`crates/core/src/lib.rs`](crates/core/src/lib.rs):
   - **Función por defecto** (alrededor de la línea 142):
     ```rust
     fn default_cost_exponent() -> f64 {
         1.15 // <-- Cambia el valor predeterminado aquí
     }
     ```
   - **Constructor de actividad** (alrededor de la línea 158):
     ```rust
     Self {
         config,
         progress: 0.0,
         level,
         duration_divisor: 1.0,
         cost_exponent: 1.15, // <-- Cambia aquí
         is_revealed,
     }
     ```
   - **Efecto de la mejora permanente** (alrededor de la línea 686 en `apply_permanent_upgrade_effects`):
     ```rust
     let cost_exponent = if self.has_permanent_upgrade("cost_optimization") {
         1.12 // <-- Exponente cuando se compra "Optimización del Desgano"
     } else {
         1.15 // <-- Exponente base
     };
     ```
2. (Opcional) Si cambias los valores, actualiza la descripción en [`crates/core/src/prestige.rs`](crates/core/src/prestige.rs) (líneas 68 y 124) para que la tienda refleje el nuevo porcentaje en el texto descriptivo.

---

### Configuración de Actividades e Hitos

Las 5 actividades base y sus hitos están configuradas en [`crates/core/src/roster.rs`](crates/core/src/roster.rs):

```rust
ActivityConfig {
    id: "wait_bar",
    name: "Esperar a que cargue la barrita",
    lore: "La vida se mide en barras de carga que sospechosamente se quedan en 99%.",
    duration: 5.0,  // Duración base del ciclo en segundos
    cost: 0.0,      // Costo para desbloquear (0.0 = desbloqueada de inicio)
    reward: 1.0,    // Puntos otorgados por ciclo en Nivel 1
    milestones: default_milestones(), // Niveles que otorgan aceleración (25, 50, 100...)
}
```

- Para cambiar la velocidad a ciertos niveles, modifica `default_milestones()` en `crates/core/src/roster.rs`.
- En `save.json`, puedes alterar `duration`, `cost` o `reward` individualmente para cada actividad bajo su objeto `"config"`.

---

### Configuración del Lapicero (Clicker)

La mecánica del lapicero está definida en [`crates/core/src/pen.rs`](crates/core/src/pen.rs). Puedes modificarla directamente en `save.json` bajo la clave `"pen_config"`:

```json
"pen_config": {
  "base_reward": 0.25,              // Puntos otorgados en cada clic base
  "lifetime_scaling_factor": 0.0001, // Multiplicador escalado por puntos históricos
  "bar_speedup_percentage": 0.02,   // 0.02 = 2% de avance regalado a la barra más cercana
  "sound_effects": [
    "*¡Tac!*",
    "*¡Clic!*",
    "*¡Tac-tac-tac!*",
    "*¡Crack! (casi rompes el resorte)*"
  ]
}
```

---

### Configuración de Crisis Existencial (Prestigio)

Definida en [`crates/core/src/prestige.rs`](crates/core/src/prestige.rs) y configurable en `save.json` bajo `"prestige_config"`:

```json
"prestige_config": {
  "base_cost": 1000.0,             // Puntos históricos necesarios para desbloquear Epifanías
  "exponent": 0.5,                 // 0.50 = raíz cuadrada: floor((puntos / base_cost) ^ 0.5)
  "default_bonus_per_point": 0.1   // 0.10 = +10% de producción por cada Epifanía no gastada
}
```

#### Tienda de Mejoras Permanentes
Las mejoras disponibles y sus costos en Epifanías se definen en `crates/core/src/prestige.rs`:
- **`muscle_memory`** (2 Epifanías): Inicia la primera actividad en Nivel 10 tras reiniciar.
- **`cost_optimization`** (5 Epifanías): Reduce el exponente de costo de las actividades a 1.12.
- **`zen_enlightenment`** (10 Epifanías): Aumenta el bono por Epifanía libre de +10% a +15%.
- **`eternal_sloth`** (20 Epifanías): Todas las actividades son permanentemente un 25% más rápidas (`duration_divisor: 1.25`).
- **`autopilot`** (50 Epifanías): Compra automáticamente 1 nivel de la actividad más barata cada 2 segundos.

---

### Configuración de Distracciones y Frenesí

Definida en [`crates/core/src/distraction.rs`](crates/core/src/distraction.rs) y configurable en `save.json` bajo `"distraction_config"`:

```json
"distraction_config": {
  "min_spawn_interval": 60.0,  // Tiempo mínimo en segundos para que aparezca una distracción
  "max_spawn_interval": 180.0, // Tiempo máximo en segundos para que aparezca una distracción
  "roster": [ ... ]            // Catálogo de eventos de distracción disponibles
}
```

Cada evento en `roster` cuenta con un tiempo de vigencia (`time_to_claim`) y un tipo de recompensa:
- `Frenzy`: `{ "multiplier": 7.0, "duration_secs": 25.0 }` (multiplica toda la producción por 7x durante 25s).
- `InstantSloth`: `{ "percentage_of_current": 0.10, "min_flat": 50.0 }` (otorga 10% de tu saldo actual o 50 pts).
- `TimeWarp`: `{ "simulated_seconds": 30.0 }` (simula 30 segundos instantáneos de producción).

---

### Configuración de Persistencia y Progreso Offline

Definida en [`crates/core/src/persistence.rs`](crates/core/src/persistence.rs) y configurable en `save.json` bajo `"persistence_config"`:

```json
"persistence_config": {
  "auto_save_interval": 30.0,  // Intervalo en segundos del autoguardado en segundo plano
  "offline_efficiency": 0.50,  // Eficiencia de producción fuera de línea (0.50 = 50%)
  "max_offline_hours": 12.0,   // Límite máximo de horas de progreso offline calculadas
  "save_file_name": "save.json" // Ruta o nombre del archivo de guardado
}
```

---

## Pruebas y Verificación de Calidad

El proyecto cuenta con una amplia suite de pruebas unitarias que validan el cálculo del progreso, las fórmulas de costos exponenciales, los hitos y la persistencia:

```bash
cargo test --workspace
```

Si deseas reiniciar tu progreso desde cero para comenzar una partida limpia:
```bash
rm save.json
cargo run -p cli
```
