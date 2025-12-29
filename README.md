# Foundry

A browser-based factory/survival sandbox built with **Rust + WebAssembly** and **Bevy**.

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
- Player movement + camera follow
- Persistent world data (IndexedDB)
- Persistent inventory (IndexedDB)
- Ores/resources and mining (collected into inventory)

## Controls

- Move: WASD / Arrow keys
- Mine: Left click (on resource tiles)

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