---
title: "Report 008: Epoch 8, 1,000-Event 8-Voice Polyphonic + CC Sweep Stress Benchmark, Non-Invasive Hardware Verification CLI, Documentation & Release Polish"
date: 2026-11-06 03:37:00
project: novation-peak-pigs
project_dir: /home/renick/antigravity/novation-peak-pigs
report_number: 8
subject: "1,000-Event 8-Voice Polyphonic + CC Sweep Stress Benchmark, Non-Invasive Hardware Verification CLI, Documentation & Release Polish"
operator: "Antigravity (Gemini 3.8 Flash)"
follows: ["report-007.md"]
tldr: "Epoch 8 completes the final milestone of the Novation Peak Manager workspace. In src/stress_bench.rs, we implemented a 1,000-event polyphonic note and 9-lane continuous controller sweep stress benchmark verifying zero dropped notes, strict 8-voice hardware polyphony budget compliance, zero stuck voices on panic, and strict safeSweepRange enforcement across CCs 29, 36, 79, 80, 104, and 110 at over 19,000 events per second. In src/main.rs and src/hardware_verify.rs, we added comprehensive non-blocking CLI flags (--verify-hardware, --dry-stress, --export-shared-schemas, --synth, --midi-in, --midi-out, --channel, --preset, --headless-server, --port) supporting headless OSC networking and exporting 26 portable JSON schemas across the 5-synth ecosystem. In README.md, we authored complete documentation covering the 5-synth architecture, the 74-CC parameter map, +32 bipolar dual-CC pairs, 14-bit NRPNs, SysEx checksum codecs, and the konduktiva-peak live-coding API. All 203 automated tests (177 Rust and 26 Node.js) pass cleanly with zero compiler warnings."
tests_total: 203
qa_verdict: PASS
---

# Prompt

```
[FORMAL INTENT]: Epoch 8 of 8: 1,000-Event 8-Voice Polyphonic + CC Sweep Stress Benchmark, Non-Invasive Hardware Verification CLI (--verify-hardware, --dry-stress, --export-shared-schemas), Documentation & Release Polish (Target: src/stress_bench.rs; Related: src/main.rs, README.md). Implement the 1,000-event 8-voice polyphonic note + 9-lane CC sweep stress benchmark in src/stress_bench.rs verifying zero dropped notes, 8-voice polyphony budget compliance, zero stuck voices on panic(), and strict safeSweepRange enforcement on CCs 29/36/79/80/104/110. Add non-blocking CLI flags (--verify-hardware, --dry-stress, --export-shared-schemas, --synth, --midi-in, --midi-out, --channel, --preset, --headless-server, --port) in src/main.rs, document the complete 5-synth architecture, 74-CC/NRPN/SysEx map, and konduktiva-peak live-coding API in README.md, and verify zero compiler warnings across all crates and Node.js packages. Ensure all workspace tests pass (CARGO_BUILD_JOBS=2 cargo test --workspace and npm test). Write reports/report-008.md and compile it with /home/renick/claude/html-report-generator/build-report.sh --pdf.
```

# Content

## 1. Overview and Everyday Purpose

When a musician performs live on stage or records in a recording studio, their synthesizer must remain completely reliable. If an automated music sequencer bombards a hardware synthesizer with hundreds of musical notes and rapid knob turns every second, software bugs can cause notes to hang indefinitely, voice allocation circuits to crash, or extreme resonance settings to produce sudden ear-piercing audio shrieks.

Epoch 8 provides the ultimate stress-testing, safety verification, and operational tooling for the Novation Peak 8-voice polyphonic synthesizer manager. To understand what this subsystem accomplishes, consider these everyday parallels:

1. The Busy Airport Runway (8-Voice Polyphony Budget): Imagine an airport with exactly eight landing runways. If eight airplanes are currently parked on the runways and a ninth airplane approaches to land, air traffic control cannot simply crash the planes together. Instead, air traffic control must systematically clear the oldest parked plane to a hangar (graceful voice stealing) so the new plane can land safely. Our polyphonic voice engine guarantees that no matter how many note commands arrive, exactly eight hardware voices are managed with zero crashes and zero dropped notes.
2. The Engine Speed Governor (Acoustic Safety Clamping): Heavy trucks have speed governors that prevent drivers from accelerating past safe highway speeds, protecting the engine and vehicle occupants from catastrophic failure. In synthesis, turning filter resonance, analogue distortion, or delay feedback to maximum can produce runaway audio screaming that destroys studio monitor speakers and damages hearing. Our safe sweep range engine acts as an automatic governor, strictly capping dangerous dial values before they reach the sound hardware.
3. The Emergency Fire Sprinkler Cutoff (Zero Stuck Voices on Panic): In a building with fire alarms, pulling the emergency lever immediately triggers safety doors and halts machinery. In electronic music, an emergency panic command must immediately cut all sound, reset every gate, and silence every audio oscillator. Our panic verification ensures that zero voices remain stuck sounding after a reset.
4. Universal Blueprint Standards (Shared Schema Export): When parts are manufactured in different factories, every engineer works from identical standard blueprints. Our schema exporter exports 26 portable JSON schema definitions and profile documents, ensuring that desktop applications, web dashboards, and live-coding scripts across five distinct synthesizers agree on every data structure.

We define all specialized industry concepts on first use:

- Polyphony: The capability of an electronic synthesizer to play multiple distinct musical pitches simultaneously. The Novation Peak has eight independent voice channels.
- Voice Stealing: The algorithmic process of reallocating an active synthesizer voice to play a new incoming musical note when all hardware voices are already sounding.
- Continuous Controller (CC): A standardized MIDI message that alters a continuous sound parameter (such as volume, filter brightness, or echo level) across 128 steps (0 to 127).
- Continuous Sweep: A stream of continuous controller values sent in rapid succession to smoothly modulate a parameter over time.
- Panic: A specialized MIDI procedure that transmits All-Notes-Off, All-Sound-Off, and Reset-Controllers commands to silence all active audio generators instantly.
- Non-Registered Parameter Number (NRPN): An extended MIDI command protocol using 14-bit addressing to control up to 16,384 distinct synthesizer settings.
- System Exclusive (SysEx): Manufacturer-specific data packets used to transmit complete sound programs and hardware configurations between computers and synthesizers.
- JSON Schema: A formal specification describing the structure, required fields, and acceptable value types for JSON documents.

The following table connects these everyday goals directly to their technical code implementations:

| Everyday Practical Goal | Technical Implementation | Code Path and Symbol |
| :--- | :--- | :--- |
| Enforce 8-voice hardware limit | Dynamic 8-Voice Allocator & Voice Stealer | `PeakSynthEngine::note_on`, `src/peak_synth.rs` |
| Execute 1,000-event stress test | Polyphonic Note & CC Benchmark Suite | `run_stress_benchmark`, `src/stress_bench.rs` |
| Eliminate stuck sounding notes | Instant Voice Gate Release & Filter Reset | `PeakSynthEngine::panic`, `src/peak_synth.rs` |
| Protect hearing and speakers | Conservative CC Value Safety Clamping | `safe_sweep_range`, `synth_midi_osc::routing` |
| Inspect hardware without sound | Non-Invasive Procfs ALSA Scanner | `verify_system_hardware`, `src/hardware_verify.rs` |
| Export universal data schemas | Multi-Crate Portable Schema Exporter | `export_shared_schemas`, `src/hardware_verify.rs` |
| Command-line tool operation | Non-Blocking Flag Processing Parser | `parse_cli_args`, `src/main.rs` |
| Complete technical manual | Markdown Architecture & CC Specification | `README.md` |

---

## 2. The 1,000-Event Polyphonic and 9-Lane CC Stress Benchmark (`src/stress_bench.rs`)

In `src/stress_bench.rs`, we designed and implemented a stress benchmark that bombards the synthesis engine with a dense stream of polyphonic note triggers and continuous controller sweeps.

### 2.1 The Four Invariants Tested

The benchmark enforces four strict invariants across every event:
1. Zero Dropped Notes: Every note-on event requested by the sequence must either receive a free hardware voice or be gracefully assigned through the voice-stealing allocator. Not a single note command is ignored or dropped.
2. Polyphony Budget Compliance: At no point during the 1,000 events may the active voice count exceed the physical hardware limit of eight voices (`active_voice_count <= 8`).
3. Zero Stuck Voices on Panic: Whenever `panic()` is triggered (both during an intermediate mid-stream test at event 500 and at the conclusion of the test), all active voice gates and envelope levels must immediately drop to zero.
4. Strict Safe Sweep Range Enforcement: All continuous controller sweeps across the nine test lanes must respect the safety bounds defined in `safe_sweep_range`, with zero clamp violations.

### 2.2 The Nine Continuous Sweep Lanes and Six Strict Safety Caps

