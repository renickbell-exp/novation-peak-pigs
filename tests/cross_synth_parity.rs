// SPDX-License-Identifier: GPL-3.0-or-later
//! `tests/cross_synth_parity.rs`: Cross-language integration and parity tests
//! between Rust (`novation_peak_manager`, `synth_core`, `synth_midi_osc`) and
//! Node.js (`konduktiva-peak`, `shared-synth-sdk`).
//!
//! Verifies:
//! - Byte-identical 74 CC mappings and 32 bipolar dual-CC pairs
//! - 14-bit NRPNs for wavetable selection, filter type, and filter slope
//! - Safe sweep range (`safeSweepRange` / `safe_sweep_range`) clamping limits
//! - OSC address patterns and parameter dispatching
//! - Novation SysEx frame structure, checksum calculation, and 0x7F bypass
//! - Preset JSON roundtrip parity against standard schema
//! - 8 canonical Peak players and density graph rates

use novation_peak_manager::konduktiva_bridge::{
    tri, DensityGraphMode, KonduktivaBridge, CC_PLAYER_NAMES, DEFAULT_SCALE_POOL,
    NOTE_PLAYER_NAMES, PEAK_PLAYER_NAMES,
};
use novation_peak_manager::peak_librarian::{
    calculate_peak_checksum, decode_single_patch_sysex, encode_single_patch_sysex,
    verify_peak_checksum, NOVATION_MANUFACTURER_ID, PEAK_SYSEX_CMD_SOUND_DUMP,
    PEAK_SYSEX_DEVICE_ID, UNIVERSAL_BYPASS_CHECKSUM,
};
use novation_peak_manager::peak_synth::PeakSynthEngine;
use serde_json::Value;
use std::fs;
use std::path::Path;
use synth_core::preset::SynthPatch;
use synth_core::profiles::NovationPeakProfile;
use synth_midi_osc::osc::{parse_peak_address, PeakOscBridge, PeakOscTarget};
use synth_midi_osc::routing::{
    safe_sweep_range, safe_sweep_range_by_name, MidiPacket, MidiSafetyMode, PeakMidiRouter,
};

#[test]
fn test_cc_map_json_74_entries_parity() {
    let json_path = Path::new("js/konduktiva-peak/midi-cc-map.json");
    let content = fs::read_to_string(json_path).expect("read midi-cc-map.json");
    let parsed: Value = serde_json::from_str(&content).expect("parse json");

    let cc_map = parsed["cc_map"].as_array().expect("cc_map array");
    assert_eq!(cc_map.len(), 74, "Peak CC map must contain exactly 74 CC entries");

    // Verify key musical controllers match both Rust and JS definitions
    let mut map_by_param: std::collections::HashMap<String, u8> = std::collections::HashMap::new();
    for entry in cc_map {
        let cc = entry["cc"].as_u64().expect("cc number") as u8;
        let param = entry["param"].as_str().expect("param name").to_lowercase();
        map_by_param.insert(param, cc);
    }

    assert_eq!(map_by_param.get("filter cutoff (frequency)"), Some(&29));
    assert_eq!(map_by_param.get("filter resonance"), Some(&79));
    assert_eq!(map_by_param.get("mod wheel (modulation)"), Some(&1));
    assert_eq!(map_by_param.get("reverb level"), Some(&112));
    assert_eq!(map_by_param.get("delay level"), Some(&108));
    assert_eq!(map_by_param.get("distortion level"), Some(&104));
    assert_eq!(map_by_param.get("filter overdrive"), Some(&80));
    assert_eq!(map_by_param.get("filter post drive"), Some(&36));
    assert_eq!(map_by_param.get("delay feedback"), Some(&110));
    assert_eq!(map_by_param.get("chorus level"), Some(&105));
    assert_eq!(map_by_param.get("animate 1 hold"), Some(&114));
    assert_eq!(map_by_param.get("animate 2 hold"), Some(&115));
    assert_eq!(map_by_param.get("arp clock gate"), Some(&116));

    // Profile verification
    let profile = NovationPeakProfile::device_profile();
    assert_eq!(profile.id, "novation_peak");
    assert_eq!(profile.name, "Novation Peak");

    for entry in cc_map {
        let cc = entry["cc"].as_u64().unwrap() as u8;
        let found = profile.parameters.iter().any(|p| match p.address {
            synth_core::param::ParamAddress::Cc7(param_cc) => param_cc == cc,
            _ => false,
        });
        assert!(found, "Profile must cover CC {}", cc);
    }
}

