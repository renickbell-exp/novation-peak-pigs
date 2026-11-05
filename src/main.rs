//! CLI and GUI entrypoint for Novation Peak Manager.
use novation_peak_manager::hardware_verify::{
    export_shared_schemas, print_verification_report, verify_system_hardware,
};
use novation_peak_manager::peak_librarian::PeakLibrarian;
use novation_peak_manager::stress_bench::{run_stress_benchmark, StressBenchmarkConfig};
use novation_peak_manager::synth_midi_osc::osc::OscServer;
use novation_peak_manager::synth_midi_osc::routing::MidiSafetyMode;
use novation_peak_manager::{LayoutGeometryAuditor, NovationPeakApp};
use std::path::PathBuf;

#[derive(Debug, Default)]
struct CliOptions {
    verify_hardware: bool,
    dry_stress: bool,
    export_shared_schemas: bool,
    schema_dir: Option<PathBuf>,
    synth: Option<String>,
    midi_in: Option<String>,
    midi_out: Option<String>,
    channel: Option<u8>,
    preset: Option<String>,
    headless_server: bool,
    port: Option<u16>,
    audit_geometry: bool,
    gui: bool,
    dry_run: bool,
    help: bool,
}

fn parse_cli_args(args: &[String]) -> CliOptions {
    let mut opts = CliOptions::default();
    let mut i = 1;

    while i < args.len() {
        let arg = &args[i];
        match arg.as_str() {
            "--verify-hardware" => opts.verify_hardware = true,
            "--dry-stress" => opts.dry_stress = true,
            "--dry-run" => opts.dry_run = true,
            "--audit-geometry" => opts.audit_geometry = true,
            "--gui" => opts.gui = true,
            "--headless-server" => opts.headless_server = true,
            "-h" | "--help" => opts.help = true,
            "--export-shared-schemas" => {
                opts.export_shared_schemas = true;
                if i + 1 < args.len() && !args[i + 1].starts_with("--") {
                    i += 1;
                    opts.schema_dir = Some(PathBuf::from(&args[i]));
                }
            }
            "--synth" => {
                if i + 1 < args.len() {
                    i += 1;
                    opts.synth = Some(args[i].clone());
                }
            }
            "--midi-in" => {
                if i + 1 < args.len() {
                    i += 1;
                    opts.midi_in = Some(args[i].clone());
                }
            }
            "--midi-out" => {
                if i + 1 < args.len() {
                    i += 1;
                    opts.midi_out = Some(args[i].clone());
                }
            }
            "--channel" => {
                if i + 1 < args.len() {
                    i += 1;
                    if let Ok(ch) = args[i].parse::<u8>() {
                        opts.channel = Some(ch.clamp(1, 16));
                    }
                }
            }
            "--preset" => {
                if i + 1 < args.len() {
                    i += 1;
                    opts.preset = Some(args[i].clone());
                }
            }
            "--port" => {
                if i + 1 < args.len() {
                    i += 1;
                    if let Ok(p) = args[i].parse::<u16>() {
                        opts.port = Some(p);
                    }
                }
            }
            _ => {}
        }
        i += 1;
    }

    opts
}

