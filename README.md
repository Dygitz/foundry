# Foundry

A browser-based alchemical factory/survival sandbox built with **Rust + WebAssembly** and **Bevy**.

Play the current build here: **https://dygitz.github.io/foundry/**

This repo is structured to keep simulation/game state independent from rendering and browser APIs:
- Core world/simulation types live in `simulation_core`
- Persistence contracts live in `persistence`
- IndexedDB implementation lives in `web_storage_indexeddb`
- Bevy frontend lives in `bevy_frontend`

---

## Features (current)

- Chunked world streaming + caching + eviction
- Deterministic terrain generation
- Ores/resources and mining (collected into inventory)
- Player movement + camera follow + mouse-wheel zoom
- Crafting menu + placeable structures (Athanor, Reliquary, Brass Arm, Extractor, Alembic, Crucible)
- Athanor smelting (ore -> calx), Alembic distillation (reagents -> essences), Crucible transmutation (essences -> components), and Reliquary storage UI
- Directional Brass Arms with inspectable internal buffers that move items between Reliquaries, Athanors, Extractors, Alembics, and Crucibles
- Persistent world data + inventory (IndexedDB, autosave + recovery)

## Controls

- Move: WASD / Arrow keys
- Mine / interact: Left click (resource tiles; open structure UI)
- Pick up placed structures: Hold right click on the structure
- Crafting/inventory: E to open/close; click recipe icons to craft
- Map: M to open/close; drag to pan; mouse wheel to zoom
- Hotbar: 1-0 to select crafted placeables; left click to place; Esc/right click clears selection
- Quick place: F selects Athanor, C selects Reliquary, I selects Brass Arm, D selects Extractor, L selects Alembic, X selects Crucible
- Rotate Brass Arm before placement: R
- Close UI panels: Esc or right click
- Zoom: Mouse wheel

## Reset your local world

Open browser DevTools → Application → IndexedDB → delete the game_worlds database

---

## Repo layout

- `simulation_core/`  
  Pure Rust game state + world/chunk data structures and deterministic generation.  
  **No Bevy**, **no browser APIs**.

- `persistence/`  
  Traits + types for world storage and codecs (stable contracts).

- `web_storage_indexeddb/`  
  `WorldStorage` implementation using **IndexedDB** (WASM only).

- `bevy_frontend/`  
  Bevy app: rendering, input, UI, async pumps, chunk streaming orchestration.

---

## Local development

### Prerequisites

- Rust (stable): https://rustup.rs
- WASM target:
  ```bash
  rustup target add wasm32-unknown-unknown
  ```
- Trunk:
  ```bash
  cargo binstall trunk
  ```

### Run the game locally
  ```bash
  trunk serve
  ```
  Then go to `http://127.0.0.1:8080`
