//! Novation Peak 8-Voice Hybrid Synthesizer Manager, Librarian & Konduktiva Companion.
pub use synth_core;
pub use synth_midi_osc;
pub use synth_ui_theme;

pub use synth_core::profiles::{NovationPeakProfile, UnifiedEcosystemSchemaBundle};

pub mod app;
pub mod gui;
pub mod hardware_verify;
pub mod konduktiva_bridge;
pub mod midi;
pub mod peak_librarian;
pub mod peak_mod_fx;
pub mod peak_synth;

pub use app::{
    GeometryAuditReport, HardwareStatusBarState, LayoutGeometryAuditor, NovationPeakApp,
};
pub use hardware_verify::{
    format_report, print_verification_report, run_cli_verification, verify_hardware_from_paths,
    verify_system_hardware, HardwareVerificationReport, VerifiedHardwareDevice,
};
pub use konduktiva_bridge::*;
pub use midi::PeakMidiService;
pub use peak_librarian::*;
pub use peak_mod_fx::*;
pub use peak_synth::*;

/// Returns the canonical synthesizer identifier for this application.
pub fn synth_id() -> &'static str {
    "novation-peak"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_synth_id_baseline() {
        assert_eq!(synth_id(), "novation-peak");
    }

    #[test]
    fn test_novation_peak_profile_root_integration() {
        let profile = NovationPeakProfile::device_profile();
        assert_eq!(profile.id, "novation_peak");
        assert_eq!(profile.name, "Novation Peak");

        let bundle = UnifiedEcosystemSchemaBundle::new();
        assert_eq!(bundle.synthesizer_profiles().len(), 5);
        assert!(bundle.get_profile("novation_peak").is_some());
    }

    #[test]
    fn test_hardware_verify_and_midi_service_exported() {
        let report = verify_system_hardware(synth_midi_osc::routing::MidiSafetyMode::VirtualLoopback);
        assert!(report.is_safe);
        assert!(report.all_detected);

        let mut service = PeakMidiService::default();
        let pkt = service.send_cc(29, 85);
        assert_eq!(
            pkt,
            synth_midi_osc::routing::MidiPacket::ControlChange {
                channel: 0,
                cc: 29,
                value: 85,
            }
        );
    }

    #[test]
    fn test_peak_synth_domain_engine_root_exports() {
        let mut engine = PeakSynthEngine::new();
        assert_eq!(engine.active_voice_count(), 0);
        assert_eq!(engine.voices.len(), 8);

        // Test parameter mutation with undo/redo
        engine.set_cc(29, 75).expect("set filter cutoff");
        assert_eq!(engine.get_param("filter_cutoff"), 75.0);
        assert!(engine.can_undo());

        engine.undo().expect("undo");
        assert_eq!(engine.get_param("filter_cutoff"), 100.0);

        // Test wavetable & harmonic spectrum calculation
        let spectrum = calculate_harmonic_spectrum(0, 64.0);
        assert_eq!(spectrum.harmonics.len(), 32);
        assert!(spectrum.spectral_centroid() >= 1.0);

        // Test filter response curve calculation
        let curve = engine.calculate_filter_response_curve(32);
        assert_eq!(curve.len(), 32);

        // Test voice allocation and rendering
        engine.note_on(60, 90);
        assert_eq!(engine.active_voice_count(), 1);
        let sample = engine.render_sample();
        assert!(sample >= -1.0 && sample <= 1.0);
    }

    #[test]
    fn test_peak_mod_fx_engine_root_exports() {
        // 1. Test ADSR curve generation
        let curve = generate_adsr_curve(5.0, 64.0, 127.0, 32.0, 32);
        assert_eq!(curve.len(), 32);

        // 2. Test LFO processing
        let mut lfo = LfoProcessor::new(LfoWaveform::Triangle, 48000.0);
        let lfo_val = lfo.step(0.001, 120.0);
        assert!(lfo_val >= -1.0 && lfo_val <= 1.0);

        // 3. Test 16-Slot Mod Matrix & 4-Slot FX Matrix
        let mut mod_matrix = ModMatrix::new();
        mod_matrix.slots[0] = ModMatrixSlot {
            source_a: ModSource::Animate1,
            source_b: ModSource::Direct,
            destination: ModDestination::FilterCutoff,
            depth: 64.0,
        };
        let mut fx_matrix = FxModMatrix::default();
        fx_matrix.slots[0] = FxModMatrixSlot {
            source: ModSource::Animate2,
            destination: FxModDestination::ReverbLevel,
            depth: 32.0,
        };

        let patch = create_default_peak_patch();
        let (param_previews, fx_previews) = calculate_animate_offsets(
            &mod_matrix,
            &fx_matrix,
            &patch,
            true,
            true,
        );
        assert_eq!(param_previews.len(), 1);
        assert_eq!(param_previews[0].destination, ModDestination::FilterCutoff);
        assert_eq!(fx_previews.len(), 1);

        // 4. Test Arpeggiator and Glide
        let mut arp = ArpEngine::default();
        arp.enabled = true;
        arp.note_on(60);
        let mut got_arp_event = false;
        for _ in 0..20 {
            if arp.step(0.01, 120.0).is_some() {
                got_arp_event = true;
                break;
            }
        }
        assert!(got_arp_event, "Arpeggiator should emit note event within sequence");

        let mut glide = GlideProcessor::default();
        glide.enabled = true;
        glide.glide_time_cc = 50.0;
        glide.current_semitone = 60.0;
        glide.set_target_note(72);
        assert!(glide.step(0.02) > 60.0);

        // 5. Test Master FX Chain
        let mut fx = MasterFxChain::new(48000.0);
        fx.distortion.level_cc = 50.0;
        fx.chorus.level_cc = 60.0;
        fx.delay.level_cc = 40.0;
        fx.reverb.level_cc = 70.0;
        let (l, r) = fx.process(0.4, 0.4);
        assert!(l >= -1.2 && l <= 1.2);
        assert!(r >= -1.2 && r <= 1.2);
    }

    #[test]
    fn test_peak_librarian_domain_root_exports() {
        // 1. Curated Patches & 512-Patch Bank Manager
        let patches = create_curated_peak_patches();
        assert_eq!(patches.len(), 8);

        let mut librarian = PeakLibrarian::new();
        assert_eq!(librarian.library.len(), 512);
        assert_eq!(librarian.get_patch(0, 0).map(|p| p.meta.name.as_str()), Some("Init Patch"));
        assert_eq!(librarian.get_patch(0, 7).map(|p| p.meta.name.as_str()), Some("Animate Morph Arp"));

        let (msb, lsb, pc) = librarian.select_patch(1, 10); // Bank B, Slot 10
        assert_eq!(msb, synth_midi_osc::routing::MidiPacket::ControlChange { channel: 0, cc: 0, value: 0 });
        assert_eq!(lsb, synth_midi_osc::routing::MidiPacket::ControlChange { channel: 0, cc: 32, value: 1 });
        assert_eq!(pc, synth_midi_osc::routing::MidiPacket::ProgramChange { channel: 0, program: 10 });

        // 2. SysEx dump codec and checksum
        let sysex_frame = encode_single_patch_sysex(&patches[0]).expect("encode patch sysex");
        let decoded_patch = decode_single_patch_sysex(&sysex_frame).expect("decode patch sysex");
        assert_eq!(decoded_patch.meta.name, "Init Patch");

        let mut state_machine = PeakSysExStateMachine::new();
        let events = state_machine.feed_buffer(&sysex_frame);
        assert_eq!(events.len(), 1);
        assert!(matches!(events[0], SysExEvent::PatchReceived(_)));

        // 3. Continuous A..B patch morphing
        let mut morph = ContinuousMorphFader::new(patches[0].clone(), patches[1].clone());
        let deltas = morph.set_factor(0.5);
        assert!(!deltas.is_empty());
        assert!((morph.factor() - 0.5).abs() < f64::EPSILON);

        // 4. Safe randomizer with automatic safety caps on CCs 36, 79, 80, 104, 110
        let randomizer = PeakSafeRandomizer::new();
        let mut dangerous = SynthPatch::new("Extreme Danger");
        dangerous.set_param("filter_post_drive", 127.0);
        dangerous.set_param("filter_resonance", 127.0);
        dangerous.set_param("filter_overdrive", 127.0);
        dangerous.set_param("distortion_level", 127.0);
        dangerous.set_param("delay_feedback", 127.0);

        let safe = randomizer.randomize_global(&dangerous, 100.0, 100.0, Some(777));
        assert!(safe.get_param("filter_post_drive").unwrap() <= SAFE_CAP_FILTER_POST_DRIVE);
        assert!(safe.get_param("filter_resonance").unwrap() <= SAFE_CAP_FILTER_RESONANCE);
        assert!(safe.get_param("filter_overdrive").unwrap() <= SAFE_CAP_FILTER_OVERDRIVE);
        assert!(safe.get_param("distortion_level").unwrap() <= SAFE_CAP_DISTORTION_LEVEL);
        assert!(safe.get_param("delay_feedback").unwrap() <= SAFE_CAP_DELAY_FEEDBACK);

        // 5. Korg nanoKONTROL2 Surface
        let mut surface = NanoKontrol2Surface::new(SoftTakeoverMode::Jump);
        let action = surface.process_midi(0, 100).expect("fader 1");
        assert!(matches!(action, NanoSurfaceAction::ParamChange { is_slider: true, target_cc: 29, .. }));

        let anim = surface.process_midi(32, 127).expect("solo 1 animate 1");
        assert!(matches!(anim, NanoSurfaceAction::AnimateTrigger { index: 1, active: true, .. }));
    }

    #[test]
    fn test_konduktiva_bridge_root_exports() {
        let mut bridge = KonduktivaBridge::new();
        assert_eq!(bridge.players.len(), 8);
        assert_eq!(bridge.cc_telemetry.len(), 4);

        bridge.start();
        assert!(bridge.playing);

        let events = bridge.step_beats(4.0);
        assert!(!events.is_empty());

        bridge.stop();
        assert!(!bridge.playing);

        let panic_events = bridge.panic();
        assert_eq!(panic_events.len(), 3);
    }
}
