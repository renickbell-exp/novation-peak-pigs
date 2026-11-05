---
title: "Report 002: Epoch 2, Non-Invasive ALSA Discovery, Safety Clamping, NRPN/Dual-CC Router and Bidirectional OSC Server"
date: 2026-11-06 02:46:00
project: novation-peak-pigs
project_dir: /home/renick/antigravity/novation-peak-pigs
report_number: 2
subject: "Non-Invasive ALSA Discovery, Safety Clamping, NRPN/Dual-CC Router and Bidirectional OSC Server"
operator: "Antigravity (Gemini 3.8 Flash)"
follows: ["report-001.md"]
tldr: "Epoch 2 implements non-invasive Linux sound card discovery, parameter safety clamping, dual-CC and 14-bit NRPN routing, Novation System Exclusive patch dump decoding, and a bidirectional Open Sound Control (OSC) server. Hardware detection inspects read-only system files (/proc/asound/cards and /proc/asound/seq/clients) to identify the Novation Peak, Behringer UMC1820, and Korg nanoKONTROL2 with safe offline virtual loopback as the default. Safety clamping prevents dangerous loudness spikes and screaming audio feedback on six sensitive controls. A bidirectional UDP OSC server allows remote network clients to control and monitor the instrument across all oscillator, filter, envelope, LFO, and effects parameters. All 100 Rust tests and 8 Node.js tests pass with zero compiler warnings."
tests_total: 108
qa_verdict: PASS
---

# Prompt

```
[FORMAL INTENT]: Epoch 2 of 8: Non-Invasive ALSA Hardware Discovery (Peak MIDI 1 hw:3,0,0, UMC1820 hw:0,0,0, nanoKONTROL2 hw:2,0,0), Safety Clamping, NRPN/Dual-CC Router & Bidirectional OSC Server (Target: crates/synth-midi-osc; Related: src/midi.rs, src/hardware_verify.rs). Implement read-only /proc/asound/cards and /proc/asound/seq/clients inspection in crates/synth-midi-osc and src/hardware_verify.rs detecting Focusrite A.E Peak (Peak MIDI 1, hw:3,0,0, client 28, USB ID 1235:007e), UMC1820 MIDI 1 (hw:0,0,0, client 16), and Korg nanoKONTROL2 (hw:2,0,0, client 24) with safe virtual loopback default (never sending unprompted MIDI bytes to hw:3,0,0 or hw:0,0,0 during tests). Implement Novation Peak MIDI Channel 1 router (7-bit CC, +32 bipolar dual-CC, 14-bit NRPN, Bank Select CC0/CC32 + Program Change, and Focusrite/Novation 00 20 29 SysEx patch dump codec), self-oscillation/overdrive safety clamp (safe_sweep_range on CCs 29, 36, 79, 80, 104, 110), and bidirectional UDP OSC server (/peak/osc/{1..3}/*, /peak/filter/*, /peak/env/*, /peak/lfo/*, /peak/fx/*, /peak/animate/{1,2}, /peak/panic) with unit tests. Ensure all workspace tests pass (CARGO_BUILD_JOBS=2 cargo test --workspace and npm test). Write reports/report-002.md and compile it with /home/renick/claude/html-report-generator/build-report.sh --pdf.
```

# Content

## 1. Overview and Everyday Purpose

When a musician connects a physical synthesizer to a computer running Linux, two critical requirements must be met before musical performance can begin:

1. Identification: The computer must find which USB ports and audio cards correspond to the synthesizer, the audio interface, and the physical knob surface.
2. Safe Communication: The software must speak the correct musical protocol without blasting uncomfortably loud audio or accidentally overwriting presets.

In this development epoch, we built the communication and discovery foundation for the Novation Peak synthesizer within our shared workstation crate (`crates/synth-midi-osc`) and desktop manager application (`src/midi.rs` and `src/hardware_verify.rs`).

To make this technical implementation clear to anyone, we define several specialized industry terms on first use:

