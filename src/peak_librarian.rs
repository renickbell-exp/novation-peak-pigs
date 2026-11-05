// SPDX-License-Identifier: GPL-3.0-or-later
//! `novation_peak_manager::peak_librarian`: Complete Novation Peak patch librarian,
//! 512-patch bank and SysEx manager (Banks A..D), A/B patch diffing and selective merge,
//! continuous A..B patch morphing, lock-shielded safe randomizer with automatic safety caps,
//! and Korg nanoKONTROL2 (hw:2,0,0) 8-column physical surface controller.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub use synth_core::{
    diff::{selective_merge, selective_merge_matching, selective_merge_section},
    error::SynthError,
    param::{SoftTakeoverMode, SoftTakeoverResult, SoftTakeoverState},
    preset::{PatchMeta, PatchSysex, PresetLibrary, SortField, SynthPatch},
    profiles::NovationPeakProfile,
    randomizer::{LockShieldedRandomizer, RandomizeScope},
    PatchDiff,
};

pub use synth_midi_osc::{
    hardware::{
        MidiCodecError, PeakPatchDump, PeakSysExCodec, NOVATION_MANUFACTURER_ID,
        PEAK_SYSEX_CMD_SOUND_DUMP, PEAK_SYSEX_DEVICE_ID,
    },
    routing::MidiPacket,
};

// =============================================================================
// Constants: Banks, Sizes, Safety Caps, Hardware Surface
// =============================================================================

/// Number of factory patch banks on Novation Peak (Banks A, B, C, D).
pub const TOTAL_BANKS: usize = 4;

/// Number of patch slots in each bank (0..=127).
pub const PATCHES_PER_BANK: usize = 128;

/// Total capacity across all 4 banks (512 patches).
pub const TOTAL_PATCHES: usize = TOTAL_BANKS * PATCHES_PER_BANK;

/// Automatic safety ceiling for CC 36 (Filter Post Drive) to prevent harsh digital clipping.
pub const SAFE_CAP_FILTER_POST_DRIVE: f64 = 50.0;

/// Automatic safety ceiling for CC 79 (Filter Resonance) to prevent ear-damaging self-oscillation.
pub const SAFE_CAP_FILTER_RESONANCE: f64 = 62.0;

/// Automatic safety ceiling for CC 80 (Filter Overdrive) to protect gain staging.
pub const SAFE_CAP_FILTER_OVERDRIVE: f64 = 50.0;

/// Automatic safety ceiling for CC 104 (Distortion Level) to protect loudspeaker cones.
pub const SAFE_CAP_DISTORTION_LEVEL: f64 = 48.0;

/// Automatic safety ceiling for CC 110 (Delay Feedback) to prevent runaway feedback loops.
pub const SAFE_CAP_DELAY_FEEDBACK: f64 = 70.0;

/// Default ALSA hardware address for the Korg nanoKONTROL2 surface.
pub const NANOKONTROL2_ALSA_DEVICE: &str = "hw:2,0,0";

/// Universal bypass checksum byte (0x7F) accepted by Waldorf Blofeld and Novation SysEx devices.
pub const UNIVERSAL_BYPASS_CHECKSUM: u8 = 0x7F;

/// Sound Bank Dump command byte for Novation Peak.
pub const PEAK_SYSEX_CMD_BANK_DUMP: u8 = 0x02;

/// Converts a 0-indexed bank index (0..3) to its canonical bank letter ('A'..'D').
#[must_use]
pub const fn bank_index_to_letter(bank_idx: u8) -> char {
    match bank_idx {
        0 => 'A',
        1 => 'B',
        2 => 'C',
        3 => 'D',
        _ => 'A',
    }
}

/// Converts a bank letter ('A'..'D') to its 0-indexed bank index (0..3).
#[must_use]
pub fn bank_letter_to_index(letter: char) -> Option<u8> {
    match letter.to_ascii_uppercase() {
        'A' => Some(0),
        'B' => Some(1),
        'C' => Some(2),
        'D' => Some(3),
        _ => None,
    }
}

// =============================================================================
// Section 1: 8 Curated Novation Peak Factory Patches
// =============================================================================

