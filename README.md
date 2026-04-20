# Lightsaber Game

This repo is now oriented around a Rust + Python architecture:

- Rust owns the shared combat engine, mode rules, and runtime loop
- Python owns MediaPipe hand tracking and emits clean gameplay actions
- Bevy renders the current playable 2D MVP window
- the browser prototype is still available for visual experimentation

## Rust foundation

The Rust side is split so solo, fitness, and multiplayer share one command and combat model from day one.

Key files:

- [`Cargo.toml`](/Users/ktr/Documents/GitHub/lightsaber_game/Cargo.toml)
- [`Docs/RUST_PYTHON_PLAN.md`](/Users/ktr/Documents/GitHub/lightsaber_game/Docs/RUST_PYTHON_PLAN.md)
- [`crates/lightsaber_core/src/lib.rs`](/Users/ktr/Documents/GitHub/lightsaber_game/crates/lightsaber_core/src/lib.rs)
- [`crates/lightsaber_core/src/simulation.rs`](/Users/ktr/Documents/GitHub/lightsaber_game/crates/lightsaber_core/src/simulation.rs)
- [`crates/lightsaber_runtime/src/udp_bridge.rs`](/Users/ktr/Documents/GitHub/lightsaber_game/crates/lightsaber_runtime/src/udp_bridge.rs)
- [`crates/lightsaber_app/src/main.rs`](/Users/ktr/Documents/GitHub/lightsaber_game/crates/lightsaber_app/src/main.rs)

## Python gesture bridge

The camera service is here:

- [`Tools/gesture_bridge/mediapipe_bridge.py`](/Users/ktr/Documents/GitHub/lightsaber_game/Tools/gesture_bridge/mediapipe_bridge.py)
- [`Tools/gesture_bridge/README.md`](/Users/ktr/Documents/GitHub/lightsaber_game/Tools/gesture_bridge/README.md)

It detects gestures and sends final commands like `slash_left`, `slash_right`, `force_push`, and `guard_start` over UDP to the Rust runtime.

## Run the Rust app

```bash
cargo run -p lightsaber_app
```

Modes:

- `cargo run -p lightsaber_app`
- `cargo run -p lightsaber_app -- fitness`
- `cargo run -p lightsaber_app -- duel`

Player 1 keyboard controls:

- `A` or `slash_left`
- `D` or `slash_right`
- `W` or `force_push`
- `S` or `guard_start`
- `guard_end`

Player 2 duel controls:

- `J` slash left
- `L` slash right
- `I` force push
- `K` guard

What you get now:

- windowed 2D arena
- parallax background
- solo drone combat
- fitness HUD metrics
- local 1v1 duel rules
- UDP camera-command intake from Python

## Run the Python bridge

```bash
cd Tools/gesture_bridge
python3 -m venv .venv
source .venv/bin/activate
pip install -r requirements.txt
python mediapipe_bridge.py
```

## Browser prototype

The browser prototype is still here if we want to keep iterating visually:

- [`index.html`](/Users/ktr/Documents/GitHub/lightsaber_game/index.html)
- [`styles.css`](/Users/ktr/Documents/GitHub/lightsaber_game/styles.css)
- [`app.js`](/Users/ktr/Documents/GitHub/lightsaber_game/app.js)
