//! Novation Peak 8-Voice Hybrid Synthesizer Manager, Librarian & Konduktiva Companion.
pub use synth_core;
pub use synth_midi_osc;
pub use synth_ui_theme;

pub use synth_core::profiles::{NovationPeakProfile, UnifiedEcosystemSchemaBundle};

pub mod hardware_verify;
pub mod midi;
pub mod peak_synth;

pub use hardware_verify::{
    format_report, print_verification_report, run_cli_verification, verify_hardware_from_paths,
    verify_system_hardware, HardwareVerificationReport, VerifiedHardwareDevice,
};
pub use midi::PeakMidiService;
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
}