The benchmark simultaneously sweeps nine continuous controller lanes while notes are sounding. Six of these lanes target parameters capable of creating dangerous speaker overload or self-oscillation:

| Sweep Lane | CC Number | Target Parameter | Raw Range | Strict Clamped Safe Bound | Safety Purpose |
| :--- | :--- | :--- | :--- | :--- | :--- |
| Lane 1 | CC 29 | Filter Cutoff | 0..127 | 0..127 | Full spectrum sweep |
| Lane 2 | CC 36 | Post-Filter Drive | 0..127 | **0..50** (39.4%) | Prevents amplifier rail clipping |
| Lane 3 | CC 79 | Filter Resonance | 0..127 | **0..62** (48.8%) | Prevents runaway self-oscillation |
| Lane 4 | CC 80 | Pre-Filter Overdrive | 0..127 | **0..50** (39.4%) | Prevents harsh input waveshaping |
| Lane 5 | CC 104 | Analogue Distortion | 0..127 | **0..48** (37.8%) | Prevents square-wave harmonic overload |
| Lane 6 | CC 110 | Delay Feedback | 0..127 | **0..70** (55.1%) | Prevents infinite echo feedback build |
| Lane 7 | CC 108 | Delay Time / Sync | 0..127 | 0..90 | Keeps delay send within headroom |
| Lane 8 | CC 112 | Reverb Level | 0..127 | 0..90 | Prevents wet signal audio wash |
| Lane 9 | CC 78 | Mod Env 1 Depth | 0..127 | 0..127 | Expressive envelope modulation |

### 2.3 Benchmark Performance and Telemetry Output

When executed on the test workstation, the 1,000-event benchmark executes in 52.2 milliseconds, demonstrating a throughput exceeding 19,000 events per second:

```text
=================================================================
  Novation Peak 1,000-Event Polyphonic & CC Stress Benchmark
=================================================================
  Total Events Executed    : 1000
  Note-On Events           : 306
  Note-Off Events          : 182
  9-Lane CC Sweep Events   : 512
  Dropped Notes (budget)   : 0 (Required: 0)
  Peak Active Voices       : 8 (Hardware limit: 8)
  Voice Steals Handled     : 137
  Safety Clamp Violations  : 0 (Required: 0)
  Stuck Voices After Panic : 0 (Required: 0)
  Execution Duration       : 52203 us (52.20 ms)
  Throughput               : 19155.7 events/sec
-----------------------------------------------------------------
  VERDICT                  : PASS
=================================================================
```

---

## 3. Non-Blocking CLI Flags and Hardware Verification (`src/main.rs`)

In `src/main.rs` and `src/hardware_verify.rs`, we expanded the command-line interface to support non-blocking operational modes suitable for automated testing, continuous integration, headless servers, and hardware verification:

### 3.1 Supported CLI Flags

1. `--verify-hardware`: Executes non-invasive inspection of system soundcards via `/proc/asound/cards` and `/proc/asound/seq/clients`. Safely detects Focusrite Peak (`hw:3,0,0`), Behringer UMC1820 (`hw:0,0,0`), and Korg nanoKONTROL2 (`hw:2,0,0`).
2. `--dry-stress`: Runs the 1,000-event polyphonic note and 9-lane CC sweep benchmark, validates all safety invariants, prints the formatted report, and exits cleanly.
3. `--export-shared-schemas [DIR]`: Exports 26 portable JSON schema and device profile files across the 5-synth ecosystem to a target directory (defaulting to `./schemas`).
4. `--synth <NAME>`: Configures the target synthesizer profile (default: `novation-peak`).
5. `--midi-in <DEVICE>`: Configures the target MIDI input device path.
6. `--midi-out <DEVICE>`: Configures the target MIDI output device path.
7. `--channel <1-16>`: Configures the target MIDI channel (default: channel 1, wire index 0).
8. `--preset <NAME_OR_INDEX>`: Queries the 512-patch preset library by name or bank slot index and prints key parameters.
9. `--headless-server`: Binds a non-blocking UDP Open Sound Control (OSC) server on the specified port, performs a readiness poll, and reports status.
10. `--port <PORT>`: Sets the UDP network port for OSC communication (default: 9100).
11. `--audit-geometry`: Runs headless window geometry auditing at 1280x800 and 1920x1080 to ensure zero panel overlap in the user interface.
12. `--gui`: Launches the native Two-Bowls egui desktop graphical user interface.
13. `--dry-run`: Enforces read-only discovery mode.
14. `-h`, `--help`: Prints comprehensive command-line help.

