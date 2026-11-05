---
title: "Report 004: Epoch 4, Novation Peak 3x ADSR Envelopes, Dual Tempo-Synced LFOs, 16-Slot Mod Matrix, 4-Slot FX Matrix, Animate Macros, Arpeggiator and Master FX Chain"
date: 2026-11-06 02:58:00
project: novation-peak-pigs
project_dir: /home/renick/antigravity/novation-peak-pigs
report_number: 4
subject: "3x ADSR Envelopes, Dual Tempo-Synced LFOs, 16-Slot Mod Matrix, 4-Slot FX Matrix, Animate Macros, Arpeggiator and Master FX Chain"
operator: "Antigravity (Gemini 3.8 Flash)"
follows: ["report-003.md"]
tldr: "Epoch 4 implements the complete modulation architecture, performance macros, arpeggiation, portamento glide, and master effects processing engine for the Novation Peak 8-voice hybrid synthesizer in src/peak_mod_fx.rs, wired directly into src/lib.rs. The engine provides three independent ADSR envelopes (Amp, Mod 1, Mod 2) with deterministic curve generation and exponential decay response; two tempo-synced multi-waveform LFOs spanning sub-audio and audio-rate FM ranges; a 16-slot dual-source modulation matrix with bipolar depths (-64 to +63); a dedicated 4-slot FX modulation matrix; Animate 1 and 2 performance buttons with live parameter offset preview; an arpeggiator with clock gate duty-cycle control; pitch glide slewing; and a stereo master effects chain incorporating analog soft saturation distortion, 3-mode chorus, tempo-synced delay with feedback damping, and 3-type algorithmic reverberation. All 132 tests (124 Rust workspace unit tests and 8 Node.js tests) pass with zero compiler warnings."
tests_total: 132
qa_verdict: PASS
---

# Prompt

```
[FORMAL INTENT]: Epoch 4 of 8: 3x ADSR Envelopes, Dual Tempo-Synced LFOs, 16-Slot Mod Matrix + 4-Slot FX Matrix, Animate 1/2 Macros, Arpeggiator & Master FX Chain (Target: src/peak_mod_fx.rs; Related: src/lib.rs, src/peak_synth.rs). Implement the complete Novation Peak modulation and effects engine in src/peak_mod_fx.rs and wire into src/lib.rs: 3 ADSR envelopes (Amp CC86-89, Mod 1 CC90-93, Mod 2 CC94/95/117/103 with deterministic ADSR curve generator), 2 LFOs (Rate CC30/31, Sync Rate CC81/84, Fade CC82/85, Range CC83, LFO->Pitch/Shape/Filter depths), 16-slot dual-source Modulation Matrix (Source A x Source B -> Destination with bipolar depth -64..+63) + 4-slot FX Mod Matrix, Animate 1 & 2 performance macro buttons (CC114/CC115) with live parameter offset preview, Arpeggiator gate/clock (CC116) & Glide (CC5/CC35), and Master FX chain (Analogue Distortion CC104, 3-mode Chorus CC105/107/118, Sync Delay CC108/109/110, 3-type Reverb CC112/113) with unit tests. Ensure all workspace tests pass (CARGO_BUILD_JOBS=2 cargo test --workspace and npm test). Write reports/report-004.md and compile it with /home/renick/claude/html-report-generator/build-report.sh --pdf.
```

# Content

## 1. Overview and Everyday Purpose

To understand how a musical synthesizer brings static electronic tones to life, imagine how a skilled instrumentalist plays an acoustic violin or brass horn. A musician does not simply produce a flat, unvarying buzz:

1. Volume and Tone Over Time (Envelopes): When a trumpet is blown, the volume attacks quickly, settles to a sustained level, and then dies away when the player stops blowing. We shape volume and brightness over time using Envelopes (ADSR: Attack, Decay, Sustain, Release).
2. Vibrato and Movement (LFOs): A violinist gently rocks their finger on the string to add a shimmering vibrato. In electronic synthesizers, Low-Frequency Oscillators (LFOs) create this rhythmic, repeating motion in pitch, tone color, or loudness.
3. Flexible Patch Cord Routing (Modulation Matrix): Vintage modular synthesizers used physical cables to connect control dials to sound generators. The Novation Peak features an internal digital switchboard (called a Modulation Matrix) that connects 17 performance sources to 32 sound destinations with fine positive or negative depth control.
4. Instant Performance Buttons (Animate Macros): Two dedicated front-panel buttons allow the performer to trigger dramatic sonic changes with a single touch, such as opening the filter, adding delay, or speeding up vibrato.
5. Automatic Melodic Rhythms (Arpeggiator) and Smooth Pitch Slides (Glide): The arpeggiator turns held chords into dancing rhythmic patterns, while glide (portamento) smoothly glides pitch from one note to the next like a trombone slide.
6. Studio Polishing (Master Effects Chain): Warm analog distortion adds harmonic bite, chorus widens the stereo image, delay adds rhythmic echoes, and reverberation places the instrument inside an acoustic room, concert hall, or cathedral.

