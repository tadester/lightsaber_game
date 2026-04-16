# Lightsaber Game

A working browser prototype for a webcam-powered AR Jedi combat simulator.

## What It Does

- Uses your webcam as the live scene background
- Runs MediaPipe hand tracking directly in the browser
- Builds a lightsaber from hand landmarks with heuristic math
- Detects a Force Push from left-hand forward motion using area growth and palm depth
- Spawns drones, checks saber collisions, applies push impulses, and renders particles
- Scales difficulty over time with faster drones and shorter respawn windows
- Tracks combo chains and score multipliers
- Uses procedural browser audio for saber hits, force bursts, and warning cues
- Keeps hand roles steadier with position, handedness, and temporal role memory
- Presents the whole system in a modern sci-fi UI

## Run It

This project should be served from a local web server so camera access and ES module imports work correctly.

```bash
python3 -m http.server 8000
```

Then open:

`http://localhost:8000`

## Controls

- Click `Start Camera`
- Keep your right-side hand in frame to drive the saber
- Point with your index finger to angle the blade
- Use your left-side hand and thrust it forward to trigger a Force Push
- Chain hits quickly to build combo multiplier and higher scores

## Notes

- The hand tracking runtime is loaded from the MediaPipe CDN in the browser
- A stable webcam distance helps the push heuristic behave more consistently
- Browser audio starts after user interaction when you click `Start Camera`
- The browser version is the fastest path to a complete demo; the same logic can later move into Rust, Bevy, Godot, or a websocket-based architecture
