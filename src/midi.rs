// SPDX-License-Identifier: GPL-3.0-or-later
//! `novation_peak_manager::midi`: High-level Novation Peak MIDI service,
//! Channel 1 routing, safety clamping, +32 bipolar dual-CC, NRPN, and SysEx dump codec.

use synth_core::preset::SynthPatch;
pub use synth_midi_osc::hardware::{MidiCodecError, PeakPatchDump, PeakSysExCodec};
pub use synth_midi_osc::routing::{
    clamp_cc_value, safe_sweep_range, safe_sweep_range_by_name,
    MidiPacket, MidiSafetyMode, PeakMidiRouter,
};

/// High-level MIDI service for the Novation Peak polyphonic synthesizer.
#[derive(Debug, Clone)]
pub struct PeakMidiService {
    pub router: PeakMidiRouter,
    loopback_history: Vec<MidiPacket>,
}

impl Default for PeakMidiService {
    fn default() -> Self {
        Self::new(MidiSafetyMode::VirtualLoopback)
    }
}

impl PeakMidiService {
    /// Creates a new Peak MIDI service defaulting to safe Virtual Loopback mode.
    #[must_use]
    pub fn new(safety_mode: MidiSafetyMode) -> Self {
        Self {
            router: PeakMidiRouter::new().with_safety_mode(safety_mode),
            loopback_history: Vec::new(),
        }
    }

    /// Sets the operational safety mode.
    #[must_use]
    pub fn with_safety_mode(mut self, mode: MidiSafetyMode) -> Self {
        self.router = self.router.with_safety_mode(mode);
        self
    }

    /// Configures whether self-oscillation and overdrive safety clamping is active.
    #[must_use]
    pub fn with_safety_clamp(mut self, enabled: bool) -> Self {
        self.router = self.router.with_safety_clamp(enabled);
        self
    }

    /// Sends a 7-bit Continuous Controller message on Channel 1.
    ///
    /// Applies safety clamping on CCs 29, 36, 79, 80, 104, 110 when enabled.
    pub fn send_cc(&mut self, cc: u8, value: u8) -> MidiPacket {
        let pkt = self.router.route_cc(cc, value);
        if self.router.safety_mode.is_safe() {
            self.loopback_history.push(pkt.clone());
        }
        pkt
    }

    /// Sends a +32 high-resolution 14-bit dual-CC pair `(MSB, LSB)`.
    pub fn send_dual_cc(&mut self, msb_cc: u8, value14: u16) -> (MidiPacket, MidiPacket) {
        let (msb, lsb) = self.router.route_dual_cc(msb_cc, value14);
        if self.router.safety_mode.is_safe() {
            self.loopback_history.push(msb.clone());
            self.loopback_history.push(lsb.clone());
        }
        (msb, lsb)
    }

    /// Sends a +32 bipolar dual-CC pair centered at 0 `[-8192..8191]`.
    pub fn send_bipolar_dual_cc(&mut self, msb_cc: u8, bipolar: i16) -> (MidiPacket, MidiPacket) {
        let (msb, lsb) = self.router.route_bipolar_dual_cc(msb_cc, bipolar);
        if self.router.safety_mode.is_safe() {
            self.loopback_history.push(msb.clone());
            self.loopback_history.push(lsb.clone());
        }
        (msb, lsb)
    }

    /// Sends a 14-bit or 7-bit Non-Registered Parameter Number (NRPN) sequence.
    pub fn send_nrpn(&mut self, param: u16, value: u16, is_14bit: bool) -> Vec<MidiPacket> {
        let pkts = self.router.route_nrpn(param, value, is_14bit);
        if self.router.safety_mode.is_safe() {
            self.loopback_history.extend(pkts.clone());
        }
        pkts
    }

    /// Sends Bank Select (CC0 MSB = 0, CC32 LSB = bank 0..3) followed by Program Change (0..127).
    pub fn send_bank_program(&mut self, bank: u8, program: u8) -> Vec<MidiPacket> {
        let pkts = self.router.route_bank_program(bank, program);
        if self.router.safety_mode.is_safe() {
            self.loopback_history.extend(pkts.clone());
        }
        pkts
    }

    /// Sends emergency All-Notes-Off panic sequence for Channel 1.
    pub fn panic(&mut self) -> Vec<MidiPacket> {
        let pkts = self.router.route_panic();
        if self.router.safety_mode.is_safe() {
            self.loopback_history.extend(pkts.clone());
        }
        pkts
    }

    /// Encodes a `SynthPatch` into a standard Novation Peak SysEx dump frame (`00 20 29`).
    pub fn encode_patch_dump(&self, patch: &SynthPatch) -> Result<Vec<u8>, MidiCodecError> {
        PeakSysExCodec::encode_synth_patch(patch)
    }