In this development epoch, we implemented this entire modulation, macro, arpeggio, and stereo effects system in `src/peak_mod_fx.rs` and integrated its full public interface into `src/lib.rs`.

We define all specialized industry concepts on first use:

- ADSR Envelope: A four-stage shape generator defining how a sound changes from key press to silence: Attack (rise time), Decay (fall time to sustain), Sustain (held volume), and Release (fade out after releasing key).
- LFO (Low-Frequency Oscillator): A cyclic waveform generator operating below human hearing (or up to audio frequency for FM effects) used to modulate pitch, timbre, or volume.
- Modulation Matrix: A configurable routing grid that connects control sources (like mod wheels, touch pressure, or envelopes) to sound parameters (like filter frequency or pitch).
- Dual-Source Modulation: A routing where Source A is scaled by Source B before affecting the destination (for example, LFO 1 vibrato depth controlled by the Mod Wheel).
- FX Modulation Matrix: A dedicated four-slot sub-matrix on the Novation Peak focused exclusively on animating delay, reverb, chorus, and distortion settings.
- Portamento (Glide): A continuous smooth slide in pitch between consecutive musical notes.
- Gate Duty Cycle: The percentage of time a musical note sounds during a rhythmic step before turning off.
- Comb Filter: An audio delay circuit with feedback that produces harmonically spaced peaks and notches like the teeth of a comb.
- All-Pass Filter: A filter circuit that preserves all frequencies equally while shifting phase relationships to diffuse and smooth acoustic echoes.

The following table pairs everyday musical concepts with their exact code components:

| Everyday Musical Concept | Technical Parameter / Feature | Code Path and Symbol |
| :--- | :--- | :--- |
| Note start and fade-out shaping | 3x ADSR Envelopes (Amp, Mod 1, Mod 2) | `AdsrEnvelope`, `src/peak_mod_fx.rs` |
| Vector visualizer for envelope shape | Deterministic ADSR Curve Generator | `generate_adsr_curve`, `to_envelope_curve_model` |
| Vibrato, tremolo, and rhythmic pulse | Dual Tempo-Synced LFOs | `LfoProcessor`, `LfoSyncDivision` |
| Custom routing switchboard | 16-Slot Dual-Source Mod Matrix | `ModMatrix`, `ModMatrixSlot` |
| Dedicated effects routing | 4-Slot FX Modulation Matrix | `FxModMatrix`, `FxModMatrixSlot` |
| Live touch buttons and visual preview | Animate 1 & 2 Performance Macros | `calculate_animate_offsets`, `ParamOffsetPreview` |
| Rhythmic pattern generator | Arpeggiator Clock Gate & Sequencer | `ArpEngine`, `ArpEvent`, CC 116 |
| Smooth trombone-style pitch slide | Portamento Pitch Glide Slewing | `GlideProcessor`, CC 5, CC 35 |
| Warm tube-style overdrive | Analogue Distortion Processor | `AnalogueDistortion`, CC 104 |
| Stereo widening and ensemble motion | 3-Mode Stereo Chorus (Classic/Ensemble/Flanger) | `ChorusFx`, `ChorusMode`, CC 105/107/118 |
| Rhythmic stereo echoes | Tempo-Synced Damped Delay | `SyncDelayFx`, CC 108/109/110 |
| Room, Hall, and Arena space simulation | 3-Type Algorithmic Reverb | `ReverbFx`, `ReverbType`, CC 112/113 |

## 2. 3x ADSR Envelopes and Deterministic Curve Generator

The Novation Peak provides three distinct envelope generators per voice:

1. Amplifier Envelope (Amp Env): Shapes overall voice volume. Controlled via CC 86 (Attack), CC 87 (Decay), CC 88 (Sustain), and CC 89 (Release).
2. Modulation Envelope 1 (Mod Env 1): Primarily assigned to filter frequency modulation and oscillator shape morphing. Controlled via CC 90 (Attack), CC 91 (Decay), CC 92 (Sustain), and CC 93 (Release).
3. Modulation Envelope 2 (Mod Env 2): Primarily assigned to pitch modulation and auxiliary modulations. Controlled via CC 94 (Attack), CC 95 (Decay), CC 117 (Sustain), and CC 103 (Release). Note that Sustain and Release use non-contiguous CC numbers matching the physical Peak panel specification.