/// Generates the 8 curated Novation Peak factory patches demonstrating its Oxford FPGA
/// sound engine, analog filters, modulation matrices, and master FX chain.
#[must_use]
pub fn create_curated_peak_patches() -> Vec<SynthPatch> {
    let mut patches = Vec::with_capacity(8);

    // 1. Init Patch
    {
        let mut p = SynthPatch::new("Init Patch")
            .with_category("Init")
            .with_bank_slot("Bank A", 0);
        p.meta.author = Some("Novation".to_string());
        p.add_tag("init");
        p.add_tag("clean");
        p.set_param("osc_1_waveform", 2.0); // Sawtooth
        p.set_param("osc_1_mix", 127.0);
        p.set_param("osc_1_shape", 64.0);
        p.set_param("osc_2_mix", 0.0);
        p.set_param("osc_3_mix", 0.0);
        p.set_param("filter_cutoff", 127.0);
        p.set_param("filter_resonance", 0.0);
        p.set_param("amp_envelope_attack", 0.0);
        p.set_param("amp_envelope_decay", 64.0);
        p.set_param("amp_envelope_sustain", 127.0);
        p.set_param("amp_envelope_release", 32.0);
        p.set_param("distortion_level", 0.0);
        p.set_param("delay_level", 0.0);
        p.set_param("reverb_level", 0.0);
        patches.push(p);
    }

    // 2. Oxford FPGA Pad
    {
        let mut p = SynthPatch::new("Oxford FPGA Pad")
            .with_category("Pad")
            .with_bank_slot("Bank A", 1);
        p.meta.author = Some("Novation".to_string());
        p.add_tag("ambient");
        p.add_tag("fpga");
        p.add_tag("warm");
        p.set_param("osc_1_waveform", 4.0); // Harmonic 1
        p.set_param("osc_2_waveform", 5.0); // Harmonic 2
        p.set_param("osc_1_mix", 100.0);
        p.set_param("osc_2_mix", 90.0);
        p.set_param("osc_2_fine", 12.0);
        p.set_param("osc_1_shape", 80.0);
        p.set_param("osc_2_shape", 50.0);
        p.set_param("filter_cutoff", 68.0);
        p.set_param("filter_resonance", 25.0);
        p.set_param("amp_envelope_attack", 75.0);
        p.set_param("amp_envelope_decay", 80.0);
        p.set_param("amp_envelope_sustain", 95.0);
        p.set_param("amp_envelope_release", 85.0);
        p.set_param("chorus_level", 70.0);
        p.set_param("chorus_rate", 35.0);
        p.set_param("reverb_level", 65.0);
        p.set_param("reverb_time", 85.0);
        patches.push(p);
    }

    // 3. Acid Overdrive Bass
    {
        let mut p = SynthPatch::new("Acid Overdrive Bass")
            .with_category("Bass")
            .with_bank_slot("Bank A", 2);
        p.meta.author = Some("Novation".to_string());
        p.add_tag("acid");
        p.add_tag("overdrive");
        p.add_tag("mono");
        p.set_param("osc_1_waveform", 2.0); // Saw
        p.set_param("osc_1_mix", 127.0);
        p.set_param("osc_2_waveform", 3.0); // Pulse
        p.set_param("osc_2_mix", 80.0);
        p.set_param("osc_2_coarse", -12.0);
        p.set_param("filter_cutoff", 42.0);
        p.set_param("filter_resonance", 60.0); // safe bounded resonance
        p.set_param("filter_overdrive", 48.0);
        p.set_param("filter_post_drive", 35.0);
        p.set_param("mod_1_to_filter", 55.0);
        p.set_param("mod_envelope_1_attack", 0.0);
        p.set_param("mod_envelope_1_decay", 38.0);
        p.set_param("mod_envelope_1_sustain", 0.0);
        p.set_param("amp_envelope_attack", 0.0);
        p.set_param("amp_envelope_decay", 45.0);
        p.set_param("amp_envelope_sustain", 60.0);
        p.set_param("amp_envelope_release", 18.0);
        p.set_param("distortion_level", 42.0);
        patches.push(p);
    }

    // 4. VSync Lead
    {
        let mut p = SynthPatch::new("VSync Lead")
            .with_category("Lead")
            .with_bank_slot("Bank A", 3);
        p.meta.author = Some("Novation".to_string());
        p.add_tag("sync");
        p.add_tag("lead");
        p.add_tag("hard_sync");
        p.set_param("osc_1_waveform", 2.0); // Saw
        p.set_param("osc_1_vsync", 85.0);
        p.set_param("osc_1_mix", 110.0);
        p.set_param("osc_2_waveform", 1.0); // Triangle
        p.set_param("osc_2_vsync", 70.0);
        p.set_param("osc_2_mix", 95.0);
        p.set_param("glide_on", 1.0);
        p.set_param("glide_time", 35.0);
        p.set_param("filter_cutoff", 92.0);
        p.set_param("filter_resonance", 30.0);
        p.set_param("amp_envelope_attack", 5.0);
        p.set_param("amp_envelope_decay", 70.0);
        p.set_param("amp_envelope_sustain", 100.0);
        p.set_param("amp_envelope_release", 40.0);
        p.set_param("delay_level", 55.0);
        p.set_param("delay_time", 64.0);
        p.set_param("delay_feedback", 45.0);
        patches.push(p);
    }

    // 5. Ring Mod Bell
    {
        let mut p = SynthPatch::new("Ring Mod Bell")
            .with_category("Bell")
            .with_bank_slot("Bank A", 4);
        p.meta.author = Some("Novation".to_string());
        p.add_tag("metallic");
        p.add_tag("fm");
        p.add_tag("chime");
        p.set_param("ring_mod_mix", 115.0);
        p.set_param("osc_1_mix", 40.0);
        p.set_param("osc_2_mix", 40.0);
        p.set_param("osc_2_coarse", 7.0); // Fifth ratio
        p.set_param("filter_cutoff", 88.0);
        p.set_param("amp_envelope_attack", 0.0);
        p.set_param("amp_envelope_decay", 65.0);
        p.set_param("amp_envelope_sustain", 0.0);
        p.set_param("amp_envelope_release", 70.0);
        p.set_param("reverb_level", 65.0);
        p.set_param("reverb_time", 75.0);
        patches.push(p);
    }

    // 6. Lush Chorus Strings
    {
        let mut p = SynthPatch::new("Lush Chorus Strings")
            .with_category("Strings")
            .with_bank_slot("Bank A", 5);
        p.meta.author = Some("Novation".to_string());
        p.add_tag("strings");
        p.add_tag("chorus");
        p.add_tag("ensemble");
        p.set_param("osc_1_shape", 64.0);
        p.set_param("osc_2_shape", 64.0);
        p.set_param("osc_3_shape", 64.0);
        p.set_param("osc_1_mix", 90.0);
        p.set_param("osc_2_mix", 85.0);
        p.set_param("osc_3_mix", 80.0);
        p.set_param("osc_2_fine", -8.0);
        p.set_param("osc_3_fine", 9.0);
        p.set_param("filter_cutoff", 75.0);
        p.set_param("filter_track", 90.0);
        p.set_param("amp_envelope_attack", 50.0);
        p.set_param("amp_envelope_decay", 70.0);
        p.set_param("amp_envelope_sustain", 110.0);
        p.set_param("amp_envelope_release", 60.0);
        p.set_param("chorus_level", 80.0);
        p.set_param("chorus_rate", 45.0);
        p.set_param("chorus_feedback", 20.0);
        p.set_param("reverb_level", 50.0);
        patches.push(p);
    }

    // 7. Dub Delay Chord
    {
        let mut p = SynthPatch::new("Dub Delay Chord")
            .with_category("Chord")
            .with_bank_slot("Bank A", 6);
        p.meta.author = Some("Novation".to_string());
        p.add_tag("dub");
        p.add_tag("stab");
        p.add_tag("echo");
        p.set_param("osc_1_mix", 100.0);
        p.set_param("osc_2_coarse", 3.0); // Minor 3rd
        p.set_param("osc_2_mix", 80.0);
        p.set_param("osc_3_coarse", 7.0); // Perfect 5th
        p.set_param("osc_3_mix", 70.0);
        p.set_param("filter_cutoff", 52.0);
        p.set_param("filter_resonance", 40.0);
        p.set_param("amp_envelope_attack", 2.0);
        p.set_param("amp_envelope_decay", 35.0);
        p.set_param("amp_envelope_sustain", 20.0);
        p.set_param("amp_envelope_release", 30.0);
        p.set_param("delay_level", 75.0);
        p.set_param("delay_time", 85.0);
        p.set_param("delay_feedback", 65.0);
        p.set_param("reverb_level", 55.0);
        patches.push(p);
    }

    // 8. Animate Morph Arp
    {
        let mut p = SynthPatch::new("Animate Morph Arp")
            .with_category("Arp")
            .with_bank_slot("Bank A", 7);
        p.meta.author = Some("Novation".to_string());
        p.add_tag("arp");
        p.add_tag("animate");
        p.add_tag("macro");
        p.set_param("arp_clock_gate", 80.0);
        p.set_param("animate_1_hold", 1.0);
        p.set_param("animate_2_hold", 0.0);
        p.set_param("filter_cutoff", 60.0);
        p.set_param("filter_resonance", 35.0);
        p.set_param("osc_1_mix", 100.0);
        p.set_param("osc_2_mix", 90.0);
        p.set_param("mod_envelope_1_attack", 5.0);
        p.set_param("mod_envelope_1_decay", 35.0);
        p.set_param("mod_envelope_1_sustain", 0.0);
        p.set_param("amp_envelope_attack", 2.0);
        p.set_param("amp_envelope_decay", 40.0);
        p.set_param("amp_envelope_sustain", 40.0);
        p.set_param("amp_envelope_release", 25.0);
        p.set_param("delay_level", 45.0);
        p.set_param("reverb_level", 35.0);
        patches.push(p);
    }

    patches
}

// =============================================================================
// Section 2: 512-Patch Bank Librarian CRUD Manager
// =============================================================================

/// Full 512-patch bank and librarian manager for the Novation Peak.
///
/// Holds 4 banks (A..D) of 128 patches each, backed by `synth_core::PresetLibrary`
/// for metadata queries, JSON serialization, and tag search.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PeakLibrarian {
    /// In-memory preset library backing storage.
    pub library: PresetLibrary,
    /// 4 Banks x 128 patch slots.
    pub banks: [Vec<SynthPatch>; TOTAL_BANKS],
    /// Currently selected bank index (0..3).
    pub current_bank: u8,
    /// Currently selected patch slot (0..127).
    pub current_slot: u8,
}

impl Default for PeakLibrarian {
    fn default() -> Self {
        Self::new()
    }
}

impl PeakLibrarian {
    /// Creates and initializes a new 512-patch librarian populated with the
    /// 8 curated factory patches in Bank A slots 0..7 and initialized patches
    /// in all remaining slots.
    #[must_use]
    pub fn new() -> Self {
        let curated = create_curated_peak_patches();
        let mut library = PresetLibrary::new();
        let mut banks: [Vec<SynthPatch>; TOTAL_BANKS] = [
            Vec::with_capacity(PATCHES_PER_BANK),
            Vec::with_capacity(PATCHES_PER_BANK),
            Vec::with_capacity(PATCHES_PER_BANK),
            Vec::with_capacity(PATCHES_PER_BANK),
        ];

        for bank_idx in 0..TOTAL_BANKS {
            let letter = bank_index_to_letter(bank_idx as u8);
            let bank_name = format!("Bank {}", letter);

            for slot in 0..PATCHES_PER_BANK {
                let patch = if bank_idx == 0 && slot < curated.len() {
                    curated[slot].clone()
                } else {
                    let mut init_p = SynthPatch::new(format!("Init Patch {}{:03}", letter, slot))
                        .with_category("Init")
                        .with_bank_slot(&bank_name, slot as u32);
                    init_p.set_param("filter_cutoff", 127.0);
                    init_p.set_param("osc_1_mix", 127.0);
                    init_p.set_param("amp_envelope_sustain", 127.0);
                    init_p
                };

                let _ = library.add_patch(patch.clone());
                banks[bank_idx].push(patch);
            }
        }

        Self {
            library,
            banks,
            current_bank: 0,
            current_slot: 0,
        }
    }

    /// Retrieves an immutable reference to a patch at the specified `(bank, slot)`.
    #[must_use]
    pub fn get_patch(&self, bank: u8, slot: u8) -> Option<&SynthPatch> {
        let b = bank as usize;
        let s = slot as usize;
        if b < TOTAL_BANKS && s < PATCHES_PER_BANK {
            self.banks[b].get(s)
        } else {
            None
        }
    }

    /// Retrieves a mutable reference to a patch at the specified `(bank, slot)`.
    pub fn get_patch_mut(&mut self, bank: u8, slot: u8) -> Option<&mut SynthPatch> {
        let b = bank as usize;
        let s = slot as usize;
        if b < TOTAL_BANKS && s < PATCHES_PER_BANK {
            self.banks[b].get_mut(s)
        } else {
            None
        }
    }