### 3.2 Exported Ecosystem Schemas

The `--export-shared-schemas` command exports 26 canonical schema and profile files:
- Synthesizer Profiles (5 synths): `novation-peak.profile.json`, `asm-hydrasynth.profile.json`, `novation-bass-station-2.profile.json`, `behringer-td3-md.profile.json`, `clavia-nord-drum-2.profile.json`.
- Hardware Profiles: `novation-peak.hardware.json`, `asm-hydrasynth.hardware.json`, `clavia-nord-drum-2.hardware.json`, `korg-nanokontrol2.hardware.json`, `novation-bs2.hardware.json`.
- Controller Mappings: `korg-nanokontrol2.profile.json`, `akai-midimix.profile.json`, `korg-nanokontrol2-nd2.mapping.json`.
- UI Themes: `novation-peak.theme.json`, `asm-hydrasynth.theme.json`, `nkeditor.theme.json`, `bs2-td3.theme.json`, `clavia-nord-drum-2.theme.json`.
- Core Data Schemas: `synth-patch.schema.json`, `device-profile.schema.json`, `controller-mapping.schema.json`, `theme-config.schema.json`, `stored-device-profile.schema.json`, `dual-midi-router.schema.json`, `peak-midi-router.schema.json`, `unified-ecosystem-bundle.json`.

---

## 4. Documentation and Ecosystem Architecture (`README.md`)

We authored a complete, standalone `README.md` in the project root documenting:
1. Complete Five-Synthesizer Architecture: Explains the role of `crates/synth-core`, `crates/synth-midi-osc`, `crates/synth-ui-theme`, `js/shared-synth-sdk`, and `js/konduktiva-peak`.
2. Complete 74-CC Parameter Table: Documents every physical continuous controller on the Peak, including default ranges, clamped safety ranges, and functional descriptions.
3. High-Resolution Bipolar Dual-CC Pairs (+32 Offset): Explains how 14-bit resolution is achieved via paired controllers (MSB 0..31, LSB 32..63).
4. 14-Bit NRPN Parameters: Details non-registered parameter addressing for filter types, slopes, and modulation matrix routing.
5. Focusrite SysEx Specification: Details the SysEx frame structure (`F0 00 20 29 ... F7`) and provides the exact 7-bit checksum calculation formula.
6. Acoustic Safety Limits: Explains the acoustic and technical reasons for capping resonance, post-filter drive, pre-filter overdrive, distortion, and delay feedback.
7. Konduktiva Live-Coding API: Details how to instantiate `createPeakKit`, create the eight canonical performers, apply triangular sweeps, and switch density graphs.
8. CLI Reference and Build Guide: Provides exact build commands using `CARGO_BUILD_JOBS=2` and details the connected hardware endpoints.

---

## 5. Verification Results and Quality Audit

All automated unit, integration, layout geometry, stress, and cross-language tests across the workspace pass cleanly with zero compiler warnings.

### 5.1 Test Suite Breakdown

| Test Suite / Binary | Test File or Component | Tests Passed | Status |
| :--- | :--- | :--- | :--- |
| `novation_peak_manager` (lib) | `src/lib.rs`, `stress_bench.rs`, `hardware_verify.rs`, etc. | 63 | PASS |
| `cross_synth_parity` | `tests/cross_synth_parity.rs` | 9 | PASS |
| `synth_core` | `crates/synth-core/src/lib.rs` | 43 | PASS |
| `synth_midi_osc` | `crates/synth-midi-osc/src/lib.rs` | 33 | PASS |
| `synth_ui_theme` | `crates/synth-ui-theme/src/lib.rs` | 29 | PASS |
| `konduktiva-peak` test suite | `js/konduktiva-peak/test/run.js` | 18 | PASS |
| Node.js profile & schema tests | `test/novation-peak-profile.test.js`, `baseline.test.js` | 8 | PASS |
| **Total Test Count** | **All 7 Test Suites** | **203** | **PASS** |

### 5.2 Compiler and Lint Discipline
- Zero compiler warnings (`rustc` and `cargo` report zero warnings across all crates).
- Zero linter errors in Node.js packages.
- Strict hardware safety preserved: Virtual Loopback mode default prevented any unprompted MIDI data transmission to `hw:3,0,0` during tests.
- Prose discipline: Zero em dashes in documentation and report files.
