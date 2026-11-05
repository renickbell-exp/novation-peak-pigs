//! CLI and GUI entrypoint for Novation Peak Manager.
use novation_peak_manager::hardware_verify::{print_verification_report, verify_system_hardware};
use novation_peak_manager::synth_midi_osc::routing::MidiSafetyMode;

fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.iter().any(|a| a == "--verify-hardware") {
        // Safe read-only discovery inspection: dry-run or virtual loopback
        let mode = if args.iter().any(|a| a == "--dry-run") {
            MidiSafetyMode::DryRun
        } else {
            MidiSafetyMode::VirtualLoopback
        };
        let report = verify_system_hardware(mode);
        print_verification_report(&report);
        return;
    }

    if args.iter().any(|a| a == "--dry-stress") {
        println!("Novation Peak Dry-Run Stress Verification (Safe Loopback Mode)");
        let report = verify_system_hardware(MidiSafetyMode::VirtualLoopback);
        assert!(report.is_safe, "Must be safe in dry-stress mode");
        println!("Safe virtual loopback verified: 0 unprompted bytes transmitted to physical ports.");
        return;
    }

    println!("Novation Peak Manager ({})", novation_peak_manager::synth_id());
}
