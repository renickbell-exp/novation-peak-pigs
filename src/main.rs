//! CLI and GUI entrypoint for Novation Peak Manager.
use novation_peak_manager::hardware_verify::{print_verification_report, verify_system_hardware};
use novation_peak_manager::synth_midi_osc::routing::MidiSafetyMode;
use novation_peak_manager::{LayoutGeometryAuditor, NovationPeakApp};

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

    if args.iter().any(|a| a == "--audit-geometry") {
        println!("Running headless layout geometry audit...");
        let mut app = NovationPeakApp::new();
        let r800 = LayoutGeometryAuditor::audit_at_resolution(&mut app, 1280.0, 800.0);
        let r1080 = LayoutGeometryAuditor::audit_at_resolution(&mut app, 1920.0, 1080.0);
        println!(
            "1280x800: passed={}, panels={}, overlap={}",
            r800.passed, r800.panels_audited, r800.max_overlap_area
        );
        println!(
            "1920x1080: passed={}, panels={}, overlap={}",
            r1080.passed, r1080.panels_audited, r1080.max_overlap_area
        );
        assert!(r800.passed && r1080.passed);
        return;
    }

    if args.iter().any(|a| a == "--gui") {
        let native_options = eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default()
                .with_inner_size([1280.0, 800.0])
                .with_min_inner_size([960.0, 600.0])
                .with_title("Novation Peak Manager"),
            ..Default::default()
        };
        let _ = eframe::run_native(
            "Novation Peak Manager",
            native_options,
            Box::new(|_cc| Ok(Box::new(NovationPeakApp::new()))),
        );
        return;
    }

    println!("Novation Peak Manager ({})", novation_peak_manager::synth_id());
}