#[test]
fn test_32_bipolar_dual_cc_pairs_parity() {
    let profile = NovationPeakProfile::device_profile();

    // Verify 32 dual pairs (MSB 0..31, LSB 32..63)
    for msb in 0..32u8 {
        let lsb = msb + 32;
        let found = profile.parameters.iter().any(|p| match p.address {
            synth_core::param::ParamAddress::Cc14Bit {
                msb: p_msb,
                lsb: p_lsb,
            } => p_msb == msb && p_lsb == lsb,
            _ => false,
        });
        assert!(found, "Profile must contain 14-bit dual CC pair ({}, {})", msb, lsb);
    }
}

#[test]
fn test_14bit_nrpn_parity() {
    let profile = NovationPeakProfile::device_profile();

    // Filter Type: LSB 48
    assert!(profile.parameters.iter().any(|p| match p.address {
        synth_core::param::ParamAddress::Nrpn { lsb, .. } => lsb == 48,
        _ => false,
    }));

    // Filter Slope: LSB 49
    assert!(profile.parameters.iter().any(|p| match p.address {
        synth_core::param::ParamAddress::Nrpn { lsb, .. } => lsb == 49,
        _ => false,
    }));

    // Osc 1..3 Waveform NRPNs: 64, 80, 96
    for &lsb_target in &[64u8, 80u8, 96u8] {
        assert!(profile.parameters.iter().any(|p| match p.address {
            synth_core::param::ParamAddress::Nrpn { lsb, .. } => lsb == lsb_target,
            _ => false,
        }));
    }

    // Router NRPN packet generation (14-bit mode)
    let router = PeakMidiRouter::new().with_safety_mode(MidiSafetyMode::VirtualLoopback);
    let packets = router.route_nrpn(48, 1, true);
    assert_eq!(packets.len(), 4);
    assert_eq!(packets[0], MidiPacket::ControlChange { channel: 0, cc: 99, value: 0 }); // NRPN MSB
    assert_eq!(packets[1], MidiPacket::ControlChange { channel: 0, cc: 98, value: 48 }); // NRPN LSB
    assert_eq!(packets[2], MidiPacket::ControlChange { channel: 0, cc: 6, value: 0 }); // Data MSB
    assert_eq!(packets[3], MidiPacket::ControlChange { channel: 0, cc: 38, value: 1 }); // Data LSB
}

#[test]
fn test_safe_sweep_range_clamping_parity() {
    // Both Rust and Node.js must enforce identical safe ceilings
    // Resonance (CC 79): max 62
    assert_eq!(safe_sweep_range(79), (0, 62));
    assert_eq!(safe_sweep_range_by_name("filter_resonance"), (0, 62));
    assert_eq!(safe_sweep_range_by_name("reso"), (0, 62));

    // Distortion (CC 104): max 48
    assert_eq!(safe_sweep_range(104), (0, 48));
    assert_eq!(safe_sweep_range_by_name("distortion_level"), (0, 48));

    // Filter Overdrive (CC 80) and Post-Drive (CC 36): max 50
    assert_eq!(safe_sweep_range(80), (0, 50));
    assert_eq!(safe_sweep_range(36), (0, 50));
    assert_eq!(safe_sweep_range_by_name("filter_overdrive"), (0, 50));
    assert_eq!(safe_sweep_range_by_name("filter_post_drive"), (0, 50));

    // Delay Feedback (CC 110): max 70
    assert_eq!(safe_sweep_range(110), (0, 70));
    assert_eq!(safe_sweep_range_by_name("delay_feedback"), (0, 70));

    // FX Levels (Reverb 112, Delay 108, Chorus 105): max 90
    assert_eq!(safe_sweep_range(112), (0, 90));
    assert_eq!(safe_sweep_range(108), (0, 90));
    assert_eq!(safe_sweep_range(105), (0, 90));
    assert_eq!(safe_sweep_range_by_name("reverb_level"), (0, 90));
    assert_eq!(safe_sweep_range_by_name("delay_level"), (0, 90));
    assert_eq!(safe_sweep_range_by_name("chorus_level"), (0, 90));

    // Filter Cutoff (CC 29): full 0..127 range
    assert_eq!(safe_sweep_range(29), (0, 127));
    assert_eq!(safe_sweep_range_by_name("filter_cutoff"), (0, 127));
}

