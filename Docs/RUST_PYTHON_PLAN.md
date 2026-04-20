# Rust + Python Build Plan

This project now treats the product as one shared combat engine implemented in Rust, with Python handling camera gesture recognition.

## Product shape

One combat core, three rulesets:

- Solo combat
- Fitness mode
- Multiplayer duel/co-op

## Core pipeline

`camera / keyboard / network -> ActionCommand -> Rust simulation -> mode rules -> renderer / UI`

## Workspace layout

- `crates/lightsaber_core`
  - shared domain model
  - combat rules
  - scoring, combo, fitness, duel state
- `crates/lightsaber_runtime`
  - UDP camera command receiver
  - keyboard fallback parsing
  - protocol parsing
- `crates/lightsaber_app`
  - Bevy 2D playable app for solo / fitness / duel loop
- `Tools/gesture_bridge`
  - Python MediaPipe process that emits clean action commands

## Mode strategy

### Solo

- one player
- waves
- score
- combo

### Fitness

- same combat commands
- timed survival workout
- session metrics
- effort scoring

### Multiplayer

- same combat commands
- local duel first
- networked command sync later

## Immediate next implementation steps

1. Expand the Bevy layer with slash trails, sparks, audio, and screen shake.
2. Add explicit hitboxes, startup/recovery windows, and better lane targeting.
3. Expand `lightsaber_core` with enemy AI data loading, fitness session goals, and duel round transitions.
4. Add network transport for remote `ActionCommand` messages after local duel feels good.
