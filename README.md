# LUNA

A Windows computer-use agent prototype. I built it to explore how far screen
understanding can go with hand-written computer vision instead of ML models.

The pipeline: capture the screen, detect UI elements with hand-written CV
(Sobel edge detection, rectangle detection, classification by aspect ratio
and brightness), plan actions from a natural-language command with simple
rules, and execute mouse/keyboard actions through a safety layer with a
pattern blocklist and rate limiting. No ML frameworks, no GPU, no async
runtime — seven small crates.

## Demo

<!-- Screen-recording GIF goes here: cargo run, then `analyze` and a command like "click the save button" -->

## Status: working prototype

What works today:

- The whole pipeline compiles and runs end to end:
  command -> safety check -> capture -> CV analysis -> action planning -> guarded execution
- Hand-written CV: Sobel edge detection, rectangle detection, element
  classification (button / text field / icon) from aspect ratio, area, and
  brightness (`src/ai/mod.rs`, `src/utils/image_processing.rs`)
- Safety layer: regex blocklist for destructive commands (`format c:`,
  `rm -rf`, ...), per-action validation, and rate limiting at 10 actions/sec
  and 100/min (`src/core/safety.rs`, `src/input/mod.rs`)
- Template-based character recognition scaffolding (`src/vision/text_recognition.rs`)
- Overlay/highlight data structures with an animation system (`src/overlay/`)
- 73 unit tests and 6 doc tests pass; CI runs `cargo check --all-targets`
  and `cargo test` on every push

What does not work yet:

- Screen capture is a placeholder. It returns a synthetic test pattern
  instead of calling GDI/DXGI (`src/vision/screen_capture.rs`), so the
  pipeline currently runs against that pattern, not the live screen.
- Input injection is a placeholder. Actions pass through the safety and
  rate-limit checks and are then logged, not performed — there are no
  SendInput calls yet (`src/input/mod.rs`).
- Detection is untuned. With default thresholds the detector finds few or
  no elements on the synthetic pattern.
- No GUI. The binary is a REPL.

In short: the parts I find interesting — the CV algorithms, the safety
design, the pipeline architecture — are real and tested. The OS integration
is stubbed and is the obvious next step.

## Architecture

```
src/
├── main.rs           REPL entry point (analyze / stats / free-text commands)
├── lib.rs            library API: init(), analyze_current_screen(), ...
├── core/
│   ├── mod.rs        Luna coordinator: command -> capture -> analyze -> validate -> execute
│   ├── safety.rs     SafetySystem: command and action blocklist validation
│   ├── config.rs     JSON config (safety, vision, input, logging sections)
│   └── error.rs      error types
├── ai/               screen analysis and rule-based action planning
├── vision/           screen capture (stub), UI detection, text recognition
├── input/            InputController: safety check + rate limit -> (stubbed) OS input
├── overlay/          visual feedback structures and animations
└── utils/            geometry, image processing (Sobel, threshold, crop), logging
```

Dependencies: `image`, `serde`, `serde_json`, `anyhow`, `log`, `regex`,
`dirs`, plus `env_logger` behind the optional `logging` feature.

## Build and run

Requires a Rust toolchain (stable).

```
git clone https://github.com/sushiionwest/LUNA.git
cd LUNA
cargo build --release
cargo test
cargo run
```

The REPL accepts:

```
analyze              capture and analyze the screen
stats                processing statistics
quit                 exit
<anything else>      treated as an automation command, e.g. "click the save button"
```

## History

Earlier versions of this repo carried a second, parallel ML-based
implementation (CLIP/Florence/SAM/TrOCR via candle) that never resolved its
dependencies, plus a stack of AI-generated marketing documents claiming
benchmarks and releases that never existed. In July 2026 I deleted all of
it, kept the one codebase that matches what I actually set out to build,
and fixed it until it compiled and its tests passed. Everything removed is
still in git history.

## License

MIT — see [LICENSE](LICENSE).
