# Gesture Bridge

This Python process is the camera-side bridge for the Rust runtime.

## Responsibilities

- Opens the webcam
- Uses MediaPipe Hands
- Smooths simple gesture heuristics
- Sends final action commands over UDP to Rust

## Output Protocol

```json
{
  "gesture": "slash_left",
  "confidence": 0.93,
  "timestamp": 1710000000.1,
  "playerId": 0
}
```

## Run

```bash
python3 -m venv .venv
source .venv/bin/activate
pip install -r requirements.txt
python mediapipe_bridge.py
```

Then start the Rust app:

```bash
cargo run -p lightsaber_app
```

Choose `Python Camera` or `Keyboard + Python` in the game menu. The Rust app will start this bridge from `.venv` automatically.
