---
title: "Report 001: Epoch 1, 5-Synth Ecosystem Profile (NovationPeakProfile) and Shared Rust/JS SDK Registration"
date: 2026-11-06 02:25:00
project: novation-peak-pigs
project_dir: /home/renick/antigravity/novation-peak-pigs
report_number: 1
subject: "Novation Peak 5-Synth Ecosystem Profile and Shared Rust/JS SDK Registration"
operator: "Antigravity (Gemini 3.8 Flash)"
follows: []
tldr: "Epoch 1 integrates the Novation Peak 8-voice polyphonic hybrid synthesizer into the shared 5-synthesizer ecosystem alongside ASM Hydrasynth, Korg nanoKONTROL2 (nkEditor), Novation Bass Station II with Behringer TD-3, and Clavia Nord Drum 2. We authored the comprehensive NovationPeakProfile covering all 74 continuous controllers from midi-cc-map.json, 32 bipolar dual-CC pairs, 14-bit NRPN descriptors, 3 Oxford FPGA NCO oscillators with 17 wavetables, state-variable filter with pre/post drive, 3 ADSR envelopes, 2 LFOs, 16-slot modulation matrix, Animate 1 and 2 controls, and analogue distortion plus FX. All 83 Rust tests and 8 Node.js tests pass with zero warnings."
tests_total: 91
qa_verdict: PASS
---

# Prompt

```
[FORMAL INTENT]: Epoch 1 of 8: 5-Synth Ecosystem Profile (NovationPeakProfile) & Shared Rust/JS SDK Registration (Target: crates/synth-core; Related: js/shared-synth-sdk, js/konduktiva-peak). Extend crates/synth-core (src/profiles.rs and src/lib.rs) and js/shared-synth-sdk with NovationPeakProfile covering all 74 CCs from js/konduktiva-peak/midi-cc-map.json, +32 bipolar dual-CC pairs, 14-bit NRPN descriptors, 3 FPGA NCO oscillators (17 wavetables), mixer/ring-mod/noise, state-variable LP/BP/HP filter with pre/post drive, 3 ADSR envelopes, 2 tempo-synced LFOs, 16-slot mod matrix, Animate 1 & 2 holds, arpeggiator gate, and analogue distortion + chorus/delay/reverb FX. Register Novation Peak as the 5th synth in UnifiedEcosystemSchemaBundle alongside nkEditor, Hydrasynth, BS2/TD-3, and Nord Drum 2, and add comprehensive unit tests. Ensure all workspace tests pass (CARGO_BUILD_JOBS=2 cargo test --workspace and npm test). Write reports/report-001.md and compile it with /home/renick/claude/html-report-generator/build-report.sh --pdf.
```

# Content

## 1. Overview and Everyday Purpose

A synthesizer is an electronic instrument that generates sound. In this project, we are developing software that configures, manages, and creates sound presets for physical synthesizers connected to a Linux workstation.

Before this development phase, our software workspace supported four hardware synthesizer platforms:

1. ASM Hydrasynth: a modern digital wavetable polyphonic synthesizer.
2. nkEditor: a tactile MIDI controller editor for the Korg nanoKONTROL2 and Akai MIDImix.
3. Novation Bass Station II and Behringer TD-3: analog monophonic bass synthesizers.
4. Clavia Nord Drum 2: a 6-voice physical modeling drum synthesizer.

In this first epoch, we registered the Novation Peak as the fifth instrument in our unified ecosystem.

The Novation Peak is an 8-voice polyphonic synthesizer, which means it can play up to 8 notes at the same time. It is a hybrid instrument because its sound oscillators are generated digitally inside an FPGA (Field-Programmable Gate Array, a high-speed programmable computer chip that computes audio waveforms at 24 million samples per second), while its sound filters, overdrive circuits, and master distortion stages are built with real analog electronics.

By establishing a formal device profile in both Rust (`crates/synth-core`) and JavaScript (`js/shared-synth-sdk`), our desktop application, preset librarian, and live-coding sequencer now share an identical map of every knob, switch, and sound setting on the instrument.

## 2. Parameter Mapping: 74 CCs, 32 Dual-CC Pairs, and 14-bit NRPNs

MIDI (Musical Instrument Digital Interface) is the worldwide standard protocol that musical instruments use to talk to computers. Standard MIDI commands use Continuous Controller (CC) messages with numbers from 0 to 127 to change settings such as volume, pitch, or brightness.