    /// Decodes a raw Novation Peak SysEx dump frame (`00 20 29`) into a `SynthPatch`.
    pub fn decode_patch_dump(&self, bytes: &[u8]) -> Result<SynthPatch, MidiCodecError> {
        PeakSysExCodec::decode_synth_patch(bytes)
    }

    /// In-memory loopback sink log of transmitted MIDI packets.
    #[must_use]
    pub fn loopback_history(&self) -> &[MidiPacket] {
        &self.loopback_history
    }

    /// Clears the in-memory loopback sink log.
    pub fn clear_history(&mut self) {
        self.loopback_history.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_peak_midi_service_channel_1_default_and_loopback() {
        let mut service = PeakMidiService::new(MidiSafetyMode::VirtualLoopback);
        assert_eq!(service.router.channel, 0); // MIDI Channel 1
        assert!(service.router.safety_mode.is_safe());
        assert_eq!(service.loopback_history().len(), 0);

        // 7-bit CC with safety clamping: Resonance clamped to 62
        let pkt = service.send_cc(79, 120);
        assert_eq!(
            pkt,
            MidiPacket::ControlChange {
                channel: 0,
                cc: 79,
                value: 62,
            }
        );
        assert_eq!(service.loopback_history().len(), 1);

        // Post-drive clamped to 50
        let pkt2 = service.send_cc(36, 90);
        assert_eq!(
            pkt2,
            MidiPacket::ControlChange {
                channel: 0,
                cc: 36,
                value: 50,
            }
        );
        assert_eq!(service.loopback_history().len(), 2);
    }

    #[test]
    fn test_peak_dual_cc_and_bipolar_routing() {
        let mut service = PeakMidiService::new(MidiSafetyMode::VirtualLoopback);

        // Dual-CC cutoff (CC 29 MSB + CC 61 LSB)
        let (msb, lsb) = service.send_dual_cc(29, 8192);
        assert_eq!(
            msb,
            MidiPacket::ControlChange {
                channel: 0,
                cc: 29,
                value: 64,
            }
        );
        assert_eq!(
            lsb,
            MidiPacket::ControlChange {
                channel: 0,
                cc: 61,
                value: 0,
            }
        );

        // Bipolar dual-CC (0 center -> 8192)
        let (b_msb, b_lsb) = service.send_bipolar_dual_cc(29, 0);
        assert_eq!(
            b_msb,
            MidiPacket::ControlChange {
                channel: 0,
                cc: 29,
                value: 64,
            }
        );
        assert_eq!(
            b_lsb,
            MidiPacket::ControlChange {
                channel: 0,
                cc: 61,
                value: 0,
            }
        );
    }

    #[test]
    fn test_peak_nrpn_and_bank_program_change() {
        let mut service = PeakMidiService::new(MidiSafetyMode::VirtualLoopback);

        let nrpn = service.send_nrpn(200, 14000, true);
        assert_eq!(nrpn.len(), 4);

        // Bank Select CC0 / CC32 + Program Change
        let bp = service.send_bank_program(2, 63); // Bank C, Patch 63
        assert_eq!(bp.len(), 3);
        assert_eq!(
            bp[0],
            MidiPacket::ControlChange {
                channel: 0,
                cc: 0,
                value: 0,
            }
        );
        assert_eq!(
            bp[1],
            MidiPacket::ControlChange {
                channel: 0,
                cc: 32,
                value: 2,
            }
        );
        assert_eq!(
            bp[2],
            MidiPacket::ProgramChange {
                channel: 0,
                program: 63,
            }
        );
    }

    #[test]
    fn test_peak_sysex_patch_dump_roundtrip() {
        let service = PeakMidiService::new(MidiSafetyMode::VirtualLoopback);

        let mut patch = SynthPatch::new("Oxford Pad");
        patch.meta.bank = "Bank B".to_string();
        patch.meta.slot = 24;
        patch.parameters.insert("filter_cutoff".to_string(), 90.0);

        let dump_bytes = service.encode_patch_dump(&patch).expect("SysEx encode failed");
        assert_eq!(dump_bytes[0], 0xF0);
        assert_eq!(&dump_bytes[1..4], &[0x00, 0x20, 0x29]); // Focusrite/Novation ID
        assert_eq!(*dump_bytes.last().unwrap(), 0xF7);

        let restored = service.decode_patch_dump(&dump_bytes).expect("SysEx decode failed");
        assert_eq!(restored.meta.name, "Oxford Pad");
        assert_eq!(restored.meta.bank, "Bank B");
        assert_eq!(restored.meta.slot, 24);
    }
}
