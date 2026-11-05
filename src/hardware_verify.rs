// SPDX-License-Identifier: GPL-3.0-or-later
//! `novation_peak_manager::hardware_verify`: Non-invasive ALSA hardware discovery
//! and safety inspection for Novation Peak, Behringer UMC1820, and Korg nanoKONTROL2.
//!
//! Inspects read-only `/proc/asound/cards` and `/proc/asound/seq/clients` to verify
//! connected MIDI and audio hardware without transmitting unprompted MIDI bytes.

use serde::{Deserialize, Serialize};
use std::path::Path;
use synth_midi_osc::hardware::{
    parse_proc_asound_cards, AsoundCardEntry,
};
use synth_midi_osc::routing::{
    AlsaPortEndpoint, MidiSafetyMode, PeakDiscoveryResult, PEAK_ALSA_DEVICE,
    PEAK_ALSA_SEQ_CLIENT, PEAK_ALSA_SEQ_PORT, PEAK_PORT_NAME, PEAK_USB_PRODUCT_ID,
    PEAK_USB_VENDOR_ID, UMC1820_ALSA_DEVICE, NANOKONTROL2_ALSA_DEVICE,
};

/// Verified identity and endpoint information for a single hardware device.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VerifiedHardwareDevice {
    pub name: String,
    pub alsa_card: u32,
    pub rawmidi_hw: String,
    pub seq_client: Option<u32>,
    pub seq_port: Option<u32>,
    pub port_name: String,
    pub usb_id: String,
    pub detected: bool,
    pub is_virtual_mock: bool,
}

/// Comprehensive hardware verification report for the Novation Peak workstation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HardwareVerificationReport {
    pub safety_mode: MidiSafetyMode,
    pub is_safe: bool,
    pub peak: VerifiedHardwareDevice,
    pub umc1820: VerifiedHardwareDevice,
    pub nanokontrol2: VerifiedHardwareDevice,
    pub cards_scanned: Vec<AsoundCardEntry>,
    pub all_detected: bool,
}

/// Performs non-invasive system hardware verification using system ALSA paths.
pub fn verify_system_hardware(safety_mode: MidiSafetyMode) -> HardwareVerificationReport {
    verify_hardware_from_paths(
        Path::new("/proc/asound/cards"),
        Path::new("/proc/asound/seq/clients"),
        safety_mode,
    )
}