In `src/peak_mod_fx.rs`, our `AdsrEnvelope` struct implements real-time per-sample evaluation with authentic analog characteristics:

- Exponential response curves: Attack times range exponentially from 1 millisecond up to 18 seconds via `cc_to_attack_time`. Decay and release times range exponentially from 2 milliseconds to 24 seconds via `cc_to_decay_time` and `cc_to_release_time`.
- Deterministic Curve Generator: The function `generate_adsr_curve` computes an array of `AdsrCurvePoint` structs containing normalized time, amplitude (0.0 to 1.0), and stage markers (`Attack`, `Decay`, `Sustain`, `Release`). This powers consistent visual curve rendering in the graphical user interface.
- Shared UI Theme Integration: The helper function `to_envelope_curve_model` converts MIDI CC values directly into a `synth_ui_theme::widgets::EnvelopeCurveModel`, allowing direct vector display on egui canvases without redundant translation layers.

The table below outlines the continuous controller mapping and time constants for all three envelopes:

| Envelope Channel | Attack CC & Range | Decay CC & Range | Sustain CC & Range | Release CC & Range | Primary Routing Target |
| :--- | :--- | :--- | :--- | :--- | :--- |
| Amp Envelope | CC 86 (1ms to 18s) | CC 87 (2ms to 24s) | CC 88 (0.0 to 1.0) | CC 89 (2ms to 24s) | Voice Output Level (VCA) |
| Mod Envelope 1 | CC 90 (1ms to 18s) | CC 91 (2ms to 24s) | CC 92 (0.0 to 1.0) | CC 93 (2ms to 24s) | Filter Cutoff (CC 78), Osc Shape (CC 119/40) |
| Mod Envelope 2 | CC 94 (1ms to 18s) | CC 95 (2ms to 24s) | CC 117 (0.0 to 1.0) | CC 103 (2ms to 24s) | Osc Pitch (CC 9/38/43) |

## 3. Dual Tempo-Synced LFOs

The Novation Peak includes two low-frequency oscillators (LFO 1 and LFO 2) per voice:

- Waveforms: Triangle (smooth vibrato), Sawtooth (percussive ramp), Square (trill/pulsing), and Sample & Hold (stepped organic random motion).
- Free-Running vs. Tempo-Synced Rates:
  * Free-running rate: Controlled by CC 30 for LFO 1 and CC 31 for LFO 2, spanning 0.01 Hz to 20.0 Hz in standard Low range.
  * Tempo-synced rate: Controlled by CC 81 for LFO 1 and CC 84 for LFO 2, locked to musical divisions from 32 bars down to 1/32 triplet based on workspace tempo (BPM).
  * Audio-Rate FM Range: LFO 2 Range (CC 83) switches the oscillator from Low range (sub-audio) to High range (1.0 Hz up to 1600.0 Hz), allowing audio-rate frequency modulation for complex metallic and bell textures.
- Fade-In Envelope: Controlled by CC 82 for LFO 1 and CC 85 for LFO 2. When a key is pressed, LFO amplitude ramps up smoothly from 0.0 to 1.0 over a configurable fade time, preventing vibrato from distracting during the initial attack transient.
- Front-Panel Direct Depths:
  * Pitch Modulation: Osc 1 LFO 2 Pitch (CC 16), Osc 2 LFO 2 Pitch (CC 19), Osc 3 LFO 2 Pitch (CC 22).
  * Shape Modulation: Osc 1 LFO 1 Shape (CC 33), Osc 2 LFO 1 Shape (CC 41), Osc 3 LFO 1 Shape (CC 73).
  * Filter Cutoff Modulation: LFO 1 to Filter Depth (CC 28).

Our implementation in `LfoProcessor` maintains phase continuity, handles clock division updates, and evaluates output samples with zero dynamic memory allocation.

## 4. 16-Slot Dual-Source Modulation Matrix and 4-Slot FX Matrix

The Novation Peak modulation architecture employs a dual-source multiplication scheme:

$$\text{Effective Modulation} = \text{Source A} \times \text{Source B} \times \frac{\text{Depth}}{64}$$

When Source B is set to `Direct`, its value is 1.0, and Source A modulates the destination with the configured depth (-64 to +63). When Source B is set to a performance controller (such as Mod Wheel, Aftertouch, or an Envelope), Source B acts as a dynamic volume control over Source A.

### 16-Slot Modulation Matrix

