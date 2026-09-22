# AGENTS.md — Development Guidelines & System Architecture

## Project Overview

This project is an incremental / idle game written in Rust, styled entirely with ASCII and Unicode text. The core visual mechanic relies on smooth, sub-block progress bars using Unicode characters (8 levels of granularity per character cell).

The project is structured as a multi-target application intended to run first as a native Terminal CLI, with future ports to WebAssembly (Canvas/HTML or Xterm.js) and Desktop.

---

## Workspace Architecture

The repository is organized as a Cargo Workspace to enforce strict decoupling:

```text
├── Cargo.toml                  # Workspace manifest
├── crates/
│   ├── core/                  # Pure game logic (state, tick, resources, saves)
│   ├── ui_text/               # Agnostic text renderers, bar math & ASCII buffers
│   ├── cli/                   # Native terminal binary (Crossterm / Ratatui)
│   └── web/                   # (Future) WASM target bindings
```

### Critical Architecture Rules

1. **Purity of `core`:**
   - Must NEVER import terminal libraries (`crossterm`, `ratatui`, `std::io::stdout`).
   - Must NEVER handle OS inputs directly.
   - All state updates must be driven via a deterministic `tick(dt: f64)` function receiving delta time in seconds, along with explicit action intents (e.g., `enum Action { Click, BuyUpgrade(Id) }`).
   - Must support `serde` serialization for game persistence.

2. **Decoupled Visuals in `ui_text`:**
   - Visual helpers, string formatters, and sub-block text generation live here.
   - Outputs plain `String` buffers or structured text frames, agnostic of whether they are drawn to stdout, an ANSI buffer, or a Web canvas.

---

## Unicode Sub-Block Bar Specification

Progress bars must use Unicode Block Elements (`U+2588` through `U+258F`) to deliver sub-character rendering resolution (8 subdivisions per block).

### Block Table Reference

- `0/8`: ` ` (`\u{0020}`)
- `1/8`: `▏` (`\u{258F}`)
- `2/8`: `▎` (`\u{258E}`)
- `3/8`: `▍` (`\u{258D}`)
- `4/8`: `▌` (`\u{258C}`)
- `5/8`: `▋` (`\u{258B}`)
- `6/8`: `▊` (`\u{258A}`)
- `7/8`: `▉` (`\u{2589}`)
- `8/8`: `█` (`\u{2588}`)

### Implementation Standards

- Functions rendering bars must accept `(current: f64, max: f64, width_in_chars: usize) -> String`.
- Clamp inputs between `0.0` and `max` to prevent visual overflows.
- Pre-allocate string capacity: `String::with_capacity(width * 4 + padding)` to avoid dynamic reallocations during frame renders.
- Never use non-monospace assumptions. Every character must represent exactly 1 terminal column.

---

## Coding Standards & Idioms

- **Rust Edition:** 2021 or 2024.
- **Safety:** `#![forbid(unsafe_code)]` across all crates unless strictly required for low-level WASM/C bindings.
- **Floating Point & Idle Numbers:**
  - Standard resources should use `f64` for high precision and smooth interpolation.
  - Avoid NaN and infinities by asserting limits or using `.clamp()`.
  - For exponential scaling in late-game stages, isolate numeric representations behind a custom type (e.g., wrapper around `f64` with scientific notation or `num-bigint` when needed).
- **Error Handling:** Use `thiserror` for library crates (`core`, `ui_text`) and `anyhow` for binary targets (`cli`).

---

## Development Workflow & Commands

- **Check entire workspace:**

```bash
cargo check --workspace
```

- **Run CLI in development mode:**

```bash
cargo run -p cli
```

- **Execute unit tests:**

```bash
cargo test --workspace
```

- **Lint code:**

```bash
cargo clippy --workspace -- -D warnings
```