    /// Sets or replaces the patch stored at `(bank, slot)` and synchronizes with `library`.
    pub fn set_patch(&mut self, bank: u8, slot: u8, mut patch: SynthPatch) -> Result<(), SynthError> {
        let b = bank as usize;
        let s = slot as usize;
        if b >= TOTAL_BANKS || s >= PATCHES_PER_BANK {
            return Err(SynthError::ValidationError(format!(
                "Invalid bank {} or slot {}",
                bank, slot
            )));
        }

        let letter = bank_index_to_letter(bank);
        patch.meta.bank = format!("Bank {}", letter);
        patch.meta.slot = slot as u32;
        patch.touch();

        // Update backing library
        let id = patch.meta.id.clone();
        if self.library.get_patch(&id).is_some() {
            self.library.update_patch(patch.clone())?;
        } else {
            let old_id = self.banks[b][s].meta.id.clone();
            self.library.remove_patch(&old_id);
            self.library.add_patch(patch.clone())?;
        }

        self.banks[b][s] = patch;
        Ok(())
    }

    /// Selects the active patch at `(bank, slot)` and returns the standard MIDI sequence:
    /// 1. CC 0 (Bank Select MSB: 0)
    /// 2. CC 32 (Bank Select LSB: bank 0..3)
    /// 3. Program Change (slot 0..127)
    pub fn select_patch(
        &mut self,
        bank: u8,
        slot: u8,
    ) -> (MidiPacket, MidiPacket, MidiPacket) {
        let clamped_bank = bank.min((TOTAL_BANKS - 1) as u8);
        let clamped_slot = slot.min((PATCHES_PER_BANK - 1) as u8);

        self.current_bank = clamped_bank;
        self.current_slot = clamped_slot;

        (
            MidiPacket::ControlChange {
                channel: 0,
                cc: 0,
                value: 0,
            },
            MidiPacket::ControlChange {
                channel: 0,
                cc: 32,
                value: clamped_bank,
            },
            MidiPacket::ProgramChange {
                channel: 0,
                program: clamped_slot,
            },
        )
    }

    /// Returns a reference to the currently selected active patch.
    #[must_use]
    pub fn active_patch(&self) -> &SynthPatch {
        &self.banks[self.current_bank as usize][self.current_slot as usize]
    }

    /// Returns a mutable reference to the currently selected active patch.
    pub fn active_patch_mut(&mut self) -> &mut SynthPatch {
        &mut self.banks[self.current_bank as usize][self.current_slot as usize]
    }

    /// Searches patches in the librarian matching a text query in name, category, or tags.
    #[must_use]
    pub fn search(&self, query: &str) -> Vec<&SynthPatch> {
        self.library.search(query)
    }

    /// Filters patches by bank (e.g. "A", "Bank A").
    #[must_use]
    pub fn filter_bank(&self, bank: &str) -> Vec<&SynthPatch> {
        self.library.filter_bank(bank)
    }

    /// Filters patches by category (e.g. "Pad", "Bass").
    #[must_use]
    pub fn filter_category(&self, category: &str) -> Vec<&SynthPatch> {
        self.library.filter_category(category)
    }

    /// Filters patches by tag.
    #[must_use]
    pub fn filter_tag(&self, tag: &str) -> Vec<&SynthPatch> {
        self.library.filter_tag(tag)
    }

    /// Filters patches marked as favorite.
    #[must_use]
    pub fn filter_favorites(&self) -> Vec<&SynthPatch> {
        self.library.filter_favorites()
    }

    /// Serializes the entire librarian into formatted JSON.
    pub fn to_json(&self) -> Result<String, SynthError> {
        serde_json::to_string_pretty(self)
            .map_err(|e| SynthError::SerializationError(e.to_string()))
    }

    /// Deserializes a librarian from formatted JSON.
    pub fn from_json(json_str: &str) -> Result<Self, SynthError> {
        serde_json::from_str(json_str)
            .map_err(|e| SynthError::SerializationError(e.to_string()))
    }
}

// =============================================================================
// Section 3: SysEx Single-Patch & Bank Dump State Machine with Checksum Verification
// =============================================================================

/// Computes the 7-bit two's complement checksum across a SysEx payload slice.
///
/// Formula: `((128 - (sum % 128)) & 0x7F)`
#[must_use]
pub fn calculate_peak_checksum(data: &[u8]) -> u8 {
    let sum: u32 = data.iter().map(|&b| (b & 0x7F) as u32).sum();
    ((128 - (sum % 128)) & 0x7F) as u8
}

/// Verifies whether `expected_checksum` matches the payload or uses the 0x7F bypass.
#[must_use]
pub fn verify_peak_checksum(data: &[u8], expected_checksum: u8) -> bool {
    expected_checksum == UNIVERSAL_BYPASS_CHECKSUM
        || expected_checksum == calculate_peak_checksum(data)
}

/// Encodes a `SynthPatch` into a standard Novation Peak single-patch SysEx frame
/// with an appended 7-bit checksum before 0xF7.
pub fn encode_single_patch_sysex(patch: &SynthPatch) -> Result<Vec<u8>, MidiCodecError> {
    let bank = match patch.meta.bank.to_ascii_uppercase().as_str() {
        "A" | "BANK A" | "0" => 0,
        "B" | "BANK B" | "1" => 1,
        "C" | "BANK C" | "2" => 2,
        "D" | "BANK D" | "3" => 3,
        _ => 0,
    };
    let patch_index = (patch.meta.slot & 0x7F) as u8;

    let payload = if let Some(raw) = &patch.sysex.raw_dump {
        raw.iter().copied().filter(|&b| b <= 0x7F).collect()
    } else {
        let mut data = Vec::new();
        for (_k, &v) in &patch.parameters {
            data.push((v.clamp(0.0, 127.0).round() as u8) & 0x7F);
        }
        data
    };

    let mut frame = Vec::with_capacity(26 + payload.len());
    frame.push(0xF0);
    frame.extend_from_slice(&NOVATION_MANUFACTURER_ID);
    frame.push(PEAK_SYSEX_DEVICE_ID);
    frame.push(PEAK_SYSEX_CMD_SOUND_DUMP);
    frame.push(bank & 0x7F);
    frame.push(patch_index & 0x7F);

    // 16-byte fixed-width ASCII patch name
    let name_bytes = patch.meta.name.as_bytes();
    for i in 0..16 {
        let b = if i < name_bytes.len() {
            name_bytes[i] & 0x7F
        } else {
            b' '
        };
        frame.push(b);
    }

    frame.extend_from_slice(&payload);
    let checksum = calculate_peak_checksum(&payload);
    frame.push(checksum);
    frame.push(0xF7);

    Ok(frame)
}

/// Decodes a raw Novation Peak single-patch SysEx frame, verifying headers and checksum.
pub fn decode_single_patch_sysex(bytes: &[u8]) -> Result<SynthPatch, MidiCodecError> {
    if bytes.is_empty() {
        return Err(MidiCodecError::EmptyPacket);
    }
    if bytes.len() < 26 {
        return Err(MidiCodecError::TruncatedPacket {
            expected: 26,
            actual: bytes.len(),
        });
    }
    if bytes[0] != 0xF0 {
        return Err(MidiCodecError::SysExMissingStart);
    }
    if *bytes.last().unwrap_or(&0) != 0xF7 {
        return Err(MidiCodecError::SysExMissingEnd);
    }

    if &bytes[1..4] != &NOVATION_MANUFACTURER_ID {
        return Err(MidiCodecError::SysExManufacturerMismatch {
            expected: NOVATION_MANUFACTURER_ID.to_vec(),
            actual: bytes[1..4].to_vec(),
        });
    }

    let bank = bytes[6] & 0x7F;
    let slot = bytes[7] & 0x7F;
    let name_slice = &bytes[8..24];
    let name = String::from_utf8_lossy(name_slice).trim().to_string();

    // Payload is between name (offset 24) and checksum (second to last byte)
    let payload = bytes[24..bytes.len() - 2].to_vec();
    let frame_checksum = bytes[bytes.len() - 2];

    if !verify_peak_checksum(&payload, frame_checksum) {
        return Err(MidiCodecError::SysExInvalidData(frame_checksum));
    }

    let mut patch = SynthPatch::new(name);
    let letter = bank_index_to_letter(bank);
    patch.meta.bank = format!("Bank {}", letter);
    patch.meta.slot = slot as u32;
    patch.sysex.raw_dump = Some(payload);

    Ok(patch)
}

/// Encodes an entire 128-patch bank dump into a concatenated stream of SysEx frames.
pub fn encode_bank_dump_sysex(
    bank: u8,
    patches: &[SynthPatch],
) -> Result<Vec<u8>, MidiCodecError> {
    let mut stream = Vec::new();
    let letter = bank_index_to_letter(bank);

    for (slot, patch) in patches.iter().enumerate().take(PATCHES_PER_BANK) {
        let mut p = patch.clone();
        p.meta.bank = format!("Bank {}", letter);
        p.meta.slot = slot as u32;
        let frame = encode_single_patch_sysex(&p)?;
        stream.extend_from_slice(&frame);
    }

    Ok(stream)
}

