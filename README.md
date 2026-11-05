# Novation Peak Manager & Konduktiva Companion (`novation-peak-pigs`)

A native Rust and egui desktop synthesizer manager, patch librarian, Korg nanoKONTROL2 surface controller, and Konduktiva live-coding companion for the **Novation Peak** 8-voice polyphonic hybrid synthesizer. This workspace is part of a 5-synth shared ecosystem spanning nkEditor, Hydrasynth, Bass Station II / TD-3, Nord Drum 2, and Novation Peak.

---

## 1. Five-Synthesizer Shared Architecture

The repository participates in an integrated 5-synth architecture sharing foundational Rust crates and JavaScript packages:

```
                                  +------------------------------------+
                                  |     5-Synth Shared Ecosystem       |
                                  +------------------------------------+
                                                    |
          +-----------------------+-----------------+-----------------------+
          |                       |                 |                       |
+-------------------+   +--------------------+  +--------------------+  +----------------------+
| crates/synth-core |   |crates/synth-midi-osc| |crates/synth-ui-theme| | js/shared-synth-sdk  |
+-------------------+   +--------------------+  +--------------------+  +----------------------+
| - SynthPatch      |   | - Non-invasive ALSA|  | - Two-Bowls / YAAE |  | - Cross-language     |
| - CommandHistory  |   |   cards/seq scan   |  |   theming (15 vars)|  |   schema sync        |
| - PresetLibrary   |   | - Safety clamping  |  | - Custom vector    |  | - Profile loader     |
| - PatchDiff/Merge |   | - PeakMidiRouter   |  |   widgets (SVF,    |  | - Theme constants    |
| - Randomizer      |   | - PeakSysExCodec   |  |   ADSR, Wavetable) |  | - Port definitions   |
| - 5 SynthProfiles |   | - Bidirectional OSC|  | - Soft takeover    |  |                      |
+-------------------+   +--------------------+  +--------------------+  +----------------------+
          |                       |                 |                       |
          +-----------------------+-----------------+-----------------------+
                                                    |
                         +-----------------------------------------+
                         | novation-peak-manager (src/)            |
                         | - PeakSynthEngine (8-voice polyphony)   |
                         | - PeakModFxEngine (16-slot mod matrix)  |
                         | - PeakLibrarian (512-patch bank CRUD)   |
                         | - KonduktivaBridge (8-player live code) |
                         | - NovationPeakApp (Two-Bowls desktop GUI)|
                         | - StressBench (1,000-event stress test) |
                         +-----------------------------------------+
                                                    |
                         +-----------------------------------------+
                         | js/konduktiva-peak (Node.js companion)  |
                         | - createPeakKit(K)                      |
                         | - 8 canonical players (bass/chords/...) |
                         | - safeSweepRange enforcement            |
                         | - runStressTest capture runner          |
                         +-----------------------------------------+
```

### The 5 Participating Synthesizers
1. **Novation Peak**: 8-voice polyphonic hybrid synthesizer featuring 3 FPGA New Oxford Oscillators, 17 wavetables, analogue state-variable filter, dual Animate macros, and analogue distortion.
2. **ASM Hydrasynth**: 8-voice polyphonic wavetable synthesizer with PolyTouch polyphonic aftertouch and Mutator matrix.
3. **Novation Bass Station II**: Monophonic dual-oscillator analogue monosynth with acid filter overdrive.
4. **Behringer TD-3-MO**: Monophonic analogue bassline synthesizer with modified resonance tracking and soft overdrive.
5. **Clavia Nord Drum 2**: 6-voice dedicated percussion synthesizer with noise resonance modeling and multi-channel percussion mapping.

---

## 2. Complete 74-Continuous-Controller (CC), NRPN & SysEx Map

Novation Peak communicates on **MIDI Channel 1** (wire index `0`). The system tracks all 74 physical continuous controllers, 32 bipolar dual-CC pairs, 14-bit NRPNs, and Focusrite/Novation System Exclusive frames.

### 2.1 The 74 Physical Continuous Controllers (CC Map)