#[test]
fn test_osc_address_parsing_and_dispatch_parity() {
    // 1. Osc parsing
    assert_eq!(
        parse_peak_address("/peak/osc/1/shape"),
        Some(PeakOscTarget::Osc {
            osc_idx: 1,
            param: "shape".to_string()
        })
    );

    // 2. Filter parsing
    assert_eq!(
        parse_peak_address("/peak/filter/cutoff"),
        Some(PeakOscTarget::Filter {
            param: "cutoff".to_string()
        })
    );

    // 3. Env parsing
    assert_eq!(
        parse_peak_address("/peak/env/amp/attack"),
        Some(PeakOscTarget::Env {
            env_name: "amp".to_string(),
            param: "attack".to_string()
        })
    );

    // 4. LFO parsing
    assert_eq!(
        parse_peak_address("/peak/lfo/2/rate"),
        Some(PeakOscTarget::Lfo {
            lfo_idx: 2,
            param: "rate".to_string()
        })
    );

    // 5. FX parsing
    assert_eq!(
        parse_peak_address("/peak/fx/reverb/level"),
        Some(PeakOscTarget::Fx {
            fx_name: "reverb".to_string(),
            param: "level".to_string()
        })
    );

    // 6. Animate and Panic
    assert_eq!(
        parse_peak_address("/peak/animate/1"),
        Some(PeakOscTarget::Animate { button_idx: 1 })
    );
    assert_eq!(parse_peak_address("/peak/panic"), Some(PeakOscTarget::Panic));

    // 7. Safety clamping during dispatch
    let router = PeakMidiRouter::new()
        .with_safety_mode(MidiSafetyMode::VirtualLoopback)
        .with_safety_clamp(true);
    let bridge = PeakOscBridge::new_ephemeral(router).expect("create ephemeral bridge");

    let incoming = synth_midi_osc::osc::OscIncomingMessage {
        sender: "127.0.0.1:9999".parse().unwrap(),
        address: "/peak/filter/resonance".to_string(),
        args: vec![rosc::OscType::Float(1.0)], // Requests 127
    };

    let packets = bridge.dispatch_incoming(&incoming);
    assert_eq!(packets.len(), 1);
    assert_eq!(
        packets[0],
        MidiPacket::ControlChange {
            channel: 0,
            cc: 79,
            value: 62, // Clamped to 62 by safe_sweep_range
        }
    );
}

#[test]
fn test_sysex_codec_and_checksum_parity() {
    let mut patch = SynthPatch::new("Parity Test");
    patch.set_param("filter_cutoff", 100.0);
    patch.set_param("filter_resonance", 50.0);
    let frame = encode_single_patch_sysex(&patch).expect("encode sysex");

    // Header validation
    assert_eq!(frame[0], 0xF0);
    assert_eq!(&frame[1..4], &NOVATION_MANUFACTURER_ID); // [0x00, 0x20, 0x29]
    assert_eq!(frame[4], PEAK_SYSEX_DEVICE_ID); // 0x00
    assert_eq!(frame[5], PEAK_SYSEX_CMD_SOUND_DUMP); // 0x00
    assert_eq!(*frame.last().unwrap(), 0xF7);

    // Checksum verification
    let payload = &frame[24..frame.len() - 2];
    let computed_chk = calculate_peak_checksum(payload);
    let frame_chk = frame[frame.len() - 2];
    assert_eq!(computed_chk, frame_chk);
    assert!(verify_peak_checksum(payload, frame_chk));
    assert!(verify_peak_checksum(payload, UNIVERSAL_BYPASS_CHECKSUM)); // 0x7F bypass

    // Decode roundtrip
    let decoded = decode_single_patch_sysex(&frame).expect("decode sysex");
    assert_eq!(decoded.meta.name, "Parity Test");
}