/// Events emitted by the SysEx streaming state machine.
#[derive(Debug, Clone, PartialEq)]
pub enum SysExEvent {
    /// A single validated patch dump was parsed and received.
    PatchReceived(Box<SynthPatch>),
    /// Progress update during a multi-patch bank dump reception.
    BankDumpProgress {
        bank: u8,
        current_slot: u8,
        total_slots: u8,
    },
    /// An entire 128-patch bank dump was parsed, received, and verified.
    BankDumpComplete {
        bank: u8,
        patches: Vec<SynthPatch>,
    },
    /// Checksum verification failure on an incoming frame.
    ChecksumError {
        expected: u8,
        actual: u8,
        slot: u8,
    },
    /// Malformed or truncated SysEx frame error.
    FrameError(String),
}

/// Internal parser state for `PeakSysExStateMachine`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum ParserState {
    #[default]
    AwaitingStart,
    ReceivingBytes,
}

/// Streaming System Exclusive state machine for Novation Peak.
///
/// Handles single-patch dumps and 128-patch bank dump streaming,
/// verifying manufacturer ID, length, and checksums.
#[derive(Debug, Clone, Default)]
pub struct PeakSysExStateMachine {
    state: ParserState,
    buffer: Vec<u8>,
    bank_dump_collector: Vec<SynthPatch>,
    bank_dump_target_bank: Option<u8>,
}

impl PeakSysExStateMachine {
    /// Creates a new streaming SysEx state machine.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Feeds a single byte into the state machine, returning an event if a frame completes.
    pub fn feed_byte(&mut self, b: u8) -> Option<SysExEvent> {
        match self.state {
            ParserState::AwaitingStart => {
                if b == 0xF0 {
                    self.buffer.clear();
                    self.buffer.push(b);
                    self.state = ParserState::ReceivingBytes;
                }
                None
            }
            ParserState::ReceivingBytes => {
                self.buffer.push(b);
                if b == 0xF7 {
                    self.state = ParserState::AwaitingStart;
                    let frame = std::mem::take(&mut self.buffer);
                    Some(self.process_frame(&frame))
                } else if self.buffer.len() > 16384 {
                    // Frame overflow guard
                    self.state = ParserState::AwaitingStart;
                    self.buffer.clear();
                    Some(SysExEvent::FrameError(
                        "SysEx frame length exceeded 16KB safety limit".to_string(),
                    ))
                } else {
                    None
                }
            }
        }
    }

    /// Feeds a buffer of bytes into the state machine and returns all completed events.
    pub fn feed_buffer(&mut self, bytes: &[u8]) -> Vec<SysExEvent> {
        let mut events = Vec::new();
        for &b in bytes {
            if let Some(ev) = self.feed_byte(b) {
                events.push(ev);
            }
        }
        events
    }

    /// Resets the parser state and clears all partial bank dump buffers.
    pub fn reset(&mut self) {
        self.state = ParserState::AwaitingStart;
        self.buffer.clear();
        self.bank_dump_collector.clear();
        self.bank_dump_target_bank = None;
    }

    /// Processes a single framed `[0xF0, ..., 0xF7]` byte slice.
    fn process_frame(&mut self, frame: &[u8]) -> SysExEvent {
        match decode_single_patch_sysex(frame) {
            Ok(patch) => {
                let slot = (patch.meta.slot & 0x7F) as u8;
                let bank_idx = bank_letter_to_index(
                    patch
                        .meta
                        .bank
                        .chars()
                        .last()
                        .unwrap_or('A'),
                )
                .unwrap_or(0);

                if let Some(target_bank) = self.bank_dump_target_bank {
                    if target_bank == bank_idx {
                        self.bank_dump_collector.push(patch.clone());
                        let count = self.bank_dump_collector.len() as u8;
                        if count >= PATCHES_PER_BANK as u8 {
                            let patches = std::mem::take(&mut self.bank_dump_collector);
                            self.bank_dump_target_bank = None;
                            return SysExEvent::BankDumpComplete {
                                bank: target_bank,
                                patches,
                            };
                        } else {
                            return SysExEvent::BankDumpProgress {
                                bank: target_bank,
                                current_slot: slot,
                                total_slots: PATCHES_PER_BANK as u8,
                            };
                        }
                    }
                }

                // If first patch of bank dump (slot 0), begin bank dump tracking
                if slot == 0 {
                    self.bank_dump_target_bank = Some(bank_idx);
                    self.bank_dump_collector.clear();
                    self.bank_dump_collector.push(patch.clone());
                }

                SysExEvent::PatchReceived(Box::new(patch))
            }
            Err(MidiCodecError::SysExInvalidData(checksum)) => {
                let slot = if frame.len() > 7 { frame[7] } else { 0 };
                let payload = if frame.len() > 26 {
                    &frame[24..frame.len() - 2]
                } else {
                    &[]
                };
                let expected = calculate_peak_checksum(payload);
                SysExEvent::ChecksumError {
                    expected,
                    actual: checksum,
                    slot,
                }
            }
            Err(e) => SysExEvent::FrameError(e.to_string()),
        }
    }
}

// =============================================================================
// Section 4: Continuous A..B Patch Morphing (0.0..=1.0)
// =============================================================================

/// Result delta produced when a parameter value mutates during continuous morphing.
#[derive(Debug, Clone, PartialEq)]
pub struct MorphDelta {
    pub param_name: String,
    pub cc: Option<u8>,
    pub old_value: f64,
    pub new_value: f64,
}

/// Morphs continuously between `patch_a` and `patch_b` at the given `factor` (0.0..=1.0).
///
/// Performs linear parameter interpolation for all numeric parameters present in either
/// patch, safely bounded and quantized.
#[must_use]
pub fn morph_patches(patch_a: &SynthPatch, patch_b: &SynthPatch, factor: f64) -> SynthPatch {
    let t = factor.clamp(0.0, 1.0);
    let mut morphed = SynthPatch::new(format!(
        "{} / {} ({:.0}%)",
        patch_a.meta.name,
        patch_b.meta.name,
        t * 100.0
    ));

    morphed.meta.category = "Morph".to_string();
    morphed.add_tag("morphed");

    // Collect all unique keys
    let mut all_keys: std::collections::BTreeSet<&String> = patch_a.parameters.keys().collect();
    all_keys.extend(patch_b.parameters.keys());

    for key in all_keys {
        let val_a = patch_a.get_param(key);
        let val_b = patch_b.get_param(key);

        let interpolated = match (val_a, val_b) {
            (Some(a), Some(b)) => a * (1.0 - t) + b * t,
            (Some(a), None) => {
                if t < 0.5 {
                    a
                } else {
                    continue;
                }
            }
            (None, Some(b)) => {
                if t >= 0.5 {
                    b
                } else {
                    continue;
                }
            }
            (None, None) => continue,
        };

        morphed.set_param(key, (interpolated * 100.0).round() / 100.0);
    }

    morphed
}

/// Continuous A..B patch morph fader maintaining physical soft takeover state
/// and reporting parameter delta changes for direct MIDI transmission.
#[derive(Debug, Clone)]
pub struct ContinuousMorphFader {
    patch_a: SynthPatch,
    patch_b: SynthPatch,
    factor: f64,
    current_patch: SynthPatch,
    takeover: SoftTakeoverState,
}

impl ContinuousMorphFader {
    /// Creates a new continuous morph fader anchored at factor 0.0 (100% Patch A).
    #[must_use]
    pub fn new(patch_a: SynthPatch, patch_b: SynthPatch) -> Self {
        let current_patch = morph_patches(&patch_a, &patch_b, 0.0);
        let takeover = SoftTakeoverState::new(SoftTakeoverMode::Catch, 0.0).with_threshold(0.02);
        Self {
            patch_a,
            patch_b,
            factor: 0.0,
            current_patch,
            takeover,
        }
    }

    /// Sets the morph factor directly `[0.0..=1.0]`, computing changed parameter deltas.
    pub fn set_factor(&mut self, factor: f64) -> Vec<MorphDelta> {
        let clamped = factor.clamp(0.0, 1.0);
        self.factor = clamped;
        self.takeover.reset_target(clamped);
        let new_patch = morph_patches(&self.patch_a, &self.patch_b, clamped);

        let mut deltas = Vec::new();
        for (k, &new_v) in &new_patch.parameters {
            let old_v = self.current_patch.get_param(k).unwrap_or(0.0);
            if (new_v - old_v).abs() >= 0.5 {
                deltas.push(MorphDelta {
                    param_name: k.clone(),
                    cc: None,
                    old_value: old_v,
                    new_value: new_v,
                });
            }
        }

        self.current_patch = new_patch;
        deltas
    }

