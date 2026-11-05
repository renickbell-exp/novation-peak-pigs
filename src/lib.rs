//! Novation Peak 8-Voice Hybrid Synthesizer Manager, Librarian & Konduktiva Companion.
pub use synth_core;
pub use synth_midi_osc;
pub use synth_ui_theme;

pub use synth_core::profiles::{NovationPeakProfile, UnifiedEcosystemSchemaBundle};

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
}