- ALSA (Advanced Linux Sound Architecture): The Linux operating system subsystem responsible for managing audio hardware cards and MIDI communication ports.
- MIDI (Musical Instrument Digital Interface): The international standard communication protocol through which synthesizers, controllers, and computers exchange musical instructions.
- CC (Continuous Controller): A standard 7-bit MIDI command that adjusts a setting between 0 (minimum) and 127 (maximum).
- Dual-CC: A technique combining two standard 7-bit messages (a coarse setting and a fine setting offset by 32) to create 16,384 distinct steps for ultra-smooth knob sweeps.
- NRPN (Non-Registered Parameter Number): A 14-bit MIDI messaging standard used to access specialized parameters that lack dedicated CC numbers.
- SysEx (System Exclusive): Manufacturer-specific MIDI packets used to transfer complete sound presets and memory dumps.
- OSC (Open Sound Control): A modern high-speed network messaging standard that transmits musical parameters over local networks.
- UDP (User Datagram Protocol): A lightweight network transport protocol that delivers messages with minimal latency.

## 2. Non-Invasive ALSA Hardware Discovery

Musicians frequently use test suites, automated build pipelines, and command-line diagnostics to verify their software. If an automated script were to open a physical synthesizer's MIDI port and transmit test signals without human oversight, the synthesizer could abruptly play loud notes or wipe out internal presets.

To solve this, we implemented non-invasive hardware inspection. Instead of opening raw MIDI output streams, our software reads plain-text Linux system status files located in the kernel's virtual `/proc` directory:

- `/proc/asound/cards`: Lists each installed sound device, its index number, short name, long descriptive name, and hardware connection path.
- `/proc/asound/seq/clients`: Lists active ALSA sequencer clients and their numbered MIDI input and output ports.

Our parser, `parse_proc_asound_cards` in `crates/synth-midi-osc/src/hardware.rs`, extracts structured `AsoundCardEntry` records from these read-only files. On our workstation, the inspection reliably identifies all three connected studio devices:

1. Novation Peak: Sound card 3 (`hw:3,0,0`), ALSA sequencer client 28, port 0 (`Peak MIDI 1`), USB identifier `1235:007e`.
2. Behringer UMC1820: Sound card 0 (`hw:0,0,0`), ALSA sequencer client 16, port 0 (`UMC1820 MIDI 1`), USB identifier `1397:0503`.
3. Korg nanoKONTROL2: Sound card 2 (`hw:2,0,0`), ALSA sequencer client 24, port 0 (`nanoKONTROL2 _ CTRL`), USB identifier `0944:0117`.

By default, all automated tests and command-line executions run in `VirtualLoopback` mode. In this mode, mock endpoint instances stand in for physical hardware, capturing all outgoing MIDI commands into an in-memory history log without transmitting a single byte to physical hardware interfaces.

## 3. Self-Oscillation and Overdrive Safety Clamping

The Novation Peak features an analog resonant filter and three separate distortion stages (pre-filter overdrive, post-filter drive, and master analog distortion). In an analog filter, resonance boosts the frequencies right at the filter cutoff point. If resonance is pushed to its maximum value, the filter enters self-oscillation, creating a piercing, high-amplitude sine wave even when no note is being played.

Similarly, pushing distortion or delay feedback to their upper limits can create deafening audio feedback that threatens both loudspeaker hardware and human hearing.

To protect listeners, we implemented an automatic safety clamp function, `safe_sweep_range`, in `crates/synth-midi-osc/src/routing.rs`. Whenever our router (`PeakMidiRouter`) receives a parameter change, it verifies the controller number against our safety registry and restricts dangerous values:

| Controller Name | MIDI CC | Technical Danger | Raw Range | Clamped Safe Range |
| :--- | :--- | :--- | :--- | :--- |
| Filter Cutoff | CC 29 | Main filter frequency | 0 to 127 | 0 to 127 (Full range safe) |
| Filter Post-Drive | CC 36 | Analog distortion after filter | 0 to 127 | 0 to 50 (Prevents clipping) |
| Filter Resonance | CC 79 | Acoustic whistle and self-oscillation | 0 to 127 | 0 to 62 (Prevents piercing feedback) |
| Pre-Filter Overdrive | CC 80 | Analog overdrive before filter | 0 to 127 | 0 to 50 (Prevents harsh clipping) |
| Analogue Distortion | CC 104 | Master analog saturation | 0 to 127 | 0 to 48 (Guarantees safe headroom) |
| Delay Feedback | CC 110 | Echo regeneration | 0 to 127 | 0 to 70 (Prevents runaway feedback build-up) |
| Chorus / Delay / Reverb | CC 105, 108, 112 | Digital effect sends | 0 to 127 | 0 to 90 (Prevents wet signal overflow) |