| Parameter Key | CC Number | Default Range | Clamped Safe Range | Subsystem Description |
| :--- | :--- | :--- | :--- | :--- |
| `osc_1_shape` | 12 | 0..127 | 0..127 | Oscillator 1 Shape / Wavetable Position |
| `osc_1_coarse` | 14 | 0..127 | 0..127 | Oscillator 1 Coarse Tuning (-12..+12 st) |
| `osc_1_fine` | 15 | 0..127 | 0..127 | Oscillator 1 Fine Tuning (-50..+50 cents) |
| `osc_1_vsync` | 34 | 0..127 | 0..127 | Oscillator 1 Virtual Hard Sync |
| `osc_1_range` | 3 | 0..3 | 0..3 | Oscillator 1 Octave Footing (16', 8', 4', 2') |
| `osc_1_mod_env` | 33 | 0..127 | 0..127 | Oscillator 1 Pitch Mod Envelope 1 Depth |
| `osc_1_mod_lfo1`| 70 | 0..127 | 0..127 | Oscillator 1 Pitch LFO 1 Depth |
| `osc_2_shape` | 39 | 0..127 | 0..127 | Oscillator 2 Shape / Wavetable Position |
| `osc_2_coarse` | 17 | 0..127 | 0..127 | Oscillator 2 Coarse Tuning (-12..+12 st) |
| `osc_2_fine` | 18 | 0..127 | 0..127 | Oscillator 2 Fine Tuning (-50..+50 cents) |
| `osc_2_vsync` | 42 | 0..127 | 0..127 | Oscillator 2 Virtual Hard Sync |
| `osc_2_range` | 37 | 0..3 | 0..3 | Oscillator 2 Octave Footing (16', 8', 4', 2') |
| `osc_2_mod_env` | 40 | 0..127 | 0..127 | Oscillator 2 Pitch Mod Envelope 1 Depth |
| `osc_2_mod_lfo1`| 72 | 0..127 | 0..127 | Oscillator 2 Pitch LFO 1 Depth |
| `osc_3_shape` | 71 | 0..127 | 0..127 | Oscillator 3 Shape / Wavetable Position |
| `osc_3_coarse` | 20 | 0..127 | 0..127 | Oscillator 3 Coarse Tuning (-12..+12 st) |
| `osc_3_fine` | 21 | 0..127 | 0..127 | Oscillator 3 Fine Tuning (-50..+50 cents) |
| `osc_3_vsync` | 44 | 0..127 | 0..127 | Oscillator 3 Virtual Hard Sync |
| `osc_3_range` | 65 | 0..3 | 0..3 | Oscillator 3 Octave Footing (16', 8', 4', 2') |
| `osc_3_mod_env` | 41 | 0..127 | 0..127 | Oscillator 3 Pitch Mod Envelope 1 Depth |
| `osc_3_mod_lfo1`| 73 | 0..127 | 0..127 | Oscillator 3 Pitch LFO 1 Depth |
| `mixer_osc_1` | 23 | 0..127 | 0..127 | Mixer Oscillator 1 Level |
| `mixer_osc_2` | 24 | 0..127 | 0..127 | Mixer Oscillator 2 Level |
| `mixer_osc_3` | 25 | 0..127 | 0..127 | Mixer Oscillator 3 Level |
| `mixer_ring_mod`| 26 | 0..127 | 0..127 | Mixer Ring Modulator (Osc 1 x Osc 2) Level |
| `mixer_noise` | 27 | 0..127 | 0..127 | Mixer White Noise Level |
| `filter_cutoff` | 29 | 0..127 | 0..127 | State-Variable Filter Cutoff Frequency |
| `filter_resonance`| 79 | 0..127 | **0..62** | Filter Resonance (Clamped against self-oscillation) |
| `filter_post_drive`| 36 | 0..127 | **0..50** | Post-Filter Analogue Drive (Acoustic protection) |
| `filter_overdrive`| 80 | 0..127 | **0..50** | Pre-Filter Analogue Overdrive (Input saturation) |
| `filter_key_track`| 75 | 0..127 | 0..127 | Filter Cutoff Keyboard Tracking |
| `filter_fm_osc3`| 76 | 0..127 | 0..127 | Oscillator 3 Direct Filter Frequency Modulation |
| `filter_mod_env`| 78 | 0..127 | 0..127 | Mod Envelope 1 to Filter Cutoff Depth |
| `filter_lfo2` | 77 | 0..127 | 0..127 | LFO 2 to Filter Cutoff Depth |
| `amp_env_attack`| 86 | 0..127 | 0..127 | Amplifier Envelope Attack Time |
| `amp_env_decay` | 87 | 0..127 | 0..127 | Amplifier Envelope Decay Time |
| `amp_env_sustain`| 88 | 0..127 | 0..127 | Amplifier Envelope Sustain Level |
| `amp_env_release`| 89 | 0..127 | 0..127 | Amplifier Envelope Release Time |
| `mod_env1_attack`| 90 | 0..127 | 0..127 | Mod Envelope 1 Attack Time |
| `mod_env1_decay` | 91 | 0..127 | 0..127 | Mod Envelope 1 Decay Time |
| `mod_env1_sustain`| 92 | 0..127 | 0..127 | Mod Envelope 1 Sustain Level |
| `mod_env1_release`| 93 | 0..127 | 0..127 | Mod Envelope 1 Release Time |
| `mod_env2_attack`| 94 | 0..127 | 0..127 | Mod Envelope 2 Attack Time |
| `mod_env2_decay` | 95 | 0..127 | 0..127 | Mod Envelope 2 Decay Time |
| `mod_env2_sustain`| 117 | 0..127 | 0..127 | Mod Envelope 2 Sustain Level |
| `mod_env2_release`| 103 | 0..127 | 0..127 | Mod Envelope 2 Release Time |
| `lfo1_rate` | 30 | 0..127 | 0..127 | LFO 1 Free-Run Rate |
| `lfo1_sync_rate`| 81 | 0..31 | 0..31 | LFO 1 Tempo-Sync Division Rate |
| `lfo1_fade` | 82 | 0..127 | 0..127 | LFO 1 Fade-In Time |
| `lfo1_range` | 83 | 0..1 | 0..1 | LFO 1 Rate Multiplier Range (Low / High) |
| `lfo2_rate` | 31 | 0..127 | 0..127 | LFO 2 Free-Run Rate |
| `lfo2_sync_rate`| 84 | 0..31 | 0..31 | LFO 2 Tempo-Sync Division Rate |
| `lfo2_fade` | 85 | 0..127 | 0..127 | LFO 2 Fade-In Time |
| `distortion_level`| 104 | 0..127 | **0..48** | Master Analogue Distortion Level |
| `chorus_level` | 105 | 0..127 | **0..90** | Chorus Wet Send Level |
| `chorus_rate` | 106 | 0..127 | 0..127 | Chorus Modulation Rate |
| `chorus_depth` | 107 | 0..127 | 0..127 | Chorus Modulation Depth |
| `chorus_type` | 118 | 0..3 | 0..3 | Chorus Algorithm Type (Ensemble, Chorus, Flanger) |
| `delay_level` | 108 | 0..127 | **0..90** | Delay Wet Send Level |
| `delay_time` | 109 | 0..127 | 0..127 | Delay Time / Sync Subdivision |
| `delay_feedback`| 110 | 0..127 | **0..70** | Delay Feedback Loop (Clamped against runaway scream) |
| `reverb_level` | 112 | 0..127 | **0..90** | Reverb Wet Send Level |
| `reverb_decay` | 113 | 0..127 | 0..127 | Reverb Decay Time |
| `reverb_type` | 119 | 0..2 | 0..2 | Reverb Algorithm Type (Large, Medium, Small) |
| `animate_1` | 114 | 0, 127 | 0, 127 | Animate 1 Performance Button Trigger |
| `animate_2` | 115 | 0, 127 | 0, 127 | Animate 2 Performance Button Trigger |
| `arp_gate` | 116 | 0..127 | 0..127 | Arpeggiator Gate Duration |
| `glide_time` | 5 | 0..127 | 0..127 | Portamento Glide Rate |
| `glide_mode` | 35 | 0..1 | 0..1 | Glide Mode (Legato / Constant) |
| `bank_select_msb`| 0 | 0 | 0 | MIDI Bank Select MSB (always 0) |
| `bank_select_lsb`| 32 | 0..3 | 0..3 | MIDI Bank Select LSB (Bank A=0, B=1, C=2, D=3) |
| `mod_wheel` | 1 | 0..127 | 0..127 | Modulation Wheel Input |
| `volume` | 7 | 0..127 | 0..127 | Master Output Channel Volume |
| `pan` | 10 | 0..127 | 0..127 | Stereo Panning Position |

