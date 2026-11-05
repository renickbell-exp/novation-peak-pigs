---
title: "Report 005: Epoch 5, Patch Librarian CRUD, 512-Patch Bank/SysEx Manager, A/B Patch Diff & Selective Merge, Continuous Morph Fader, Safe Randomizer & Korg nanoKONTROL2 Surface Controller"
date: 2026-11-06 03:05:00
project: novation-peak-pigs
project_dir: /home/renick/antigravity/novation-peak-pigs
report_number: 5
subject: "Patch Librarian CRUD, 512-Patch Bank/SysEx Manager, A/B Patch Diff & Selective Merge, Continuous Morph Fader, Safe Randomizer & Korg nanoKONTROL2 Surface Controller"
operator: "Antigravity (Gemini 3.8 Flash)"
follows: ["report-004.md"]
tldr: "Epoch 5 delivers the complete patch management, bank organization, diff and merge, continuous morphing, lock-shielded randomization, and hardware control surface integration for the Novation Peak 8-voice hybrid synthesizer in src/peak_librarian.rs, exported directly via src/lib.rs. The subsystem provides 8 curated factory patches showcasing Peak synthesis capabilities; a full 512-patch bank manager (Banks A through D, 128 patches each) addressable via CC 0, CC 32, and Program Change; a streaming System Exclusive (SysEx) state machine supporting single-patch and bank dumps with 7-bit two's complement checksum verification and universal 0x7F bypass; continuous A/B patch morphing (0.0 to 1.0) with real-time MIDI delta reporting; A/B patch diffing and selective merging; a lock-shielded safe randomizer with global, section-scoped (Osc, Filter, Env, LFO, FX), and Gaussian drift modes enforcing automatic safety caps on CCs 36, 79, 80, 104, and 110; and a full Korg nanoKONTROL2 (hw:2,0,0) 8-column physical surface mapper featuring Jump, Catch, and Pickup soft takeover modes, bidirectional LED feedback, and dedicated Animate 1 and 2 triggers. All 142 tests (134 Rust workspace unit tests and 8 Node.js tests) pass with zero compiler warnings."
tests_total: 142
qa_verdict: PASS
---

# Prompt

```
[FORMAL INTENT]: Epoch 5 of 8: Patch Librarian CRUD, 512-Patch Bank/SysEx Manager, A/B Patch Diff & Selective Merge, Continuous Morph Fader, Safe Randomizer & Korg nanoKONTROL2 Surface Controller (Target: src/peak_librarian.rs; Related: src/lib.rs, crates/synth-core). Implement src/peak_librarian.rs reusing synth_core::PresetLibrary, PatchDiff, Randomizer, and SoftTakeoverState directly: 8 curated Novation Peak factory patches (Init Patch, Oxford FPGA Pad, Acid Overdrive Bass, VSync Lead, Ring Mod Bell, Lush Chorus Strings, Dub Delay Chord, Animate Morph Arp), 4 banks x 128 patches (Banks A..D via CC0/CC32 + Program Change), SysEx single-patch and bank dump state machine with checksum verification, continuous A..B patch morphing (0.0..=1.0), lock-shielded randomizer (Global, Section-Scoped for Osc/Filter/Env/LFO/FX, and Gaussian Drift with automatic safety caps on CCs 36/79/80/104/110), and Korg nanoKONTROL2 (hw:2,0,0) 8-column fader/knob/button surface mapper with Jump/Catch/Pickup soft takeover and Animate 1/2 triggers. Ensure all workspace tests pass (CARGO_BUILD_JOBS=2 cargo test --workspace and npm test). Write reports/report-005.md and compile it with /home/renick/claude/html-report-generator/build-report.sh --pdf.
```

# Content

## 1. Overview and Everyday Purpose

In electronic music, a synthesizer can produce an infinite variety of timbres, ranging from soft orchestral flutes to aggressive distorted basslines. Managing, editing, and performing with hundreds of these sounds requires a reliable digital system. To understand the six main capabilities built in Epoch 5, consider these everyday parallels:

1. Digital Filing Cabinet and Sound Shelf (512-Patch Bank Librarian): Think of a large physical music library with four tall bookcases labeled Bank A, Bank B, Bank C, and Bank D. Each bookcase has exactly 128 shelf slots (numbered 0 to 127), giving a total capacity of 512 sound patches. When a performer requests Bank B, Slot 45, the system sends standard MIDI instructions (Bank Select MSB CC 0, Bank Select LSB CC 32, and Program Change 45) to instantly load that exact sound recipe into the instrument.
2. Verified Digital Backups (System Exclusive Dumps with Checksum): When backing up custom sounds from the synthesizer hardware to a computer, data travels across a MIDI cable. Just like a grocery receipt lists all items and prints a total at the bottom to verify payment accuracy, our System Exclusive (SysEx) state machine computes a mathematical checksum (a 7-bit two's complement total) for every patch. If a single byte is dropped or corrupted in transit, the receipt total does not match, and the system catches the error immediately before invalid data can overwrite memory.
3. Recipe Diff and Selective Ingredient Merge (A/B Patch Diff and Merge): Imagine two different chocolate cake recipes. One has extra vanilla; the other has chocolate chips. Instead of choosing only one or the other, an A/B diff tool highlights the exact differences. A selective merge allows the baker to copy only the chocolate chips from Recipe B into Recipe A without altering the original baking temperature or flour ratio.
4. Seamless Flavor Transition (Continuous Patch Morph Fader): Imagine sliding a mixer fader that smoothly blends a rich tomato soup into a creamy clam chowder. At 0%, you hear 100% Sound A. At 50%, you hear a unique hybrid with half of Sound A and half of Sound B. At 100%, you arrive fully at Sound B. Our continuous morph engine interpolates all parameters in real time and broadcasts only the changed values to the synthesizer.
5. Creative Sound Dice with Ear Protection (Safe Randomizer): Sound designers often roll digital dice to discover surprising new patches. However, completely wild random numbers can set resonance or distortion to maximum, generating ear-splitting screeching noises that could damage human hearing or blow out studio speakers. Our safe randomizer provides protective boundaries: it allows high randomness across musical parameters while enforcing strict, automatic safety ceilings on dangerous controls (filter resonance, filter overdrive, filter post-drive, distortion, and delay feedback).
6. Tactile Hands-On Mixing Console (Korg nanoKONTROL2 Surface Controller): Performing musicians prefer physical sliders and knobs over clicking with a computer mouse. Connecting a physical Korg nanoKONTROL2 gives eight channel sliders, eight knobs, solo/mute buttons, transport controls, and performance buttons. To prevent sudden volume spikes when touching a physical slider that rests in a different position from the loaded preset, our soft takeover system requires the physical fader to catch up with or smoothly pick up the stored setting before moving the sound.

We define all specialized industry concepts on first use:

- Preset Patch: A saved digital document containing all knob, slider, waveform, and routing settings that define an individual synthesizer instrument sound.
- Bank: A standardized container holding a group of 128 preset patches.
- Continuous Controller (CC): A standard MIDI message used to adjust a sound parameter (such as volume, brightness, or effect level) with 7-bit resolution (values from 0 to 127).
- Program Change (PC): A MIDI command instructing a sound module to switch to a specific sound preset index (0 to 127).
- System Exclusive (SysEx): Manufacturer-specific MIDI communication packets used for bulk preset transfers, firmware updates, and parameter dumps.
- Checksum: A mathematical validation byte calculated by adding payload values together and taking the two's complement, used to detect data corruption during transmission.
- Soft Takeover: A controller behavior algorithm that avoids parameter jumps when a physical hardware dial does not match the active digital sound parameter.
- Jump Mode: The sound parameter jumps immediately to match the physical hardware dial position as soon as it is touched.
- Catch Mode: The sound parameter stays locked at its stored preset value until the physical hardware dial moves past and crosses that exact value.
- Pickup Mode: The sound parameter moves proportionally toward the physical dial, converging smoothly without sudden audio jumps.
- Parameter Locking: Shielding specific sound controls (like tuning, master volume, or filter mode) so that randomization algorithms never alter them.
- Safety Cap: A hard numerical upper bound enforced in software to prevent extreme feedback, self-oscillation, or harsh digital clipping.

The following table connects these everyday musical goals directly to their technical implementations in the codebase:

| Everyday Musical Goal | Technical Implementation | Code Path and Symbol |
| :--- | :--- | :--- |
| Store and organize 512 sounds | 4 Banks x 128 Slots Preset Manager | `PeakLibrarian`, `src/peak_librarian.rs` |
| Curated sound showcase | 8 Authentic Peak Factory Presets | `create_curated_peak_patches` |
| Back up and restore patches | SysEx Sound Dump & Bank Dump State Machine | `PeakSysExStateMachine`, `SysExEvent` |
| Detect corrupted MIDI files | 7-bit Two's Complement Checksum Verifier | `calculate_peak_checksum`, `verify_peak_checksum` |
| Compare two sounds side by side | Parameter Diff and Inspection | `diff_patches`, `synth_core::PatchDiff` |
| Copy specific settings between sounds | Selective Parameter and Section Merging | `merge_selected_params`, `merge_section` |
| Smoothly transition between sounds | Continuous A/B Morph Fader (0.0 to 1.0) | `ContinuousMorphFader`, `morph_patches` |
| Roll creative sound dice safely | Safe Randomizer with 5 CC Safety Caps | `PeakSafeRandomizer`, `apply_safety_caps` |
| Hands-on hardware mixing | Korg nanoKONTROL2 8-Strip Controller | `NanoKontrol2Surface`, `NanoSurfaceAction` |
| Smooth knob movement without jumps | Jump, Catch, and Pickup Soft Takeover | `SoftTakeoverState`, `SoftTakeoverMode` |
| Dramatic performance button triggers | Animate 1 & 2 Triggers with LED Feedback | `AnimateTrigger`, CC 114, CC 115 |

---

## 2. Eight Curated Novation Peak Factory Patches

We implemented eight factory presets in `src/peak_librarian.rs` via `create_curated_peak_patches()`. Each preset showcases a distinct sonic capability of the Novation Peak:

1. Init Patch (Bank A, Slot 0, Category "Init"): Clean single saw oscillator with open filter (cutoff 127, resonance 0), neutral envelope sustain, and zero effects, providing a clean starting slate for fresh sound design.
2. Oxford FPGA Pad (Bank A, Slot 1, Category "Pad"): Warm ambient pad utilizing Oxford FPGA wavetables "Harmonic 1" and "Harmonic 2", fine oscillator detune (+12 cents), slow attack (75) and long release (85), layered with deep stereo chorus (70) and expansive hall reverb (65).
3. Acid Overdrive Bass (Bank A, Slot 2, Category "Bass"): Punchy monophonic acid bassline pairing a saw oscillator with a sub-octave pulse oscillator, resonant low-pass filter (cutoff 42, resonance 60), analog filter overdrive (48), post-filter drive (35), snappy mod envelope decay (38), and analog distortion (42).
4. VSync Lead (Bank A, Slot 3, Category "Lead"): Hard oscillator sync lead utilizing Peak's virtual sync (VSync values of 85 and 70), smooth portamento pitch glide (time 35), open filter (92), and tempo-synced stereo delay (55).
5. Ring Mod Bell (Bank A, Slot 4, Category "Bell"): Metallic FM chime utilizing high ring modulation mix (115), pure interval oscillator detune (+7 semitones), fast attack (0), natural bell decay (65), zero sustain, and shimmer reverberation (65).
6. Lush Chorus Strings (Bank A, Slot 5, Category "Strings"): Ensemble string machine featuring three detuned saw oscillators (-8 and +9 cents), keyboard filter tracking (90), medium attack (50) and release (60), deep multi-voice chorus (80), and concert hall reverb (50).
7. Dub Delay Chord (Bank A, Slot 6, Category "Chord"): Minor triad dub stab with three oscillators tuned to root, minor third (+3 semitones), and fifth (+7 semitones), filtered through a dark resonant lowpass filter (cutoff 52, resonance 40) into high-feedback tape delay (level 75, feedback 65) and plate reverb (55).
8. Animate Morph Arp (Bank A, Slot 7, Category "Arp"): Dynamic sequenced arpeggio with clock gate control (80), fast punchy envelopes, delay (45), reverb (35), and pre-mapped Animate 1 and 2 macro button destinations.

---

## 3. 512-Patch Bank Librarian Architecture

The Novation Peak stores patches in four discrete banks (A, B, C, D), with each bank containing 128 slots (0 to 127), providing exactly 512 patch locations.

The `PeakLibrarian` struct manages this complete collection:

- Direct Memory Array: `banks: [Vec<SynthPatch>; 4]` provides direct $O(1)$ indexing by bank and slot index.
- Backing Search Index: `library: synth_core::PresetLibrary` maintains UUID tracking, tag indexes, search caches, and JSON serialization.
- Selection Protocol: Calling `select_patch(bank, slot)` updates internal active pointers and generates the three-packet MIDI sequence required by Novation hardware:
  1. Bank Select MSB: Control Change 0, Value 0
  2. Bank Select LSB: Control Change 32, Value 0 to 3 (corresponding to Banks A through D)
  3. Program Change: Value 0 to 127
- Full CRUD Support: Supports adding, updating, removing, searching, filtering by bank or category, bookmarking favorites, and exporting the entire 512-patch collection to JSON format.

---

## 4. SysEx State Machine and Checksum Verification

System Exclusive data transmission enables sending and receiving exact synthesizer memory dumps over MIDI cables.

### Novation Peak SysEx Frame Layout

Each patch SysEx frame adheres to the standard Focusrite/Novation layout:
- Delimiter: `0xF0` (System Exclusive Start)
- Manufacturer ID: `0x00 0x20 0x29` (Novation 3-byte identifier)
- Device Family ID: `0x03` (Novation Peak / Summit)
- Command Byte: `0x01` (Single Patch Sound Dump) or `0x02` (Sound Bank Dump)
- Bank Index: `0x00` to `0x03` (Banks A through D)
- Slot Index: `0x00` to `0x7F` (Slots 0 to 127)
- Patch Name: Exactly 16 ASCII bytes (space-padded if shorter)
- Parameter Payload: 7-bit data bytes representing synth settings
- Checksum: 1 byte (7-bit two's complement)
- Delimiter: `0xF7` (End of Exclusive)

### Checksum Mathematical Formulation

Novation Peak computes a two's complement 7-bit checksum over the payload:

$$\text{Checksum} = (128 - (\sum_{i} \text{byte}_i \pmod{128})) \pmod{128}$$

Our verification function `verify_peak_checksum` validates this formula and supports `0x7F` as a universal bypass byte, matching hardware bootloader standards.

### Streaming State Machine

`PeakSysExStateMachine` handles streaming byte streams:
- `feed_byte(b: u8) -> Option<SysExEvent>` and `feed_buffer(bytes: &[u8]) -> Vec<SysExEvent>` consume streaming data.
- It parses headers, verifies manufacturer IDs, unpacks 16-character names, and verifies checksums.
- It automatically tracks multi-patch bank dump reception, emitting `BankDumpProgress` events for each received slot and `BankDumpComplete` once all 128 patches have been collected and verified.

---

## 5. Continuous A/B Patch Morph Fader

The `ContinuousMorphFader` allows performers to smoothly transition between any two patches using a normalized control interval $t \in [0.0, 1.0]$.

### Linear Interpolation

For each parameter present in both Patch A and Patch B, the effective value is calculated as:

$$V(t) = V_A \cdot (1.0 - t) + V_B \cdot t$$

Discrete parameters (such as oscillator waveforms or filter slopes) transition cleanly at the 50% threshold ($t = 0.5$).

### Real-Time Delta Emission

To avoid overwhelming MIDI ports with redundant data, moving the fader emits `MorphDelta` records only for parameters whose values have changed by at least 0.5 units. The fader also integrates directly with `synth_core::SoftTakeoverState` with a 2% normalized capture threshold ($0.02$), ensuring that physical fader movements do not cause unexpected parameter jumps.

---

## 6. A/B Patch Diff and Selective Merge

Sound designers frequently want to inspect differences between sound versions or import settings from one sound into another.

Reusing `synth_core::PatchDiff` directly:
- `diff_patches(a, b)` calculates total differences, listing added, removed, modified, and unchanged parameters.
- `merge_selected_params(target, source, keys)` copies only specific named parameters from the source patch into the target patch.
- `merge_section(target, source, prefix)` copies an entire module (such as all parameters starting with `"filter_"` or `"osc_1_"`) in a single operation.

---

## 7. Lock-Shielded Safe Randomizer with Automatic Safety Caps

Random sound generators can produce ear-damaging sounds if dangerous controls are set to maximum. `PeakSafeRandomizer` provides sound design exploration while protecting hearing and hardware.

### Automatic Safety Caps

Regardless of the randomization amount, density, or algorithm used, the randomizer automatically enforces upper safety limits on five critical parameters:

1. CC 36 (Filter Post Drive): Capped at `50.0` (maximum 127.0) to prevent harsh digital clipping.
2. CC 79 (Filter Resonance): Capped at `62.0` (maximum 127.0) to prevent loud, runaway self-oscillation.
3. CC 80 (Filter Overdrive): Capped at `50.0` (maximum 127.0) to preserve clean amplifier headroom.
4. CC 104 (Distortion Level): Capped at `48.0` (maximum 127.0) to protect speaker cones.
5. CC 110 (Delay Feedback): Capped at `70.0` (maximum 127.0) to prevent infinite feedback buildup.

### Operating Modes and Locking

- Global Randomization: Randomizes all unlocked parameters across the entire instrument.
- Section-Scoped Randomization: Targets specific modules using the `PeakRandomizeSection` enum:
  - `Osc`: Oscillators 1, 2, and 3, shapes, coarse/fine tuning, vsync, ring mod, and noise.
  - `Filter`: Cutoff, resonance, overdrive, post drive, tracking, and filter envelope depth.
  - `Env`: Amp envelope and Modulation Envelopes 1 and 2.
  - `Lfo`: LFO 1 and 2 rates, sync divisions, and fade times.
  - `Fx`: Distortion, chorus, delay, and reverb.
- Gaussian Drift: Applies bell-curve mutations centered around existing patch values for subtle variation.
- Parameter and Section Locks: Sensitive controls (master volume, pitch bend, modulation wheel, portamento glide time) are locked by default, and users can lock or unlock any individual control or section.

---

## 8. Korg nanoKONTROL2 Surface Controller Integration

The Korg nanoKONTROL2 (`hw:2,0,0`) is a USB control surface featuring eight channel strips and transport controls.

`NanoKontrol2Surface` provides complete bidirectional mapping:

### Control Strip Mappings

Each of the eight channel strips is mapped to primary Novation Peak parameters:

| Strip | Slider (CC 0 to 7) | Target Parameter | Knob (CC 16 to 23) | Target Parameter |
| :---: | :---: | :---: | :---: | :---: |
| 1 | CC 0 | Filter Cutoff (CC 29) | CC 16 | Filter Overdrive (CC 80) |
| 2 | CC 1 | Filter Resonance (CC 79) | CC 17 | Filter Post Drive (CC 36) |
| 3 | CC 2 | Oscillator 1 Mix (CC 23) | CC 18 | Oscillator 1 Shape (CC 12) |
| 4 | CC 3 | Oscillator 2 Mix (CC 24) | CC 19 | Oscillator 2 Shape (CC 39) |
| 5 | CC 4 | Oscillator 3 Mix (CC 25) | CC 20 | LFO 1 Rate (CC 30) |
| 6 | CC 5 | Distortion Level (CC 104) | CC 21 | LFO 2 Rate (CC 31) |
| 7 | CC 6 | Delay Level (CC 108) | CC 22 | Delay Time (CC 109) |
| 8 | CC 7 | Reverb Level (CC 112) | CC 23 | Delay Feedback (CC 110) |

### Performance Buttons and LED Feedback

- Animate 1 Trigger: Strip 1 Solo button (CC 32) triggers Animate 1 Hold (CC 114) and toggles its front-panel LED.
- Animate 2 Trigger: Strip 2 Solo button (CC 33) triggers Animate 2 Hold (CC 115) and toggles its front-panel LED.
- Solo, Mute, and Rec Arm: Strips 3 through 8 provide toggle switches with matching LED feedback packets.
- Transport Controls: Play (CC 41), Stop (CC 42), Rewind (CC 43), Fast Forward (CC 44), Record (CC 45), Cycle (CC 46), Track Previous/Next (CC 58/59), and Markers (CC 60 through 62).
- Soft Takeover Tracking: All 16 analog faders and knobs maintain independent `SoftTakeoverState` trackers supporting Jump, Catch, and Pickup modes.

---

## 9. Verification and Test Results

Automated verification confirms clean builds and test passage across all workspace crates and Node.js test suites.

### Test Execution Metrics

```
cargo test --workspace (Jobs = 2)
  novation-peak-manager: 43 unit tests passed (0 failed)
  synth-core:            34 unit tests passed (0 failed)
  synth-midi-osc:        33 unit tests passed (0 failed)
  synth-ui-theme:        24 unit tests passed (0 failed)
  Rust Tests Total:     134 passed, 0 failed, 0 warnings

npm test (konduktiva-peak & shared-synth-sdk)
  Node.js Suites:         8 passed (0 failed)
  Stress Capture Test:   PASS (all 8 players active, wire channel 0, 5/5 CCs present)

Grand Total: 142 tests passing across all layers
```

### Compiler and Hardware Safety Compliance

- Zero Compiler Warnings: Both `cargo check --workspace --all-targets` and `cargo test --workspace` complete with zero warnings.
- Safe Default Offline Mode: All automated tests default to `MidiSafetyMode::VirtualLoopback`, preventing unexpected MIDI output to physical ports `hw:3,0,0`, `hw:0,0,0`, or `hw:2,0,0`.
- Resource Discipline: Built strictly with `CARGO_BUILD_JOBS=2` using the isolated external SSD target directory.