Standard CC values only have 128 discrete steps. When adjusting delicate settings like filter frequency, 128 steps can cause noticeable stair-stepping or zipper noise. To provide smooth continuous sweeps, the Novation Peak supports two higher-resolution communication schemes:

1. Dual-CC Pairs: The instrument pairs a primary controller (MSB, Most Significant Byte, CCs 0 through 31) with a secondary controller (LSB, Least Significant Byte, CCs 32 through 63, which is the base CC plus 32). Combining MSB and LSB yields 16,384 distinct steps for high-precision control.
2. NRPNs (Non-Registered Parameter Numbers): A 14-bit MIDI addressing standard used to reach internal sound engine parameters that do not have dedicated standard CC numbers, such as selecting from 17 digital wavetables or routing signals through a 16-slot modulation matrix.

The table below connects plain-English synthesizer concepts to their exact code symbols, parameter addresses, and value ranges in `NovationPeakProfile`:

| Plain-English Musical Concept | Technical Symbol | Parameter Address | Working Value Range |
| :--- | :--- | :--- | :--- |
| Filter Frequency / Brightness | `filter_cutoff` | CC 29 | 0 to 127 (Exponential curve) |
| High-Resolution 14-bit Filter Cutoff | `dual_cc_29_61` | CC 29 (MSB) + CC 61 (LSB) | 0 to 16,383 (14-bit dual pair) |
| Filter Resonance / Whistle | `filter_resonance` | CC 79 | 0 to 127 (Safe sweep: 0 to 62) |
| Pre-Filter Analogue Overdrive | `filter_overdrive` | CC 80 | 0 to 127 (Safe sweep: 0 to 50) |
| Post-Filter Drive | `filter_post_drive` | CC 36 | 0 to 127 (Safe sweep: 0 to 50) |
| Master Analogue Distortion | `distortion_level` | CC 104 | 0 to 127 (Safe sweep: 0 to 48) |
| Mod Wheel Vibrato / Expression | `mod_wheel` | CC 1 | 0 to 127 |
| Portamento Glide Time | `glide_time` | CC 5 | 0 to 127 (Exponential curve) |
| Glide On / Off Switch | `glide_on` | CC 35 | 0 to 1 (Switch) |
| Oscillator 1 Level in Mixer | `osc_1_mix` | CC 23 | 0 to 127 (Default: 127) |
| Oscillator 2 Level in Mixer | `osc_2_mix` | CC 24 | 0 to 127 (Default: 0) |
| Oscillator 3 Level in Mixer | `osc_3_mix` | CC 25 | 0 to 127 (Default: 0) |
| Ring Modulation Level | `ring_mod_mix` | CC 26 | 0 to 127 (Osc 1 x Osc 2) |
| Noise Generator Level | `noise_mix` | CC 27 | 0 to 127 |
| Digital Delay Send Level | `delay_level` | CC 108 | 0 to 127 (Safe sweep: 0 to 90) |
| Digital Delay Time | `delay_time` | CC 109 | 0 to 127 |
| Digital Delay Feedback / Echoes | `delay_feedback` | CC 110 | 0 to 127 (Safe sweep: 0 to 70) |
| Digital Reverb Send Level | `reverb_level` | CC 112 | 0 to 127 (Safe sweep: 0 to 90) |
| Digital Reverb Decay Time | `reverb_time` | CC 113 | 0 to 127 |
| Stereo Chorus Level | `chorus_level` | CC 105 | 0 to 127 (Safe sweep: 0 to 90) |
| Stereo Chorus Rate | `chorus_rate` | CC 118 | 0 to 127 |
| Animate 1 Performance Button | `animate_1_hold` | CC 114 | 0 to 127 (Momentary or toggle) |
| Animate 2 Performance Button | `animate_2_hold` | CC 115 | 0 to 127 (Momentary or toggle) |
| Arpeggiator Clock Gate Length | `arp_clock_gate` | CC 116 | 0 to 127 (Note duration percentage) |
| Osc 1 Waveform / Wavetable | `osc_1_waveform` | NRPN (0, 64) | 0 to 20 (4 analog + 17 wavetables) |
| Filter Type (LP, BP, HP) | `filter_type` | NRPN (0, 48) | 0 to 2 (LowPass, BandPass, HighPass) |
| Filter Slope (12dB / 24dB) | `filter_slope` | NRPN (0, 49) | 0 to 1 (2-pole vs 4-pole) |
| 16 Mod Matrix Slots (64 params) | `mod_matrix_slot_{1..16}_*` | NRPN (1, slot*4 + 0..3) | Source 1, Source 2, Depth, Dest |

