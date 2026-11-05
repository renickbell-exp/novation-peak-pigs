//! Novation Peak 8-Voice Hybrid Synthesizer Manager, Librarian & Konduktiva Companion.
pub use synth_core;
pub use synth_midi_osc;
pub use synth_ui_theme;

pub use synth_core::profiles::{NovationPeakProfile, UnifiedEcosystemSchemaBundle};

pub mod hardware_verify;
pub mod midi;
pub mod peak_mod_fx;
pub mod peak_synth;

pub use hardware_verify::{
    format_report, print_verification_report, run_cli_verification, verify_hardware_from_paths,
    verify_system_hardware, HardwareVerificationReport, VerifiedHardwareDevice,
};
pub use midi::PeakMidiService;
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
}