### 2.2 High-Resolution Bipolar Dual-CC Pairs (+32 Offset)
Novation Peak encodes high-resolution 14-bit dial positions using paired Continuous Controllers offset by +32:
- **MSB (Most Significant Byte)**: CCs `0..31`
- **LSB (Least Significant Byte)**: CCs `32..63`
- Coarse parameter increments occur via MSB, and fine fractional increments occur via LSB.

### 2.3 14-Bit NRPN Parameters
For system settings and matrix configurations not covered by standard 7-bit CCs, Novation Peak uses 14-bit Non-Registered Parameter Numbers (NRPNs):
- NRPN MSB / LSB address parameters like Filter Slope (12dB vs 24dB), Filter Shape (LP, BP, HP), Oscillator Wavetable Index (0..16), and the 16 dual-source Modulation Matrix rows.

### 2.4 Focusrite / Novation SysEx Protocol
Novation Peak transmits and receives complete patch dumps via System Exclusive messages:
- **Manufacturer Header**: `F0 00 20 29` (Focusrite / Novation ID)
- **Product ID**: `01`
- **Device Family**: `00 00`
- **Command**: `00` (Dump Single Patch) or `02` (Request Patch)
- **Location**: Bank `0..3`, Slot `0..127`
- **Checksum Calculation**:
  ```text
  Checksum = (128 - (sum_of_payload_bytes & 0x7F)) & 0x7F
  ```