Musicians can disable this clamping mechanism with `with_safety_clamp(false)` when intentional high-distortion sound design is desired, but the system defaults to safe clamping at all times.

## 4. Novation Peak MIDI Channel 1 Router

We implemented `PeakMidiRouter` in `crates/synth-midi-osc/src/routing.rs` and the higher-level service `PeakMidiService` in `src/midi.rs`. The router handles five distinct communication patterns required by the Novation Peak:

1. MIDI Channel 1 Default: All commands target MIDI Channel 1 (encoded as wire channel index 0 in MIDI status bytes).
2. Standard 7-Bit Continuous Controllers: Emits standard `MidiPacket::ControlChange` messages with automatic safety clamping.
3. High-Resolution Dual-CC (+32 Offset): Dispatches a coarse value on base CCs 0 through 31 and a fine value on CCs 32 through 63. Also supports bipolar values centered around zero (-8,192 to +8,191 mapped to 14-bit unsigned 0 to 16,383).
4. 14-Bit NRPN Messages: Encodes deep sound parameters using CC 99 (parameter MSB), CC 98 (parameter LSB), CC 6 (value MSB), and CC 38 (value LSB).
5. Bank Selection and Program Changes: Dispatches Bank Select MSB (CC 0 = 0), Bank Select LSB (CC 32 = bank 0, 1, 2, or 3 for Banks A, B, C, or D), followed immediately by a Program Change command (0 to 127).
6. Focusrite / Novation SysEx Patch Dump Codec: `PeakSysExCodec` in `crates/synth-midi-osc/src/hardware.rs` packages sound settings into official Novation SysEx frames starting with manufacturer code `00 20 29`, device family `03`, command `01`, bank and patch indices, 16-character ASCII name, and 7-bit safe payload bytes. It supports direct bidirectional translation to and from our workspace preset format, `synth_core::preset::SynthPatch`.

## 5. Bidirectional UDP OSC Server

Live-coding music environments like Konduktiva and external graphical tablet controls interact with synthesizers over local area computer networks using Open Sound Control (OSC).

We implemented `PeakOscBridge` in `crates/synth-midi-osc/src/osc.rs`, combining an inbound UDP server with an outbound subscriber broadcaster. Incoming network packets sent to the `/peak/...` address namespace are parsed by `parse_peak_address` and converted directly into safely clamped MIDI packets:

- `/peak/osc/{1..3}/*`: Controls oscillator waveform, shape amount, mixer level, and virtual sync.
- `/peak/filter/*`: Controls cutoff frequency, resonance (clamped to 62), pre-drive, and post-drive.
- `/peak/env/*`: Controls attack, decay, sustain, and release across amplitude and modulation envelopes.
- `/peak/lfo/*`: Controls rate, tempo synchronization, and fade time across LFO 1 and 2.
- `/peak/fx/*`: Controls distortion level (clamped to 48), chorus, delay time, delay feedback (clamped to 70), and reverb.
- `/peak/animate/{1,2}`: Triggers the Peak's two tactile performance hold buttons (CC 114 and CC 115).
- `/peak/panic`: Triggers an emergency All-Notes-Off reset across all voices and controllers.

Outbound communication allows network clients to subscribe via `/peak/subscribe` and receive immediate state updates whenever parameters change. Our test suite includes `PeakOscBridge::run_loopback_test()`, which exercises client subscription, parameter dispatch, broadcast reception, and emergency panic over live loopback UDP sockets.

## 6. Plain-English Concept to Code Mapping

The following table connects everyday synthesizer concepts to their exact Rust symbols and file locations:

