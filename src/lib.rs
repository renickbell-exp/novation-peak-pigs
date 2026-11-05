//! Novation Peak 8-Voice Hybrid Synthesizer Manager, Librarian & Konduktiva Companion.
pub use synth_core;
pub use synth_midi_osc;
pub use synth_ui_theme;

pub use synth_core::profiles::{NovationPeakProfile, UnifiedEcosystemSchemaBundle};

pub mod hardware_verify;
pub mod midi;

pub use hardware_verify::{
    format_report, print_verification_report, run_cli_verification, verify_hardware_from_paths,
    verify_system_hardware, HardwareVerificationReport, VerifiedHardwareDevice,
};
pub use midi::PeakMidiService;

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
}