- 17 Modulation Sources (`ModSource`): `Off`, `Direct`, `ModWheel`, `Aftertouch`, `PitchBend`, `Expression`, `Breath`, `Velocity`, `KeyTrack`, `Lfo1`, `Lfo2`, `AmpEnv`, `ModEnv1`, `ModEnv2`, `Animate1`, `Animate2`.
- 32 Modulation Destinations (`ModDestination`): Individual and grouped oscillator pitch (`Osc1Pitch`, `Osc2Pitch`, `Osc3Pitch`, `OscAllPitch`), wavetable shapes (`Osc1Shape`, `Osc2Shape`, `Osc3Shape`, `OscAllShape`), virtual sync (`Osc1Vsync`, `Osc2Vsync`, `Osc3Vsync`, `OscAllVsync`), mixer levels (`Osc1Mix`, `Osc2Mix`, `Osc3Mix`, `RingModMix`, `NoiseMix`), filter cutoff, resonance, overdrive, post-drive, LFO rates, and envelope attack/decay/release times.

### 4-Slot FX Modulation Matrix

The Novation Peak includes a dedicated four-slot matrix specifically for modulating studio effects:

- Destinations (`FxModDestination`): Distortion Level, Chorus Level, Chorus Rate, Delay Level, Delay Time, Delay Feedback, Reverb Level, and Reverb Time.
- Real-time modulation enables expressive performance gestures, such as increasing reverb decay with aftertouch or opening delay feedback when pressing the Animate button.

The table below lists key routing examples supported by the modulation matrices:

| Matrix Slot | Source A | Source B | Destination | Bipolar Depth | Musical Effect |
| :--- | :--- | :--- | :--- | :---: | :--- |
| Slot 1 | Lfo1 | ModWheel | OscAllPitch | +32 | Mod wheel controls vibrato intensity |
| Slot 2 | ModEnv1 | Direct | FilterCutoff | +64 | Envelope sweeps filter brightness |
| Slot 3 | Velocity | Direct | AmpEnvAttack | -24 | Harder key strikes produce faster attack |
| Slot 4 | Animate1 | Direct | FilterCutoff | +48 | Animate 1 button jumps filter open |
| FX Slot 1 | Expression | Direct | ReverbLevel | +50 | Foot pedal swells ambient reverb wash |
| FX Slot 2 | Animate2 | Direct | DelayFeedback | +40 | Animate 2 button triggers dub delay repeat |

## 5. Animate 1 and 2 Performance Macros with Live Offset Preview

The Novation Peak features two rubberized performance macro buttons on the front panel: Animate 1 (CC 114) and Animate 2 (CC 115). These can be pressed momentarily or latched.

When pressed, they transmit continuous controller values (0 when released, 127 when held) and inject modulation through both the 16-slot Modulation Matrix and the 4-slot FX Modulation Matrix.

In `src/peak_mod_fx.rs`, our `calculate_animate_offsets` function evaluates the exact real-time parameter changes that occur when Animate 1 or Animate 2 is engaged. It returns structured preview records (`ParamOffsetPreview` and FX offset tuples) containing:

1. Target Destination Parameter
2. Baseline unmodulated patch value
3. Net offset added by the active Animate buttons
4. Resulting effective parameter value

This allows the user interface and patch librarian to display an interactive live preview of parameter changes before or during performance.

## 6. Arpeggiator Clock Gate and Pitch Glide (Portamento)

### Arpeggiator Engine (`ArpEngine`)

The Peak arpeggiator transforms held chords into sequential melodic patterns:

- Clock Gate (CC 116): Regulates gate duty cycle from 5% (staccato blips) up to 98% (legato overlaps) of the step duration.
- Sequencing Modes: `Up`, `Down`, `UpDown1`, `UpDown2`, `AsPlayed`, `Random`, and `Chord`.
- Clock Divisions: 1/4, 1/4 Triplet, 1/8, 1/8 Triplet, 1/16, 1/16 Triplet, 1/32, and 1/32 Triplet.
- Octave Multiplication: Spans 1 to 4 octaves.
- Event Generation: Outputs discrete `ArpEvent::NoteOn` and `ArpEvent::NoteOff` events timed to the system tempo without audio dropouts.

### Portamento Pitch Glide (`GlideProcessor`)

Glide creates smooth transitions between consecutive pitches:

- Glide Time (CC 5): Exponential slewing duration ranging from 5 milliseconds up to 5.0 seconds.
- Glide On/Off (CC 35): Switch parameter (0 to 63 off, 64 to 127 on).
- Continuous Slewing: Evaluates exponential pitch tracking on every audio sample:

$$\text{Pitch}_{\text{current}} = \text{Pitch}_{\text{current}} + (\text{Pitch}_{\text{target}} - \text{Pitch}_{\text{current}}) \times (1 - e^{-\Delta t / \tau})$$

## 7. Master Effects Chain: Distortion, Chorus, Delay, and Reverb

The final output stage of the Novation Peak passes through four high-grade stereo audio effects:

### 1. Post-Filter Analogue Distortion (`AnalogueDistortion`, CC 104)

- Models the non-linear saturation of the analog distortion circuit located immediately after the state-variable filter.
- Employs a hyperbolic tangent (`tanh`) saturation function with adaptive gain compensation, adding rich second- and third-order harmonics without harsh digital clipping.

### 2. Stereo 3-Mode Modulated Chorus (`ChorusFx`, CC 105/107/118)

- Level (CC 105), Feedback (CC 107), and Rate (CC 118, 0.1 Hz to 5.0 Hz).
- Modes:
  * Mode 1 (Classic): Subtle stereo chorusing with 15ms base delay and gentle pitch modulation.
  * Mode 2 (Ensemble): Lush multi-phase ensemble thickening with 25ms base delay.
  * Mode 3 (Flanger): Resonant comb filtering with 3ms base delay and regenerative feedback.
- Quadrature Stereo Modulation: Modulates left and right channels with a 90-degree phase difference for broad, immersive stereo width.

### 3. Tempo-Synced Damped Delay (`SyncDelayFx`, CC 108/109/110)

- Level (CC 108), Time (CC 109, 20ms to 1.5s or tempo divisions), and Feedback (CC 110).
- Ping-pong stereo cross-feedback architecture.
- Feedback Lowpass Damping: An internal single-pole lowpass filter in the recirculation path softens high frequencies on each repeat, emulating vintage analog tape echoes.

### 4. Algorithmic Stereo Reverb (`ReverbFx`, CC 112/113)

- Level (CC 112), Decay Time (CC 113, 0.2s up to 20s ambient wash).
- Room Types:
  * Room: Dense early reflections with rapid decay for intimate acoustic drum and synth spaces.
  * Hall: Expansive concert hall acoustic with balanced diffusion.
  * Arena: Massive cathedral space with extended high-frequency decay.
- Architecture: An eight-line parallel feedback comb filter matrix followed by four cascaded all-pass diffusers, delivering rich acoustic density without metallic ringing.

The complete signal flow through `MasterFxChain::process` is illustrated below:

$$\text{Voice Sum} \longrightarrow \text{Distortion} \longrightarrow \text{Chorus} \longrightarrow \text{Delay} \longrightarrow \text{Reverb} \longrightarrow \text{Master Stereo Out}$$

## 8. Test Coverage, Workspace Integration, and QA Verdict

All modules were verified through comprehensive unit tests and automated regression suites.

The test suite covers:
- ADSR envelope parameter conversions, stage state machines, exponential releases, and vector visualization curve generation.
- Dual LFO waveforms (Triangle, Sawtooth, Square, Sample & Hold), tempo-sync calculations, and audio-rate FM frequency scaling.
- Modulation matrix 16-slot dual-source multiplication and 4-slot FX matrix evaluations.
- Live Animate macro offset previews.
- Arpeggiator clock gate duty cycle and note event generation.
- Portamento pitch glide slewing.
- Master effects chain audio rendering, soft saturation, and stereo diffusion.

The table below summarizes test results across all workspace crates and companion packages:

| Test Suite / Crate | Test Scope | Number of Tests | Result |
| :--- | :--- | :---: | :---: |
| `novation_peak_manager` (Rust lib) | Modulation, Envelopes, LFOs, Matrix, FX, Hardware, MIDI | 33 | PASS |
| `synth_core` (Rust shared crate) | Profiles, Bundles, Patches, Diff, History, Randomizer | 34 | PASS |
| `synth_midi_osc` (Rust shared crate) | Hardware Fingerprints, SysEx, NRPN, MIDI Routing, OSC | 33 | PASS |
| `synth_ui_theme` (Rust UI crate) | Vector Curves, Knobs, Themes, LED Buttons, Drum Pads | 24 | PASS |
| `konduktiva-peak` (Node.js companion) | Stress Test, MIDI Routing, 8 Players, Safety Clamping | 8 | PASS |
| Total Tests Verified | Entire Novation Peak Ecosystem | 132 | PASS |

Zero compiler warnings were emitted across all crates. Automated tests operated under Virtual Loopback mode in strict compliance with hardware safety invariants.