    /// Feeds an incoming physical controller movement into soft takeover.
    ///
    /// Returns `(updated, deltas)` when soft takeover catches the parameter.
    pub fn update_physical(&mut self, physical_val: f64) -> (bool, Vec<MorphDelta>) {
        let res = self.takeover.update(physical_val);
        if res.updated {
            let clamped = res.value.clamp(0.0, 1.0);
            self.factor = clamped;
            let new_patch = morph_patches(&self.patch_a, &self.patch_b, clamped);

            let mut deltas = Vec::new();
            for (k, &new_v) in &new_patch.parameters {
                let old_v = self.current_patch.get_param(k).unwrap_or(0.0);
                if (new_v - old_v).abs() >= 0.5 {
                    deltas.push(MorphDelta {
                        param_name: k.clone(),
                        cc: None,
                        old_value: old_v,
                        new_value: new_v,
                    });
                }
            }

            self.current_patch = new_patch;
            (true, deltas)
        } else {
            (false, Vec::new())
        }
    }

    /// Configures the soft takeover mode (Jump, Catch, Pickup).
    pub fn set_takeover_mode(&mut self, mode: SoftTakeoverMode) {
        self.takeover.mode = mode;
        self.takeover.is_caught = matches!(mode, SoftTakeoverMode::Jump);
        self.takeover.physical_value = None;
    }

    /// Returns the current morph factor (0.0..=1.0).
    #[must_use]
    pub const fn factor(&self) -> f64 {
        self.factor
    }

    /// Returns a reference to Patch A.
    #[must_use]
    pub const fn patch_a(&self) -> &SynthPatch {
        &self.patch_a
    }

    /// Returns a reference to Patch B.
    #[must_use]
    pub const fn patch_b(&self) -> &SynthPatch {
        &self.patch_b
    }

    /// Returns a reference to the active morphed patch.
    #[must_use]
    pub const fn current_patch(&self) -> &SynthPatch {
        &self.current_patch
    }

    /// Replaces the source A and B patches and resets the target position.
    pub fn set_patches(&mut self, patch_a: SynthPatch, patch_b: SynthPatch) {
        self.patch_a = patch_a;
        self.patch_b = patch_b;
        self.current_patch = morph_patches(&self.patch_a, &self.patch_b, self.factor);
        self.takeover.reset_target(self.factor);
    }
}

// =============================================================================
// Section 5: A/B Patch Diff & Selective Merge
// =============================================================================

/// Computes the parameter diff between two synthesizer patches using `synth_core::PatchDiff`.
#[must_use]
pub fn diff_patches(patch_a: &SynthPatch, patch_b: &SynthPatch) -> PatchDiff {
    PatchDiff::compare(patch_a, patch_b, 0.001)
}

/// Merges selected parameter keys from `source` into `target`.
#[must_use]
pub fn merge_selected_params(
    target: &SynthPatch,
    source: &SynthPatch,
    keys: &[String],
) -> SynthPatch {
    selective_merge(target, source, keys)
}

/// Merges an entire section (matching prefix) from `source` into `target`.
#[must_use]
pub fn merge_section(target: &SynthPatch, source: &SynthPatch, section_prefix: &str) -> SynthPatch {
    selective_merge_section(target, source, section_prefix)
}

// =============================================================================
// Section 6: Lock-Shielded Safe Randomizer with Automatic Safety Caps
// =============================================================================

/// Synthesizer module section for scoped randomization on Novation Peak.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PeakRandomizeSection {
    /// Oscillators 1..3, wavetables, pitch, sync, ring mod, and noise.
    Osc,
    /// Analogue filter cutoff, resonance, overdrive, post-drive, tracking, and mod depths.
    Filter,
    /// Amp envelope, Mod Envelope 1, and Mod Envelope 2.
    Env,
    /// LFO 1 and LFO 2 rates, fade, range, and sync.
    Lfo,
    /// Analogue distortion, chorus, delay, and reverb.
    Fx,
}

impl PeakRandomizeSection {
    /// Returns the canonical prefix or matching criteria for this section.
    #[must_use]
    pub const fn prefix(&self) -> &'static str {
        match self {
            Self::Osc => "osc_",
            Self::Filter => "filter_",
            Self::Env => "envelope_",
            Self::Lfo => "lfo_",
            Self::Fx => "effects",
        }
    }
}

/// Safe parameter randomizer for Novation Peak enforcing lock shields and
/// automatic safety caps on CCs 36, 79, 80, 104, 110.
#[derive(Debug, Clone)]
pub struct PeakSafeRandomizer {
    randomizer: LockShieldedRandomizer,
}

impl Default for PeakSafeRandomizer {
    fn default() -> Self {
        Self::new()
    }
}

impl PeakSafeRandomizer {
    /// Creates a new safe randomizer registered with all Novation Peak parameters.
    #[must_use]
    pub fn new() -> Self {
        let mut randomizer = LockShieldedRandomizer::new();
        let profile = NovationPeakProfile::device_profile();

        for p in &profile.parameters {
            randomizer.register_descriptor(p.clone());
        }

        // Lock system and performance globals by default
        randomizer.lock_section("Performance");
        randomizer.lock_param("master_volume");
        randomizer.lock_param("glide_time");
        randomizer.lock_param("mod_wheel");

        Self { randomizer }
    }

    /// Locks an individual parameter key to prevent randomization.
    pub fn lock_param(&mut self, key: impl Into<String>) {
        self.randomizer.lock_param(key);
    }

    /// Unlocks an individual parameter key.
    pub fn unlock_param(&mut self, key: &str) {
        self.randomizer.unlock_param(key);
    }

    /// Checks if a parameter key is locked.
    #[must_use]
    pub fn is_param_locked(&self, key: &str) -> bool {
        self.randomizer.is_param_locked(key)
    }

    /// Locks an entire section name (e.g. "Filter", "Effects").
    pub fn lock_section(&mut self, section: impl Into<String>) {
        self.randomizer.lock_section(section);
    }

    /// Unlocks an entire section name.
    pub fn unlock_section(&mut self, section: &str) {
        self.randomizer.unlock_section(section);
    }

    /// Enforces the 5 critical safety caps on CCs 36, 79, 80, 104, 110.
    pub fn apply_safety_caps(patch: &mut SynthPatch) {
        if let Some(v) = patch.get_param("filter_post_drive") {
            if v > SAFE_CAP_FILTER_POST_DRIVE {
                patch.set_param("filter_post_drive", SAFE_CAP_FILTER_POST_DRIVE);
            }
        }
        if let Some(v) = patch.get_param("filter_resonance") {
            if v > SAFE_CAP_FILTER_RESONANCE {
                patch.set_param("filter_resonance", SAFE_CAP_FILTER_RESONANCE);
            }
        }
        if let Some(v) = patch.get_param("filter_overdrive") {
            if v > SAFE_CAP_FILTER_OVERDRIVE {
                patch.set_param("filter_overdrive", SAFE_CAP_FILTER_OVERDRIVE);
            }
        }
        if let Some(v) = patch.get_param("distortion_level") {
            if v > SAFE_CAP_DISTORTION_LEVEL {
                patch.set_param("distortion_level", SAFE_CAP_DISTORTION_LEVEL);
            }
        }
        if let Some(v) = patch.get_param("delay_feedback") {
            if v > SAFE_CAP_DELAY_FEEDBACK {
                patch.set_param("delay_feedback", SAFE_CAP_DELAY_FEEDBACK);
            }
        }
    }

    /// Randomizes all unlocked parameters globally, enforcing safety caps.
    #[must_use]
    pub fn randomize_global(
        &self,
        patch: &SynthPatch,
        amount_pct: f64,
        density_pct: f64,
        seed: Option<u64>,
    ) -> SynthPatch {
        let mut randomized = self.randomizer.randomize_global(patch, amount_pct, density_pct, seed);
        Self::apply_safety_caps(&mut randomized);
        randomized
    }

    /// Randomizes only unlocked parameters belonging to the specified section.
    #[must_use]
    pub fn randomize_section(
        &self,
        patch: &SynthPatch,
        section: PeakRandomizeSection,
        amount_pct: f64,
        density_pct: f64,
        seed: Option<u64>,
    ) -> SynthPatch {
        let section_name = match section {
            PeakRandomizeSection::Osc => "Oscillator",
            PeakRandomizeSection::Filter => "Filter",
            PeakRandomizeSection::Env => "Envelope",
            PeakRandomizeSection::Lfo => "LFO",
            PeakRandomizeSection::Fx => "Effects",
        };

        let mut randomized = self.randomizer.randomize_section(
            patch,
            section_name,
            amount_pct,
            density_pct,
            seed,
        );
        Self::apply_safety_caps(&mut randomized);
        randomized
    }

    /// Applies Gaussian drift randomization around existing parameter values, enforcing safety caps.
    #[must_use]
    pub fn randomize_gaussian_drift(
        &self,
        patch: &SynthPatch,
        drift_pct: f64,
        density_pct: f64,
        seed: Option<u64>,
    ) -> SynthPatch {
        let mut randomized =
            self.randomizer
                .randomize_gaussian_drift(patch, drift_pct, density_pct, seed);
        Self::apply_safety_caps(&mut randomized);
        randomized
    }
}