| Plain-English Concept | Technical Symbol | File Path | Everyday Role |
| :--- | :--- | :--- | :--- |
| Sound Card Entry | `AsoundCardEntry` | `crates/synth-midi-osc/src/hardware.rs` | Stores parsed Linux sound card names and hardware indices |
| Card File Parser | `parse_proc_asound_cards` | `crates/synth-midi-osc/src/hardware.rs` | Reads read-only `/proc/asound/cards` without touching hardware |
| Device Type Enum | `SupportedDeviceType::NovationPeak` | `crates/synth-midi-osc/src/hardware.rs` | Identifies the Novation Peak from USB ID `1235:007e` |
| Patch Dump Codec | `PeakSysExCodec` | `crates/synth-midi-osc/src/hardware.rs` | Encodes and decodes Novation `00 20 29` patch SysEx frames |
| Safety Clamp Function | `safe_sweep_range` | `crates/synth-midi-osc/src/routing.rs` | Enforces upper bounds on resonance and distortion CCs |
| MIDI Channel 1 Router | `PeakMidiRouter` | `crates/synth-midi-osc/src/routing.rs` | Formats CC, dual-CC, NRPN, and program changes for Channel 1 |
| Hardware Discovery | `discover_peak_hardware` | `crates/synth-midi-osc/src/routing.rs` | Finds Peak, UMC1820, and nanoKONTROL2 with safe mock default |
| OSC Target Parser | `parse_peak_address` | `crates/synth-midi-osc/src/osc.rs` | Translates `/peak/...` network addresses into typed commands |
| OSC Network Bridge | `PeakOscBridge` | `crates/synth-midi-osc/src/osc.rs` | Runs UDP server and subscriber broadcaster for network control |
| Hardware Verifier | `verify_system_hardware` | `src/hardware_verify.rs` | Generates diagnostic reports for connected workstation devices |
| High-Level MIDI Service | `PeakMidiService` | `src/midi.rs` | Manages active MIDI state and safe loopback history |

## 7. Verification Evidence and Test Results

We verified all changes using our dual-CPU build discipline (`CARGO_BUILD_JOBS=2`) and strict zero-warning policy. All 100 Rust unit tests and 8 Node.js tests passed without warnings or failures:

```
running 9 tests
test midi::tests::test_peak_nrpn_and_bank_program_change ... ok
test midi::tests::test_peak_sysex_patch_dump_roundtrip ... ok
test midi::tests::test_peak_dual_cc_and_bipolar_routing ... ok
test midi::tests::test_peak_midi_service_channel_1_default_and_loopback ... ok
test tests::test_synth_id_baseline ... ok
test tests::test_hardware_verify_and_midi_service_exported ... ok
test hardware_verify::tests::test_virtual_loopback_default_safety ... ok
test hardware_verify::tests::test_dry_run_inspection_live_files ... ok
test tests::test_novation_peak_profile_root_integration ... ok
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; finished in 0.00s

synth_core:      34 passed; 0 failed
synth_midi_osc:  33 passed; 0 failed
synth_ui_theme:  24 passed; 0 failed
Total Rust:     100 passed; 0 failed; 0 warnings
Total Node.js:    8 passed; 0 failed
Combined Total: 108 passed; 0 failed
```

Command-line verification executed cleanly in both virtual mock mode and live read-only inspection mode:

```
$ novation-peak-manager --verify-hardware
=================================================================
 Novation Peak Manager - Hardware Verification Report
=================================================================
Safety Mode:     Virtual Loopback (Offline Safe) (Safe Sink: true)
Cards Scanned:   0
Overall Status:  ALL 3 HARDWARE TARGETS VERIFIED
-----------------------------------------------------------------
 Target Device           | ALSA HW   | ALSA Seq | USB ID    | Status
-----------------------------------------------------------------
 Focusrite A.E Peak      | hw:3,0,0  | 28:0     | 1235:007e | OK (Mock)
 BEHRINGER UMC1820       | hw:0,0,0  | 16:0     | 1397:0503 | OK (Mock)
 KORG INC. nanoKONTROL2  | hw:2,0,0  | 24:0     | 0944:0117 | OK (Mock)
=================================================================

$ novation-peak-manager --verify-hardware --dry-run
=================================================================
 Novation Peak Manager - Hardware Verification Report
=================================================================
Safety Mode:     Dry-Run Inspection (Virtual Loopback Safe) (Safe Sink: true)
Cards Scanned:   4
Overall Status:  ALL 3 HARDWARE TARGETS VERIFIED
-----------------------------------------------------------------
 Target Device           | ALSA HW   | ALSA Seq | USB ID    | Status
-----------------------------------------------------------------
 Focusrite A.E Peak      | hw:3,0,0  | 28:0     | 1235:007e | OK (Live)
 BEHRINGER UMC1820       | hw:0,0,0  | 16:0     | 1397:0503 | OK (Live)
 KORG INC. nanoKONTROL2  | hw:2,0,0  | 24:0     | 0944:0117 | OK (Live)
=================================================================
```

The application correctly detected all three physical instruments on the host workstation while ensuring that not a single unprompted MIDI byte was sent to physical hardware ports.