## 3. Sound Engine Architecture: Oscillators, Filter, Envelopes, and Mod Matrix

The Novation Peak sound engine is organized into distinct functional blocks:

### Phase: 3 Oxford FPGA NCO Oscillators and 17 Wavetables

Each of the 3 oscillators can produce 4 classic analog waveforms (Sine, Triangle, Sawtooth, Pulse) or select from 17 digital wavetables. A wavetable is a sequence of single-cycle sound shapes stored in memory. As the `osc_shape` knob turns, the oscillator sweeps across the table to create morphing timbres. The 17 wavetable names available in `NovationPeakProfile::WAVETABLES` are:

- `Sine-Tri`, `Sine-Saw`, `Classic`, `Saw-Pulse`
- `Harmonic 1`, `Harmonic 2`, `Chebyshev`, `Double`
- `Metal`, `Resonant 1`, `Resonant 2`, `Formant 1`
- `Formant 2`, `Vocal`, `Bell`, `Organ`, `Digi`

Each oscillator also features independent range selection (16, 8, 4, or 2 feet), coarse tuning (-12 to +12 semitones), fine tuning (-100 to +100 cents), virtual oscillator sync (`vsync`), and dedicated modulation inputs from envelopes and LFOs.

### Phase: State-Variable Analogue Filter with Pre and Post Drive

The filter removes or emphasizes specific audio frequencies:

- Filter Type: switchable between Low-Pass (lets deep bass pass while cutting highs), Band-Pass (isolates a middle frequency band), and High-Pass (cuts bass while letting treble through).
- Filter Slope: switchable between 12dB per octave (a gentle slope that keeps more harmonic overtones) and 24dB per octave (a steep slope for focused filtering).
- Pre-Filter Overdrive: analog drive applied before the filter for saturation.
- Post-Filter Drive: analog drive applied after the filter to compress resonant peaks.

### Phase: 3 ADSR Envelopes and 2 Tempo-Synced LFOs

An envelope shapes how a sound evolves over time through four stages: Attack (how quickly sound starts), Decay (time to fall to sustain level), Sustain (held volume), and Release (fade-out time after releasing a key). Novation Peak includes three separate ADSR envelopes:

- Amp Envelope: shapes output volume.
- Mod Envelope 1: dedicated to filter cutoff and oscillator pitch sweeps.
- Mod Envelope 2: dedicated to secondary pitch and wave shape modulations.

Two Low-Frequency Oscillators (LFO 1 and LFO 2) generate sub-audible cyclical waves (Sine, Triangle, Sawtooth, Square, Sample and Hold) to add vibrato, filter sweeps, or tremolo, either free-running or synchronized to musical beat divisions.

### Phase: 16-Slot Modulation Matrix

The modulation matrix acts as an electronic patchbay. It has 16 independent routing channels. In each channel, a performer selects a primary source (like Mod Wheel or Aftertouch), an optional gate or multiplier source (like an LFO), a destination (like Filter Cutoff or Chorus Depth), and a bipolar depth setting (-64 to +63). This enables complex animated textures without changing physical wiring.

## 4. Hardware Safety Clamping with `safe_sweep_range`

Analog filters and distortion circuits can self-oscillate and become loud enough to damage studio monitors or cause ear fatigue if an automated sweep drives resonance to maximum (127). To ensure hardware safety during automated testing and live performance, we implemented safety clamping rules in both Rust (`NovationPeakProfile::safe_sweep_range`) and JavaScript (`safeSweepRange` in `js/konduktiva-peak` and `js/shared-synth-sdk`).

| Target Parameter Keyword | Raw Unchecked Range | Clamped Safe Range | Protection Objective |
| :--- | :--- | :--- | :--- |
| `resonance` or `reso` | 0 to 127 | 0 to 62 | Prevents runaway self-oscillation howl |
| `distortion` | 0 to 127 | 0 to 48 | Prevents harsh analog/digital clipping |
| `overdrive` or `drive` | 0 to 127 | 0 to 50 | Maintains comfortable acoustic headroom |
| `feedback` (delay) | 0 to 127 | 0 to 70 | Prevents infinite delay loop accumulation |
| `reverb`, `delay`, `chorus`, `fx` | 0 to 127 | 0 to 90 | Keeps effect sends below muddy wash levels |
| Standard parameters (cutoff, shape) | 0 to 127 | 0 to 127 | Full expressive range preserved |