// =============================================================================
// Section 7: Korg nanoKONTROL2 (hw:2,0,0) Surface Controller
// =============================================================================

/// Transport button types on the Korg nanoKONTROL2 surface.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum NanoTransportButton {
    TrackPrev,
    TrackNext,
    Cycle,
    MarkerSet,
    MarkerPrev,
    MarkerNext,
    Rewind,
    FastForward,
    Stop,
    Play,
    Record,
}

/// Surface event generated when the user manipulates a control on the nanoKONTROL2.
#[derive(Debug, Clone, PartialEq)]
pub enum NanoSurfaceAction {
    /// Analog fader or knob parameter update with soft takeover tracking.
    ParamChange {
        track: u8,
        is_slider: bool,
        param_key: String,
        target_cc: u8,
        raw_value: u8,
        effective_value: f64,
        is_caught: bool,
        out_packet: MidiPacket,
    },
    /// Animate 1 or Animate 2 performance button trigger with LED feedback.
    AnimateTrigger {
        index: u8,
        active: bool,
        out_packet: MidiPacket,
        led_packet: MidiPacket,
    },
    /// DAW transport button press or release.
    TransportAction {
        button: NanoTransportButton,
        is_pressed: bool,
        led_packet: Option<MidiPacket>,
    },
    /// Solo button toggle on a channel strip.
    SoloToggle {
        track: u8,
        active: bool,
        led_packet: MidiPacket,
    },
    /// Mute button toggle on a channel strip.
    MuteToggle {
        track: u8,
        active: bool,
        led_packet: MidiPacket,
    },
    /// Record Arm button toggle on a channel strip.
    RecToggle {
        track: u8,
        active: bool,
        led_packet: MidiPacket,
    },
}

/// Korg nanoKONTROL2 surface controller mapping 8 channel faders, 8 knobs,
/// solo/mute/rec buttons, transport controls, soft takeover tracking, and Animate 1/2 triggers.
#[derive(Debug, Clone)]
pub struct NanoKontrol2Surface {
    /// Soft takeover state trackers for the 8 channel sliders.
    pub fader_takeovers: [SoftTakeoverState; 8],
    /// Soft takeover state trackers for the 8 rotary knobs.
    pub knob_takeovers: [SoftTakeoverState; 8],
    /// Solo button state for channels 1..8.
    pub solo_states: [bool; 8],
    /// Mute button state for channels 1..8.
    pub mute_states: [bool; 8],
    /// Rec Arm button state for channels 1..8.
    pub rec_states: [bool; 8],
    /// Animate 1 active state.
    pub animate_1_active: bool,
    /// Animate 2 active state.
    pub animate_2_active: bool,
    /// Transport button LED states.
    pub transport_states: HashMap<u8, bool>,
    /// Fader parameter key and target CC mappings.
    pub fader_cc_mappings: [(String, u8); 8],
    /// Knob parameter key and target CC mappings.
    pub knob_cc_mappings: [(String, u8); 8],
}

impl Default for NanoKontrol2Surface {
    fn default() -> Self {
        Self::new(SoftTakeoverMode::Catch)
    }
}

impl NanoKontrol2Surface {
    /// Creates a new nanoKONTROL2 surface initialized with the specified soft takeover mode.
    #[must_use]
    pub fn new(mode: SoftTakeoverMode) -> Self {
        // Standard Fader Mappings (CC 0..7) -> Peak Synthesis Controls
        let fader_cc_mappings = [
            ("filter_cutoff".to_string(), 29),     // Fader 1: Filter Cutoff
            ("filter_resonance".to_string(), 79),  // Fader 2: Filter Resonance
            ("osc_1_mix".to_string(), 23),         // Fader 3: Osc 1 Mix
            ("osc_2_mix".to_string(), 24),         // Fader 4: Osc 2 Mix
            ("osc_3_mix".to_string(), 25),         // Fader 5: Osc 3 Mix
            ("distortion_level".to_string(), 104), // Fader 6: Distortion Level
            ("delay_level".to_string(), 108),      // Fader 7: Delay Level
            ("reverb_level".to_string(), 112),     // Fader 8: Reverb Level
        ];

        // Standard Knob Mappings (CC 16..23) -> Peak Modulation & Shaping
        let knob_cc_mappings = [
            ("filter_overdrive".to_string(), 80),  // Knob 1: Filter Overdrive
            ("filter_post_drive".to_string(), 36), // Knob 2: Filter Post Drive
            ("osc_1_shape".to_string(), 12),       // Knob 3: Osc 1 Shape
            ("osc_2_shape".to_string(), 39),       // Knob 4: Osc 2 Shape
            ("lfo_1_rate".to_string(), 30),        // Knob 5: LFO 1 Rate
            ("lfo_2_rate".to_string(), 31),        // Knob 6: LFO 2 Rate
            ("delay_time".to_string(), 109),       // Knob 7: Delay Time
            ("delay_feedback".to_string(), 110),   // Knob 8: Delay Feedback
        ];

        let fader_takeovers = [
            SoftTakeoverState::new(mode, 127.0),
            SoftTakeoverState::new(mode, 0.0),
            SoftTakeoverState::new(mode, 127.0),
            SoftTakeoverState::new(mode, 0.0),
            SoftTakeoverState::new(mode, 0.0),
            SoftTakeoverState::new(mode, 0.0),
            SoftTakeoverState::new(mode, 0.0),
            SoftTakeoverState::new(mode, 0.0),
        ];

        let knob_takeovers = [
            SoftTakeoverState::new(mode, 0.0),
            SoftTakeoverState::new(mode, 0.0),
            SoftTakeoverState::new(mode, 64.0),
            SoftTakeoverState::new(mode, 64.0),
            SoftTakeoverState::new(mode, 64.0),
            SoftTakeoverState::new(mode, 64.0),
            SoftTakeoverState::new(mode, 64.0),
            SoftTakeoverState::new(mode, 0.0),
        ];

        Self {
            fader_takeovers,
            knob_takeovers,
            solo_states: [false; 8],
            mute_states: [false; 8],
            rec_states: [false; 8],
            animate_1_active: false,
            animate_2_active: false,
            transport_states: HashMap::new(),
            fader_cc_mappings,
            knob_cc_mappings,
        }
    }

    /// Sets the soft takeover mode across all faders and knobs.
    pub fn set_takeover_mode(&mut self, mode: SoftTakeoverMode) {
        for t in &mut self.fader_takeovers {
            t.mode = mode;
            if mode == SoftTakeoverMode::Jump {
                t.is_caught = true;
            }
        }
        for t in &mut self.knob_takeovers {
            t.mode = mode;
            if mode == SoftTakeoverMode::Jump {
                t.is_caught = true;
            }
        }
    }

    /// Synchronizes soft takeover targets to match the parameter values of a patch.
    pub fn sync_to_patch(&mut self, patch: &SynthPatch) {
        for (i, (param_name, _)) in self.fader_cc_mappings.iter().enumerate() {
            let val = patch.get_param(param_name).unwrap_or(0.0);
            self.fader_takeovers[i].reset_target(val);
        }
        for (i, (param_name, _)) in self.knob_cc_mappings.iter().enumerate() {
            let val = patch.get_param(param_name).unwrap_or(0.0);
            self.knob_takeovers[i].reset_target(val);
        }
    }

    /// Generates a MIDI packet to set the LED state on a nanoKONTROL2 button.
    #[must_use]
    pub fn set_led(cc: u8, on: bool) -> MidiPacket {
        MidiPacket::ControlChange {
            channel: 0,
            cc,
            value: if on { 127 } else { 0 },
        }
    }