- **Terminator**: `F7`

---

## 3. Acoustic Protection & Safe Clamping Limits (`safeSweepRange`)

To protect studio monitoring hardware, headphones, and listener hearing during rapid automation sweeps, `synth_midi_osc::routing` and `js/konduktiva-peak` enforce conservative upper bounds on parameters that can cause self-oscillation or runaway acoustic feedback:

1. **Filter Resonance (CC 79)**: Capped at **62** (48.8%). Prevents self-oscillating sine shrieks at high filter cutoffs.
2. **Post-Filter Drive (CC 36)**: Capped at **50** (39.4%). Prevents downstream analogue amplifier clipping.
3. **Pre-Filter Overdrive (CC 80)**: Capped at **50** (39.4%). Prevents harsh wave-shaping distortion into the filter.
4. **Analogue Distortion (CC 104)**: Capped at **48** (37.8%). Prevents aggressive harmonic square clipping.
5. **Delay Feedback (CC 110)**: Capped at **70** (55.1%). Prevents runaway echo feedback accumulation.
6. **Chorus, Delay, and Reverb Levels (CCs 105, 108, 112)**: Capped at **90** (70.9%). Prevents wet signal wash.

---

## 4. Konduktiva-Peak Live-Coding Companion API

The JavaScript live-coding package (`js/konduktiva-peak`) provides real-time algorithmic music drivers designed specifically for Novation Peak's 8-voice architecture.

### 4.1 Creating the Peak Live-Coding Kit
```javascript
const konduktiva = require('konduktiva');
const { createPeakKit } = require('konduktiva-peak');

const kit = createPeakKit(konduktiva);
const env = konduktiva.setUpMusicalEnvironmentExamples();

// Build all 8 canonical players on MIDI Channel 1
const players = kit.createPeakPlayers(env, { channel: 1 });
```

### 4.2 The Eight Canonical Players
The live-coding bridge divides musical tasks into 4 note voices and 4 continuous sweep lanes:
1. `bass`: Root and fifth bass foundation (Octave 2, rate 0.5 onsets per beat).
2. `chords`: Rich 3-note and 4-note minor triads and minor-seventh chord voicings (Octave 4).
3. `lead`: Flowing eighth-note melodic contours from the natural-minor scale pool (Octave 5).
4. `arp`: Dynamic sixteenth-note arpeggiator patterns darting across intervals with velocity accents.
5. `ccCutoff`: Continuous triangular sweep across Filter Cutoff (CC 29).
6. `ccReso`: Continuous triangular sweep across Filter Resonance (CC 79, clamped to 62).
7. `ccMod`: Continuous triangular sweep across Modulation Wheel (CC 1).
8. `ccFx`: Coordinated dual triangular sweep modulating Delay Send (CC 108) and Reverb Send (CC 112).