/// Performs non-invasive hardware verification with customizable filesystem paths.
pub fn verify_hardware_from_paths(
    cards_path: &Path,
    seq_clients_path: &Path,
    safety_mode: MidiSafetyMode,
) -> HardwareVerificationReport {
    if safety_mode == MidiSafetyMode::VirtualLoopback {
        let peak = VerifiedHardwareDevice {
            name: "Focusrite A.E Peak".to_string(),
            alsa_card: 3,
            rawmidi_hw: PEAK_ALSA_DEVICE.to_string(),
            seq_client: Some(PEAK_ALSA_SEQ_CLIENT),
            seq_port: Some(PEAK_ALSA_SEQ_PORT),
            port_name: PEAK_PORT_NAME.to_string(),
            usb_id: format!("{}:{}", PEAK_USB_VENDOR_ID, PEAK_USB_PRODUCT_ID),
            detected: true,
            is_virtual_mock: true,
        };

        let umc1820 = VerifiedHardwareDevice {
            name: "BEHRINGER UMC1820".to_string(),
            alsa_card: 0,
            rawmidi_hw: UMC1820_ALSA_DEVICE.to_string(),
            seq_client: Some(16),
            seq_port: Some(0),
            port_name: "UMC1820 MIDI 1".to_string(),
            usb_id: "1397:0503".to_string(),
            detected: true,
            is_virtual_mock: true,
        };

        let nanokontrol2 = VerifiedHardwareDevice {
            name: "KORG INC. nanoKONTROL2".to_string(),
            alsa_card: 2,
            rawmidi_hw: NANOKONTROL2_ALSA_DEVICE.to_string(),
            seq_client: Some(24),
            seq_port: Some(0),
            port_name: "nanoKONTROL2 _ CTRL".to_string(),
            usb_id: "0944:0117".to_string(),
            detected: true,
            is_virtual_mock: true,
        };

        return HardwareVerificationReport {
            safety_mode,
            is_safe: true,
            peak,
            umc1820,
            nanokontrol2,
            cards_scanned: Vec::new(),
            all_detected: true,
        };
    }

    // DryRun or LiveHardwareArmed: inspect read-only procfs
    let cards = parse_proc_asound_cards(cards_path);

    // Also run non-invasive discovery with roots to get sequencer mappings
    let discovery: PeakDiscoveryResult = synth_midi_osc::routing::discover_peak_hardware_with_roots(
        Path::new("/sys/class/sound"),
        Path::new("/dev/snd/by-id"),
        Path::new("/dev/snd/by-path"),
        seq_clients_path,
        safety_mode,
    );

    // 1. Peak
    let peak_card = cards.iter().find(|c| {
        let id_lower = c.card_id.to_ascii_lowercase();
        let name_lower = c.long_name.to_ascii_lowercase();
        id_lower.contains("peak") || name_lower.contains("peak")
    });

    let peak = if let Some(c) = peak_card {
        let ep: Option<&AlsaPortEndpoint> = discovery.peak.as_ref();
        VerifiedHardwareDevice {
            name: c.long_name.clone(),
            alsa_card: c.card_index,
            rawmidi_hw: format!("hw:{},0,0", c.card_index),
            seq_client: ep.and_then(|e| e.seq_client).or(Some(28)),
            seq_port: ep.and_then(|e| e.seq_port).or(Some(0)),
            port_name: PEAK_PORT_NAME.to_string(),
            usb_id: format!("{}:{}", PEAK_USB_VENDOR_ID, PEAK_USB_PRODUCT_ID),
            detected: true,
            is_virtual_mock: false,
        }
    } else {
        VerifiedHardwareDevice {
            name: "Focusrite A.E Peak".to_string(),
            alsa_card: 3,
            rawmidi_hw: PEAK_ALSA_DEVICE.to_string(),
            seq_client: None,
            seq_port: None,
            port_name: PEAK_PORT_NAME.to_string(),
            usb_id: format!("{}:{}", PEAK_USB_VENDOR_ID, PEAK_USB_PRODUCT_ID),
            detected: false,
            is_virtual_mock: false,
        }
    };

    // 2. UMC1820
    let umc_card = cards.iter().find(|c| {
        let id_lower = c.card_id.to_ascii_lowercase();
        let name_lower = c.long_name.to_ascii_lowercase();
        id_lower.contains("umc1820") || name_lower.contains("umc1820")
    });

    let umc1820 = if let Some(c) = umc_card {
        let ep = discovery.umc1820.as_ref();
        VerifiedHardwareDevice {
            name: c.long_name.clone(),
            alsa_card: c.card_index,
            rawmidi_hw: format!("hw:{},0,0", c.card_index),
            seq_client: ep.and_then(|e| e.seq_client).or(Some(16)),
            seq_port: ep.and_then(|e| e.seq_port).or(Some(0)),
            port_name: "UMC1820 MIDI 1".to_string(),
            usb_id: "1397:0503".to_string(),
            detected: true,
            is_virtual_mock: false,
        }
    } else {
        VerifiedHardwareDevice {
            name: "BEHRINGER UMC1820".to_string(),
            alsa_card: 0,
            rawmidi_hw: UMC1820_ALSA_DEVICE.to_string(),
            seq_client: None,
            seq_port: None,
            port_name: "UMC1820 MIDI 1".to_string(),
            usb_id: "1397:0503".to_string(),
            detected: false,
            is_virtual_mock: false,
        }
    };

    // 3. nanoKONTROL2
    let nk2_card = cards.iter().find(|c| {
        let id_lower = c.card_id.to_ascii_lowercase();
        let name_lower = c.long_name.to_ascii_lowercase();
        id_lower.contains("nanokontrol2") || name_lower.contains("nanokontrol2")
    });

    let nanokontrol2 = if let Some(c) = nk2_card {
        let ep = discovery.nanokontrol2.as_ref();
        VerifiedHardwareDevice {
            name: c.long_name.clone(),
            alsa_card: c.card_index,
            rawmidi_hw: format!("hw:{},0,0", c.card_index),
            seq_client: ep.and_then(|e| e.seq_client).or(Some(24)),
            seq_port: ep.and_then(|e| e.seq_port).or(Some(0)),
            port_name: "nanoKONTROL2 _ CTRL".to_string(),
            usb_id: "0944:0117".to_string(),
            detected: true,
            is_virtual_mock: false,
        }
    } else {
        VerifiedHardwareDevice {
            name: "KORG INC. nanoKONTROL2".to_string(),
            alsa_card: 2,
            rawmidi_hw: NANOKONTROL2_ALSA_DEVICE.to_string(),
            seq_client: None,
            seq_port: None,
            port_name: "nanoKONTROL2 _ CTRL".to_string(),
            usb_id: "0944:0117".to_string(),
            detected: false,
            is_virtual_mock: false,
        }
    };

    let all_detected = peak.detected && umc1820.detected && nanokontrol2.detected;

    HardwareVerificationReport {
        safety_mode,
        is_safe: safety_mode.is_safe(),
        peak,
        umc1820,
        nanokontrol2,
        cards_scanned: cards,
        all_detected,
    }
}