fn print_help() {
    println!("Novation Peak Manager ({})", novation_peak_manager::synth_id());
    println!("Usage: novation-peak-manager [OPTIONS]");
    println!();
    println!("Options:");
    println!("  --verify-hardware          Non-invasive inspection of Peak, UMC1820, and nanoKONTROL2");
    println!("  --dry-stress               1,000-event polyphonic + 9-lane CC stress benchmark");
    println!("  --export-shared-schemas [DIR] Export JSON schemas and profiles across 5 synths");
    println!("  --synth <NAME>             Target synthesizer model (default: novation-peak)");
    println!("  --midi-in <DEVICE>         MIDI Input port/ALSA endpoint");
    println!("  --midi-out <DEVICE>        MIDI Output port/ALSA endpoint");
    println!("  --channel <1-16>           MIDI Channel (default: 1)");
    println!("  --preset <NAME_OR_INDEX>   Inspect preset metadata from library");
    println!("  --headless-server          Start non-blocking headless OSC and MIDI service");
    println!("  --port <PORT>              OSC listen port (default: 9100)");
    println!("  --audit-geometry           Headless layout geometry audit (1280x800 & 1920x1080)");
    println!("  --gui                      Launch native egui desktop application");
    println!("  --dry-run                  Force read-only inspection mode");
    println!("  -h, --help                 Print this help message");
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let opts = parse_cli_args(&args);

    if opts.help {
        print_help();
        return;
    }

    if opts.export_shared_schemas {
        let target_dir = opts.schema_dir.unwrap_or_else(|| PathBuf::from("schemas"));
        println!("Exporting portable schemas to {}...", target_dir.display());
        match export_shared_schemas(&target_dir) {
            Ok(files) => {
                println!("Successfully exported {} schemas and profiles:", files.len());
                for f in &files {
                    println!("  - {}", f.display());
                }
            }
            Err(e) => {
                eprintln!("Failed to export schemas: {}", e);
                std::process::exit(1);
            }
        }
        return;
    }

    if opts.dry_stress {
        println!("Running Novation Peak 1,000-event polyphonic + 9-lane CC stress benchmark...");
        let config = StressBenchmarkConfig::default();
        let report = run_stress_benchmark(&config);
        report.print_report();
        if !report.passed {
            eprintln!("Dry stress benchmark failed invariants");
            std::process::exit(1);
        }
        return;
    }

    if opts.verify_hardware {
        let mode = if opts.dry_run {
            MidiSafetyMode::DryRun
        } else {
            MidiSafetyMode::VirtualLoopback
        };
        let report = verify_system_hardware(mode);
        print_verification_report(&report);
        return;
    }

    if let Some(query) = opts.preset {
        let librarian = PeakLibrarian::new();
        println!("Querying Novation Peak preset library for '{}'...", query);

        let patch_match = if let Ok(idx) = query.parse::<usize>() {
            let bank = (idx / 128).min(3) as u8;
            let slot = (idx % 128) as u8;
            librarian.get_patch(bank, slot).cloned()
        } else {
            librarian.library.search(&query).first().copied().cloned()
        };

        match patch_match {
            Some(patch) => {
                println!("Preset Found:");
                println!("  Name:      {}", patch.meta.name);
                println!("  Category:  {}", patch.meta.category);
                println!("  Bank/Slot: Bank {} Slot {}", patch.meta.bank, patch.meta.slot);
                println!("  Cutoff:    {}", patch.get_param("filter_cutoff").unwrap_or(100.0));
                println!("  Resonance: {}", patch.get_param("filter_resonance").unwrap_or(20.0));
                println!("  Post-Drive:{}", patch.get_param("filter_post_drive").unwrap_or(0.0));
            }
            None => {
                println!("Preset '{}' not found in 512-patch library.", query);
            }
        }
        return;
    }

    if opts.headless_server {
        let port = opts.port.unwrap_or(9100);
        let channel = opts.channel.unwrap_or(1);
        let synth_name = opts.synth.as_deref().unwrap_or("novation-peak");

        println!("Initializing Novation Peak Headless OSC Server on 127.0.0.1:{}...", port);
        match OscServer::bind("127.0.0.1", port) {
            Ok(server) => {
                let fallback_addr = std::net::SocketAddr::new(
                    std::net::IpAddr::V4(std::net::Ipv4Addr::LOCALHOST),
                    0,
                );
                let local_addr = server.local_addr().unwrap_or(fallback_addr);
                println!("Novation Peak Headless Server listening on {}", local_addr);
                println!("  Synth:    {}", synth_name);
                println!("  Channel:  {} (Wire index: {})", channel, channel - 1);
                println!("  Safety:   VirtualLoopback (0 unprompted bytes transmitted)");

                // Non-blocking poll pass
                let pending = server.poll_incoming().unwrap_or_default();
                println!("  Polled:   {} queued OSC messages processed.", pending.len());
                println!("Novation Peak Headless Server ready.");
            }
            Err(e) => {
                eprintln!("Failed to bind OSC server on port {}: {}", port, e);
                std::process::exit(1);
            }
        }
        return;
    }

    if opts.audit_geometry {
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
        if !r800.passed || !r1080.passed {
            eprintln!("Layout geometry audit failed");
            std::process::exit(1);
        }
        return;
    }

    if opts.gui {
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

    // Default: print info and configured parameters if passed
    let synth = opts.synth.as_deref().unwrap_or("novation-peak");
    let channel = opts.channel.unwrap_or(1);
    println!("Novation Peak Manager ({})", synth);
    println!("  Channel: {}", channel);
    if let Some(in_dev) = opts.midi_in {
        println!("  MIDI In: {}", in_dev);
    }
    if let Some(out_dev) = opts.midi_out {
        println!("  MIDI Out: {}", out_dev);
    }
    println!("Use --help for available CLI options or --gui to launch desktop interface.");
}