### 4.3 Sweeps and Musical Density
```javascript
// Single-CC sweep helper
const cutoffSweep = kit.sweepCC(29, 20, 110, 8.0, 32);

// Dual-CC coordinated sweep helper
const fxSweep = kit.dualSweepCC(108, 10, 70, 112, 20, 80, 16.0, 32);

// Density switching between steady groove and rising crescendo
env.setDensityGraph('peakHold');  // Constant density
env.setDensityGraph('peakBuild'); // Smoothly doubles musical density over 16 beats
```

---

## 5. Command-Line Interface (CLI) Reference

The `novation-peak-manager` binary provides non-blocking flags for automated hardware verification, stress benchmarks, schema exports, preset queries, and headless server operation:

```bash
# Non-invasive hardware discovery (verifies Peak, UMC1820, and nanoKONTROL2 safely)
cargo run -- --verify-hardware

# 1,000-event 8-voice polyphonic + 9-lane CC sweep stress benchmark
cargo run -- --dry-stress

# Export all JSON schemas and profiles across the 5-synth ecosystem to a directory
cargo run -- --export-shared-schemas ./schemas

# Query preset library metadata by name or slot index
cargo run -- --preset "Oxford FPGA Pad"
cargo run -- --preset 0

# Start non-blocking headless OSC server on port 9100 for MIDI Channel 1
cargo run -- --headless-server --port 9100 --channel 1

# Run headless egui layout geometry auditor (verifies 1280x800 and 1920x1080)
cargo run -- --audit-geometry

# Launch full native Two-Bowls egui desktop application
cargo run -- --gui
```

### Complete CLI Flags Summary
- `--verify-hardware`: Non-invasive ALSA scan inspecting `/proc/asound/cards` and `/proc/asound/seq/clients` with safe virtual loopback.
- `--dry-stress`: Runs the 1,000-event polyphonic note + 9-lane CC sweep stress benchmark verifying zero dropped notes, 8-voice budget, zero stuck voices on panic, and strict safeSweepRange enforcement.
- `--export-shared-schemas [DIR]`: Exports all schema definitions and device profiles to the specified directory (defaults to `./schemas`).
- `--synth <NAME>`: Target synthesizer model name (default: `novation-peak`).
- `--midi-in <DEVICE>`: Target MIDI In port or pattern.
- `--midi-out <DEVICE>`: Target MIDI Out port or pattern.
- `--channel <1-16>`: Target MIDI channel (default: `1`).
- `--preset <NAME_OR_INDEX>`: Query and display preset metadata from the 512-patch library.
- `--headless-server`: Start non-blocking headless OSC server.
- `--port <PORT>`: OSC UDP listen port (default: `9100`).
- `--audit-geometry`: Run headless egui window geometry audit.
- `--gui`: Launch the native egui desktop graphical user interface.
- `--dry-run`: Force read-only inspection mode.
- `-h`, `--help`: Display CLI usage instructions.

---

## 6. Build, Verification & Hardware Setup

### CPU & Build Parallelism Constraint
Build commands must respect `CARGO_BUILD_JOBS=2` to ensure stability:

```bash
# Run all workspace unit and integration tests (Rust)
CARGO_BUILD_JOBS=2 cargo test --workspace

# Run all Node.js live-coding tests and schema validations
npm test
```

### Connected Physical Hardware
When running on the development workstation:
- **Novation Peak**: Connected via USB as `Focusrite A.E Peak` (`hw:3,0,0`, ALSA client 28, USB ID `1235:007e`).
- **Behringer UMC1820**: Connected as `UMC1820 MIDI 1` (`hw:0,0,0`, ALSA client 16, USB ID `1397:0503`).
- **Korg nanoKONTROL2**: Connected as `nanoKONTROL2 _ CTRL` (`hw:2,0,0`, ALSA client 24, USB ID `0944:0117`).

During all automated testing, the software operates in **Virtual Loopback / Capture Stub Mode** (`live: false`), preventing unprompted MIDI events from being transmitted to physical synthesizer hardware.