    /// Processes an incoming MIDI CC event from the nanoKONTROL2.
    pub fn process_midi(&mut self, cc: u8, value: u8) -> Option<NanoSurfaceAction> {
        let val_f64 = value as f64;

        // 1. Channel Sliders (CC 0..7)
        if cc <= 7 {
            let track = cc;
            let (param_key, target_cc) = self.fader_cc_mappings[track as usize].clone();
            let res = self.fader_takeovers[track as usize].update(val_f64);
            let effective = res.value;

            return Some(NanoSurfaceAction::ParamChange {
                track,
                is_slider: true,
                param_key,
                target_cc,
                raw_value: value,
                effective_value: effective,
                is_caught: res.is_caught,
                out_packet: MidiPacket::ControlChange {
                    channel: 0,
                    cc: target_cc,
                    value: (effective.clamp(0.0, 127.0).round() as u8) & 0x7F,
                },
            });
        }

        // 2. Channel Knobs (CC 16..23)
        if (16..=23).contains(&cc) {
            let track = cc - 16;
            let (param_key, target_cc) = self.knob_cc_mappings[track as usize].clone();
            let res = self.knob_takeovers[track as usize].update(val_f64);
            let effective = res.value;

            return Some(NanoSurfaceAction::ParamChange {
                track,
                is_slider: false,
                param_key,
                target_cc,
                raw_value: value,
                effective_value: effective,
                is_caught: res.is_caught,
                out_packet: MidiPacket::ControlChange {
                    channel: 0,
                    cc: target_cc,
                    value: (effective.clamp(0.0, 127.0).round() as u8) & 0x7F,
                },
            });
        }

        // 3. Solo Buttons (CC 32..39) -> Solo 1 and Solo 2 mapped to Animate 1 & 2
        if (32..=39).contains(&cc) {
            let track = cc - 32;
            let is_pressed = value >= 64;

            if track == 0 {
                // Solo 1 -> Animate 1 (CC 114)
                self.animate_1_active = is_pressed;
                return Some(NanoSurfaceAction::AnimateTrigger {
                    index: 1,
                    active: is_pressed,
                    out_packet: MidiPacket::ControlChange {
                        channel: 0,
                        cc: 114,
                        value: if is_pressed { 127 } else { 0 },
                    },
                    led_packet: Self::set_led(cc, is_pressed),
                });
            } else if track == 1 {
                // Solo 2 -> Animate 2 (CC 115)
                self.animate_2_active = is_pressed;
                return Some(NanoSurfaceAction::AnimateTrigger {
                    index: 2,
                    active: is_pressed,
                    out_packet: MidiPacket::ControlChange {
                        channel: 0,
                        cc: 115,
                        value: if is_pressed { 127 } else { 0 },
                    },
                    led_packet: Self::set_led(cc, is_pressed),
                });
            } else {
                // Solo 3..8 toggle
                if is_pressed {
                    self.solo_states[track as usize] = !self.solo_states[track as usize];
                }
                let active = self.solo_states[track as usize];
                return Some(NanoSurfaceAction::SoloToggle {
                    track,
                    active,
                    led_packet: Self::set_led(cc, active),
                });
            }
        }

        // 4. Mute Buttons (CC 48..55)
        if (48..=55).contains(&cc) {
            let track = cc - 48;
            if value >= 64 {
                self.mute_states[track as usize] = !self.mute_states[track as usize];
            }
            let active = self.mute_states[track as usize];
            return Some(NanoSurfaceAction::MuteToggle {
                track,
                active,
                led_packet: Self::set_led(cc, active),
            });
        }

        // 5. Rec Arm Buttons (CC 64..71)
        if (64..=71).contains(&cc) {
            let track = cc - 64;
            if value >= 64 {
                self.rec_states[track as usize] = !self.rec_states[track as usize];
            }
            let active = self.rec_states[track as usize];
            return Some(NanoSurfaceAction::RecToggle {
                track,
                active,
                led_packet: Self::set_led(cc, active),
            });
        }

        // 6. Transport Controls
        let transport_btn = match cc {
            58 => Some(NanoTransportButton::TrackPrev),
            59 => Some(NanoTransportButton::TrackNext),
            46 => Some(NanoTransportButton::Cycle),
            60 => Some(NanoTransportButton::MarkerSet),
            61 => Some(NanoTransportButton::MarkerPrev),
            62 => Some(NanoTransportButton::MarkerNext),
            43 => Some(NanoTransportButton::Rewind),
            44 => Some(NanoTransportButton::FastForward),
            42 => Some(NanoTransportButton::Stop),
            41 => Some(NanoTransportButton::Play),
            45 => Some(NanoTransportButton::Record),
            _ => None,
        };

        if let Some(btn) = transport_btn {
            let is_pressed = value >= 64;
            let led_packet = match btn {
                NanoTransportButton::Cycle
                | NanoTransportButton::Rewind
                | NanoTransportButton::FastForward
                | NanoTransportButton::Stop
                | NanoTransportButton::Play
                | NanoTransportButton::Record => Some(Self::set_led(cc, is_pressed)),
                _ => None,
            };

            return Some(NanoSurfaceAction::TransportAction {
                button: btn,
                is_pressed,
                led_packet,
            });
        }

        None
    }
}