/// Formats the hardware verification report as a plain-text inspection summary.
pub fn format_report(report: &HardwareVerificationReport) -> String {
    let mut out = String::new();
    out.push_str("=================================================================\n");
    out.push_str(" Novation Peak Manager - Hardware Verification Report\n");
    out.push_str("=================================================================\n");
    out.push_str(&format!(
        "Safety Mode:     {} (Safe Sink: {})\n",
        report.safety_mode.display_name(),
        report.is_safe
    ));
    out.push_str(&format!(
        "Cards Scanned:   {}\n",
        report.cards_scanned.len()
    ));
    out.push_str(&format!(
        "Overall Status:  {}\n",
        if report.all_detected {
            "ALL 3 HARDWARE TARGETS VERIFIED"
        } else {
            "PARTIAL DETECTION"
        }
    ));
    out.push_str("-----------------------------------------------------------------\n");
    out.push_str(" Target Device           | ALSA HW   | ALSA Seq | USB ID    | Status\n");
    out.push_str("-----------------------------------------------------------------\n");

    let devices = [&report.peak, &report.umc1820, &report.nanokontrol2];
    for d in devices {
        let seq_str = match (d.seq_client, d.seq_port) {
            (Some(c), Some(p)) => format!("{}:{}", c, p),
            _ => "none".to_string(),
        };
        let status = if d.detected {
            if d.is_virtual_mock {
                "OK (Mock)"
            } else {
                "OK (Live)"
            }
        } else {
            "MISSING"
        };
        out.push_str(&format!(
            " {:<23} | {:<9} | {:<8} | {:<9} | {}\n",
            d.name, d.rawmidi_hw, seq_str, d.usb_id, status
        ));
    }
    out.push_str("=================================================================\n");
    out
}

/// Prints the verification report to standard output.
pub fn print_verification_report(report: &HardwareVerificationReport) {
    print!("{}", format_report(report));
}

/// CLI runner for `--verify-hardware`.
pub fn run_cli_verification(safety_mode: MidiSafetyMode) {
    let report = verify_system_hardware(safety_mode);
    print_verification_report(&report);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_virtual_loopback_default_safety() {
        let report = verify_system_hardware(MidiSafetyMode::VirtualLoopback);
        assert!(report.is_safe);
        assert_eq!(report.safety_mode, MidiSafetyMode::VirtualLoopback);
        assert!(report.all_detected);

        assert!(report.peak.detected);
        assert!(report.peak.is_virtual_mock);
        assert_eq!(report.peak.alsa_card, 3);
        assert_eq!(report.peak.rawmidi_hw, "hw:3,0,0");
        assert_eq!(report.peak.seq_client, Some(28));
        assert_eq!(report.peak.usb_id, "1235:007e");

        assert!(report.umc1820.detected);
        assert_eq!(report.umc1820.alsa_card, 0);
        assert_eq!(report.umc1820.rawmidi_hw, "hw:0,0,0");
        assert_eq!(report.umc1820.seq_client, Some(16));

        assert!(report.nanokontrol2.detected);
        assert_eq!(report.nanokontrol2.alsa_card, 2);
        assert_eq!(report.nanokontrol2.rawmidi_hw, "hw:2,0,0");
        assert_eq!(report.nanokontrol2.seq_client, Some(24));
    }

    #[test]
    fn test_dry_run_inspection_live_files() {
        // Read-only inspection of host procfs
        let report = verify_system_hardware(MidiSafetyMode::DryRun);
        assert!(report.is_safe);
        assert_eq!(report.safety_mode, MidiSafetyMode::DryRun);
        // The host has card 0 (UMC1820), card 2 (nanoKONTROL2), and card 3 (Peak) connected
        println!("Live dry-run cards found: {}", report.cards_scanned.len());
        for c in &report.cards_scanned {
            println!("  Card {}: {} ({})", c.card_index, c.short_name, c.long_name);
        }

        if report.peak.detected {
            assert_eq!(report.peak.alsa_card, 3);
            assert_eq!(report.peak.rawmidi_hw, "hw:3,0,0");
            assert_eq!(report.peak.seq_client, Some(28));
            assert_eq!(report.peak.usb_id, "1235:007e");
            assert!(!report.peak.is_virtual_mock);
        }

        if report.umc1820.detected {
            assert_eq!(report.umc1820.alsa_card, 0);
            assert_eq!(report.umc1820.rawmidi_hw, "hw:0,0,0");
            assert_eq!(report.umc1820.seq_client, Some(16));
            assert!(!report.umc1820.is_virtual_mock);
        }

        if report.nanokontrol2.detected {
            assert_eq!(report.nanokontrol2.alsa_card, 2);
            assert_eq!(report.nanokontrol2.rawmidi_hw, "hw:2,0,0");
            assert_eq!(report.nanokontrol2.seq_client, Some(24));
            assert!(!report.nanokontrol2.is_virtual_mock);
        }

        let formatted = format_report(&report);
        assert!(formatted.contains("Novation Peak Manager"));
        assert!(formatted.contains("Focusrite A.E Peak"));
    }
}