#[test]
fn test_preset_json_roundtrip_parity() {
    let mut patch = SynthPatch::new("Cross-Language Init");
    patch.meta.category = "Lead".to_string();
    patch.meta.bank = "A".to_string();
    patch.meta.slot = 42;
    patch.set_param("filter_cutoff", 88.0);
    patch.set_param("filter_resonance", 44.0);
    patch.set_param("osc_1_shape", 64.0);

    // Serialize to JSON
    let json_str = serde_json::to_string_pretty(&patch).expect("serialize patch to json");

    // Parse as generic JSON Value to assert schema compliance
    let value: Value = serde_json::from_str(&json_str).expect("parse json");
    assert_eq!(value["meta"]["name"].as_str(), Some("Cross-Language Init"));
    assert_eq!(value["meta"]["category"].as_str(), Some("Lead"));
    assert_eq!(value["meta"]["bank"].as_str(), Some("A"));
    assert_eq!(value["meta"]["slot"].as_u64(), Some(42));
    assert_eq!(value["parameters"]["filter_cutoff"].as_f64(), Some(88.0));
    assert_eq!(value["parameters"]["filter_resonance"].as_f64(), Some(44.0));
    assert_eq!(value["parameters"]["osc_1_shape"].as_f64(), Some(64.0));

    // Deserialize back into SynthPatch
    let restored: SynthPatch = serde_json::from_str(&json_str).expect("deserialize patch");
    assert_eq!(restored.meta.name, patch.meta.name);
    assert_eq!(restored.meta.category, patch.meta.category);
    assert_eq!(restored.meta.slot, patch.meta.slot);
    assert_eq!(restored.get_param("filter_cutoff"), Some(88.0));
    assert_eq!(restored.get_param("filter_resonance"), Some(44.0));
}

#[test]
fn test_canonical_8_players_and_rates_parity() {
    let bridge = KonduktivaBridge::new();
    assert_eq!(bridge.players.len(), 8);

    let names: Vec<&str> = bridge.players.iter().map(|p| p.name.as_str()).collect();
    assert_eq!(names, PEAK_PLAYER_NAMES);
    assert_eq!(NOTE_PLAYER_NAMES, ["bass", "chords", "lead", "arp"]);
    assert_eq!(CC_PLAYER_NAMES, ["ccCutoff", "ccReso", "ccMod", "ccFx"]);
    assert_eq!(DEFAULT_SCALE_POOL, [0, 2, 3, 5, 7, 8, 10, 12]);

    // Check exact rates
    assert_eq!(bridge.players[0].native_rate, 0.5); // bass
    assert_eq!(bridge.players[1].native_rate, 0.5); // chords
    assert_eq!(bridge.players[2].native_rate, 2.0); // lead
    assert_eq!(bridge.players[3].native_rate, 4.0); // arp
    assert_eq!(bridge.players[4].native_rate, 4.0); // ccCutoff
    assert_eq!(bridge.players[5].native_rate, 2.0); // ccReso
    assert_eq!(bridge.players[6].native_rate, 2.0); // ccMod
    assert_eq!(bridge.players[7].native_rate, 1.0); // ccFx

    // Check Density Graph multiplier parity
    let hold = DensityGraphMode::PeakHold;
    assert_eq!(hold.rate_multiplier(0.0), 1.0);
    assert_eq!(hold.rate_multiplier(8.0), 1.0);

    let build = DensityGraphMode::PeakBuild;
    assert_eq!(build.rate_multiplier(0.0), 1.0);
    assert_eq!(build.rate_multiplier(8.0), 2.0);
    assert_eq!(build.rate_multiplier(16.0), 1.0);

    // Triangle sweep calculation parity
    assert_eq!(tri(0, 16), 0.0);
    assert_eq!(tri(8, 16), 1.0);
    assert_eq!(tri(16, 16), 0.0);
}

#[test]
fn test_engine_integration_with_konduktiva_events() {
    let mut bridge = KonduktivaBridge::new();
    let mut engine = PeakSynthEngine::new();

    bridge.start();
    let events = bridge.step_beats(4.0);
    assert!(!events.is_empty());

    for evt in events {
        match evt {
            MidiPacket::NoteOn { note, velocity, .. } => {
                engine.note_on(note, velocity);
            }
            MidiPacket::NoteOff { note, .. } => {
                engine.note_off(note);
            }
            MidiPacket::ControlChange { cc, value, .. } => {
                let _ = engine.set_cc(cc, value);
            }
            _ => {}
        }
    }

    // Engine rendered sample must be finite and within safe bounds
    let sample = engine.render_sample();
    assert!(sample.is_finite());
    assert!(sample >= -1.0 && sample <= 1.0);
}