// =============================================================================
// Unit Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_8_curated_factory_patches() {
        let patches = create_curated_peak_patches();
        assert_eq!(patches.len(), 8);

        let names: Vec<&str> = patches.iter().map(|p| p.meta.name.as_str()).collect();
        assert_eq!(
            names,
            vec![
                "Init Patch",
                "Oxford FPGA Pad",
                "Acid Overdrive Bass",
                "VSync Lead",
                "Ring Mod Bell",
                "Lush Chorus Strings",
                "Dub Delay Chord",
                "Animate Morph Arp",
            ]
        );

        // Verify categories and tags
        assert_eq!(patches[0].meta.category, "Init");
        assert_eq!(patches[1].meta.category, "Pad");
        assert_eq!(patches[2].meta.category, "Bass");
        assert_eq!(patches[3].meta.category, "Lead");
        assert_eq!(patches[4].meta.category, "Bell");
        assert_eq!(patches[5].meta.category, "Strings");
        assert_eq!(patches[6].meta.category, "Chord");
        assert_eq!(patches[7].meta.category, "Arp");

        assert!(patches[1].has_tag("ambient"));
        assert!(patches[2].has_tag("acid"));
        assert!(patches[3].has_tag("sync"));
        assert!(patches[4].has_tag("metallic"));
        assert!(patches[5].has_tag("chorus"));
        assert!(patches[6].has_tag("dub"));
        assert!(patches[7].has_tag("arp"));
    }

    #[test]
    fn test_512_patch_bank_crud_and_midi_selection() {
        let mut librarian = PeakLibrarian::new();
        assert_eq!(librarian.library.len(), 512);

        // Verify curated patches in Bank A 0..7
        let p0 = librarian.get_patch(0, 0).expect("Bank A 0");
        assert_eq!(p0.meta.name, "Init Patch");
        let p1 = librarian.get_patch(0, 1).expect("Bank A 1");
        assert_eq!(p1.meta.name, "Oxford FPGA Pad");

        // Verify Bank D Slot 127
        let p_d127 = librarian.get_patch(3, 127).expect("Bank D 127");
        assert_eq!(p_d127.meta.bank, "Bank D");
        assert_eq!(p_d127.meta.slot, 127);

        // Test MIDI patch selection CC 0, CC 32, Program Change
        let (msb, lsb, pc) = librarian.select_patch(2, 45); // Bank C, Slot 45
        assert_eq!(
            msb,
            MidiPacket::ControlChange {
                channel: 0,
                cc: 0,
                value: 0
            }
        );
        assert_eq!(
            lsb,
            MidiPacket::ControlChange {
                channel: 0,
                cc: 32,
                value: 2
            }
        );
        assert_eq!(
            pc,
            MidiPacket::ProgramChange {
                channel: 0,
                program: 45
            }
        );
        assert_eq!(librarian.current_bank, 2);
        assert_eq!(librarian.current_slot, 45);

        // Test mutating a patch
        let mut custom = SynthPatch::new("Cyber Lead");
        custom.set_param("filter_cutoff", 85.0);
        librarian.set_patch(2, 45, custom).expect("set patch");
        assert_eq!(librarian.active_patch().meta.name, "Cyber Lead");
        assert_eq!(librarian.active_patch().meta.bank, "Bank C");
        assert_eq!(librarian.active_patch().meta.slot, 45);

        // Test search
        let results = librarian.search("Cyber");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].meta.name, "Cyber Lead");
    }

    #[test]
    fn test_sysex_checksum_calculation_and_verification() {
        let payload = vec![0x10, 0x20, 0x30, 0x40];
        let checksum = calculate_peak_checksum(&payload);
        assert!(checksum <= 0x7F);
        assert!(verify_peak_checksum(&payload, checksum));
        assert!(verify_peak_checksum(&payload, UNIVERSAL_BYPASS_CHECKSUM));
        assert!(!verify_peak_checksum(&payload, (checksum + 1) & 0x7F));
    }

    #[test]
    fn test_sysex_single_patch_dump_codec() {
        let patch = create_curated_peak_patches()[1].clone(); // Oxford FPGA Pad
        let bytes = encode_single_patch_sysex(&patch).expect("encode sysex");
        assert_eq!(bytes[0], 0xF0);
        assert_eq!(&bytes[1..4], &NOVATION_MANUFACTURER_ID);
        assert_eq!(bytes[4], PEAK_SYSEX_DEVICE_ID);
        assert_eq!(bytes[5], PEAK_SYSEX_CMD_SOUND_DUMP);
        assert_eq!(*bytes.last().unwrap(), 0xF7);

        let decoded = decode_single_patch_sysex(&bytes).expect("decode sysex");
        assert_eq!(decoded.meta.name, "Oxford FPGA Pad");
        assert_eq!(decoded.meta.bank, "Bank A");
        assert_eq!(decoded.meta.slot, 1);
        assert!(decoded.sysex.raw_dump.is_some());
    }

    #[test]
    fn test_sysex_state_machine_streaming_and_bank_dump() {
        let mut state_machine = PeakSysExStateMachine::new();
        let curated = create_curated_peak_patches();

        // Feed single patch bytes
        let frame = encode_single_patch_sysex(&curated[2]).expect("encode");
        let events = state_machine.feed_buffer(&frame);
        assert_eq!(events.len(), 1);
        if let SysExEvent::PatchReceived(p) = &events[0] {
            assert!(p.meta.name.starts_with("Acid Overdrive"));
        } else {
            panic!("Expected PatchReceived event");
        }

        // Test checksum error detection
        let mut corrupt_frame = frame.clone();
        let len = corrupt_frame.len();
        corrupt_frame[len - 2] = (corrupt_frame[len - 2] + 1) & 0x7E; // invalidate checksum
        let err_events = state_machine.feed_buffer(&corrupt_frame);
        assert_eq!(err_events.len(), 1);
        assert!(matches!(err_events[0], SysExEvent::ChecksumError { .. }));
    }

    #[test]
    fn test_continuous_morph_fader() {
        let patches = create_curated_peak_patches();
        let p_a = patches[0].clone(); // Init Patch (cutoff 127)
        let p_b = patches[2].clone(); // Acid Overdrive Bass (cutoff 42)

        let mut morph = ContinuousMorphFader::new(p_a, p_b);
        assert_eq!(morph.factor(), 0.0);
        assert_eq!(morph.current_patch().get_param("filter_cutoff"), Some(127.0));

        // Morph halfway (0.5)
        let deltas = morph.set_factor(0.5);
        assert!(!deltas.is_empty());
        let cutoff_mid = morph.current_patch().get_param("filter_cutoff").unwrap();
        assert!((cutoff_mid - 84.5).abs() < 1.0);

        // Morph to 1.0 (100% Patch B)
        morph.set_factor(1.0);
        let cutoff_end = morph.current_patch().get_param("filter_cutoff").unwrap();
        assert!((cutoff_end - 42.0).abs() < 0.1);

        // Soft takeover catch testing
        morph.set_takeover_mode(SoftTakeoverMode::Catch);
        let (updated1, _) = morph.update_physical(0.2); // far from 1.0
        assert!(!updated1);
        let (updated2, _) = morph.update_physical(0.99); // crosses target
        assert!(updated2);
    }

    #[test]
    fn test_patch_diff_and_selective_merge() {
        let patches = create_curated_peak_patches();
        let p_a = &patches[0]; // Init
        let p_b = &patches[1]; // Oxford Pad

        let diff = diff_patches(p_a, p_b);
        assert!(!diff.identical);
        assert!(diff.total_differences() > 0);

        // Merge only chorus_level from Pad into Init
        let merged = merge_selected_params(p_a, p_b, &["chorus_level".to_string()]);
        assert_eq!(merged.get_param("chorus_level"), Some(70.0));
        assert_eq!(merged.get_param("filter_cutoff"), Some(127.0)); // original preserved
    }

    #[test]
    fn test_safe_randomizer_safety_caps_enforcement() {
        let mut randomizer = PeakSafeRandomizer::new();
        let mut patch = SynthPatch::new("High Danger Patch");

        // Manually place parameters at dangerous extremes
        patch.set_param("filter_post_drive", 127.0);
        patch.set_param("filter_resonance", 127.0);
        patch.set_param("filter_overdrive", 127.0);
        patch.set_param("distortion_level", 127.0);
        patch.set_param("delay_feedback", 127.0);
        patch.set_param("osc_1_shape", 50.0);

        // Apply safety caps directly
        PeakSafeRandomizer::apply_safety_caps(&mut patch);
        assert!(patch.get_param("filter_post_drive").unwrap() <= SAFE_CAP_FILTER_POST_DRIVE);
        assert!(patch.get_param("filter_resonance").unwrap() <= SAFE_CAP_FILTER_RESONANCE);
        assert!(patch.get_param("filter_overdrive").unwrap() <= SAFE_CAP_FILTER_OVERDRIVE);
        assert!(patch.get_param("distortion_level").unwrap() <= SAFE_CAP_DISTORTION_LEVEL);
        assert!(patch.get_param("delay_feedback").unwrap() <= SAFE_CAP_DELAY_FEEDBACK);

        // Global randomize with high amount and density
        let randomized = randomizer.randomize_global(&patch, 100.0, 100.0, Some(42));
        assert!(randomized.get_param("filter_post_drive").unwrap() <= SAFE_CAP_FILTER_POST_DRIVE);
        assert!(randomized.get_param("filter_resonance").unwrap() <= SAFE_CAP_FILTER_RESONANCE);
        assert!(randomized.get_param("filter_overdrive").unwrap() <= SAFE_CAP_FILTER_OVERDRIVE);
        assert!(randomized.get_param("distortion_level").unwrap() <= SAFE_CAP_DISTORTION_LEVEL);
        assert!(randomized.get_param("delay_feedback").unwrap() <= SAFE_CAP_DELAY_FEEDBACK);

        // Test section-scoped randomize
        let filter_rand = randomizer.randomize_section(
            &patch,
            PeakRandomizeSection::Filter,
            100.0,
            100.0,
            Some(99),
        );
        assert!(filter_rand.get_param("filter_resonance").unwrap() <= SAFE_CAP_FILTER_RESONANCE);

        // Test parameter locking
        randomizer.lock_param("osc_1_shape");
        let locked_rand = randomizer.randomize_global(&patch, 100.0, 100.0, Some(101));
        assert_eq!(locked_rand.get_param("osc_1_shape"), Some(50.0));
    }

    #[test]
    fn test_nanokontrol2_surface_faders_knobs_and_animate_triggers() {
        let mut surface = NanoKontrol2Surface::new(SoftTakeoverMode::Jump);

        // Fader 1 (CC 0) -> Filter Cutoff (CC 29)
        let action1 = surface.process_midi(0, 95).expect("fader 1");
        if let NanoSurfaceAction::ParamChange {
            track,
            is_slider,
            param_key,
            target_cc,
            effective_value,
            out_packet,
            ..
        } = action1
        {
            assert_eq!(track, 0);
            assert!(is_slider);
            assert_eq!(param_key, "filter_cutoff");
            assert_eq!(target_cc, 29);
            assert_eq!(effective_value, 95.0);
            assert_eq!(
                out_packet,
                MidiPacket::ControlChange {
                    channel: 0,
                    cc: 29,
                    value: 95
                }
            );
        } else {
            panic!("Expected ParamChange for Fader 1");
        }

        // Knob 1 (CC 16) -> Filter Overdrive (CC 80)
        let action2 = surface.process_midi(16, 40).expect("knob 1");
        if let NanoSurfaceAction::ParamChange {
            track,
            is_slider,
            param_key,
            target_cc,
            effective_value,
            ..
        } = action2
        {
            assert_eq!(track, 0);
            assert!(!is_slider);
            assert_eq!(param_key, "filter_overdrive");
            assert_eq!(target_cc, 80);
            assert_eq!(effective_value, 40.0);
        } else {
            panic!("Expected ParamChange for Knob 1");
        }

        // Solo 1 (CC 32) -> Animate 1 Trigger (CC 114)
        let action3 = surface.process_midi(32, 127).expect("solo 1 press");
        if let NanoSurfaceAction::AnimateTrigger {
            index,
            active,
            out_packet,
            led_packet,
        } = action3
        {
            assert_eq!(index, 1);
            assert!(active);
            assert_eq!(
                out_packet,
                MidiPacket::ControlChange {
                    channel: 0,
                    cc: 114,
                    value: 127
                }
            );
            assert_eq!(
                led_packet,
                MidiPacket::ControlChange {
                    channel: 0,
                    cc: 32,
                    value: 127
                }
            );
        } else {
            panic!("Expected AnimateTrigger for Solo 1");
        }

        // Solo 2 (CC 33) -> Animate 2 Trigger (CC 115)
        let action4 = surface.process_midi(33, 127).expect("solo 2 press");
        if let NanoSurfaceAction::AnimateTrigger { index, active, .. } = action4 {
            assert_eq!(index, 2);
            assert!(active);
        } else {
            panic!("Expected AnimateTrigger for Solo 2");
        }

        // Transport: Play (CC 41)
        let action5 = surface.process_midi(41, 127).expect("play");
        if let NanoSurfaceAction::TransportAction {
            button,
            is_pressed,
            led_packet,
        } = action5
        {
            assert_eq!(button, NanoTransportButton::Play);
            assert!(is_pressed);
            assert_eq!(
                led_packet,
                Some(MidiPacket::ControlChange {
                    channel: 0,
                    cc: 41,
                    value: 127
                })
            );
        } else {
            panic!("Expected TransportAction Play");
        }
    }
}