## 5. Unified Ecosystem Registration and Schema Exporter

We introduced `UnifiedEcosystemSchemaBundle` in both Rust and Node.js. This bundle groups all five instrument families into a single validated document:

1. ASM Hydrasynth: 65 parameters, category `Synthesizer`.
2. Korg nanoKONTROL2 / Akai MIDImix: 51 controls each, category `ControllerSurface`.
3. Novation Bass Station II: 45 parameters, category `Synthesizer`.
4. Behringer TD-3: 19 parameters, category `Synthesizer`.
5. Clavia Nord Drum 2: 202 parameters (6 voices x 31 params + 16 global), category `Synthesizer`.
6. Novation Peak: 190 parameters (74 CCs + 32 dual-CC + 84 NRPNs) and 22 panel controls, category `Synthesizer`.

The `export_all_schemas` function was updated to output 12 canonical JSON files, adding `novation-peak.profile.json`, `novation-peak.theme.json`, and `unified-ecosystem-bundle.json`.

# Files Touched

- `crates/synth-core/src/profiles.rs`: added `NovationPeakProfile`, `DeviceSynthesizerProfile::novation_peak()`, `UnifiedEcosystemSchemaBundle`, updated schema export, and comprehensive unit tests.
- `crates/synth-core/src/lib.rs`: re-exported `NovationPeakProfile` and `UnifiedEcosystemSchemaBundle`, updated 5-synth profile verification, and added tests for patch CRUD, preset library, patch diffing, lock-shielded randomizer, and command history.
- `src/lib.rs`: re-exported `NovationPeakProfile` and `UnifiedEcosystemSchemaBundle` from the workspace root crate with baseline integration tests.
- `js/shared-synth-sdk/schema-validator.js`: added `NOVATION_PEAK_PROFILE`, `NOVATION_PEAK_THEME`, `UNIFIED_ECOSYSTEM_SCHEMA_BUNDLE`, `UnifiedEcosystemSchemaBundle`, `NovationPeakProfile`, and updated `exportAllSchemas`.
- `js/shared-synth-sdk/index.js`: re-exported Peak profile, theme, bundle, and added the `NovationCyan` accent token.
- `test/novation-peak-profile.test.js`: added 8 comprehensive Node.js tests verifying CC coverage, dual-CC pairs, NRPN parameters, wavetables, mod matrix, schema validation, and safe sweeps.

# Files Not Touched

- `crates/synth-midi-osc/src/hardware.rs`: hardware ALSA port discovery and MIDI codecs preserved for Epoch 2 router implementation.
- `crates/synth-midi-osc/src/routing.rs`: existing MIDI routing and anti-choke scheduler kept intact.
- `crates/synth-ui-theme/src/lib.rs`: UI themes preserved for Epoch 6 desktop GUI development.
- `js/konduktiva-peak/src/index.js`: Konduktiva live-coding kit entry point preserved.

# Tests

- `CARGO_BUILD_JOBS=2 cargo test --workspace`: 83 tests passing, 0 failures, 0 compiler warnings.
- `npm test`: 8 tests passing, 0 failures.
- Unit tests verify all 74 CCs, 32 dual-CC pairs, 14-bit NRPNs, 17 wavetables, 16 mod matrix slots, preset library CRUD, patch diffing, and lock-shielded randomization.

# QA

- Verdict: PASS
- Memory footprint: negligible (in-memory device profiles occupy under 500 KB)
- Build parallelism: strictly capped at CARGO_BUILD_JOBS=2
- Zero compiler warnings across all Rust crates and Node.js test files
- Hardware safety: automated tests ran in virtual stub / dry-run mode without sending unprompted MIDI to physical Peak hardware at hw:3,0,0

# Lesson

Establishing a single, shared source of truth for parameter definitions across Rust and JavaScript through strongly-typed profiles and Draft-07 JSON schemas guarantees that native desktop GUI tools and live-coding sequencers share identical parameter addresses, curve scalings, and safety clamp boundaries without duplicating domain logic.
