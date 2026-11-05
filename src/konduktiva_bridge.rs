// SPDX-License-Identifier: GPL-3.0-or-later
//! `novation_peak_manager::konduktiva_bridge`: Konduktiva live-coding companion
//! engine, 8-player polyphonic voice allocation and dual-CC sweep sequencer bridge,
//! density graphs (`peakHold`, `peakBuild`), and live CC lane telemetry.

use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

use synth_midi_osc::routing::{safe_sweep_range, MidiPacket};

/// Canonical player names in the exact order created by `konduktiva-peak`.
pub const PEAK_PLAYER_NAMES: [&str; 8] = [
    "bass", "chords", "lead", "arp", "ccCutoff", "ccReso", "ccMod", "ccFx",
];

/// The 4 polyphonic note player names inside the 8-voice polyphony budget.
pub const NOTE_PLAYER_NAMES: [&str; 4] = ["bass", "chords", "lead", "arp"];

/// The 4 continuous CC sweep lane player names.
pub const CC_PLAYER_NAMES: [&str; 4] = ["ccCutoff", "ccReso", "ccMod", "ccFx"];

/// Default C natural-minor scale pool (semitone offsets from root C).
pub const DEFAULT_SCALE_POOL: [u8; 8] = [0, 2, 3, 5, 7, 8, 10, 12];

/// Canonical native rate for each player in onsets per beat.
pub const NATIVE_RATE_BASS: f64 = 0.5;
pub const NATIVE_RATE_CHORDS: f64 = 0.5;
pub const NATIVE_RATE_LEAD: f64 = 2.0;
pub const NATIVE_RATE_ARP: f64 = 4.0;
pub const NATIVE_RATE_CC_CUTOFF: f64 = 4.0;
pub const NATIVE_RATE_CC_RESO: f64 = 2.0;
pub const NATIVE_RATE_CC_MOD: f64 = 2.0;
pub const NATIVE_RATE_CC_FX: f64 = 1.0;

/// Triangle wave function over a given step count matching `sweeps.js` verbatim:
/// `1.0 - |(2 * i) / steps - 1.0|`.
#[inline]
#[must_use]
pub fn tri(step: usize, total_steps: usize) -> f64 {
    if total_steps <= 1 {
        return 0.0;
    }
    let norm = (2.0 * step as f64) / (total_steps as f64) - 1.0;
    1.0 - norm.abs()
}

/// Clamps a numeric value to a 7-bit MIDI integer `0..=127`.
#[inline]
#[must_use]
pub fn clamp7(v: f64) -> u8 {
    v.round().clamp(0.0, 127.0) as u8
}

/// Quantized CC sweep generator mirroring `sweepCC` in `sweeps.js`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SweepCc {
    pub controller: u8,
    pub lo: u8,
    pub hi: u8,
    pub cycle_beats: f64,
    pub steps: usize,
}

impl SweepCc {
    /// Creates a new `SweepCc` instance with automatic safety clamping.
    #[must_use]
    pub fn new(controller: u8, lo: u8, hi: u8, cycle_beats: f64, steps: usize) -> Self {
        let (safe_lo, safe_hi) = safe_sweep_range(controller);
        Self {
            controller,
            lo: lo.clamp(safe_lo, safe_hi),
            hi: hi.clamp(safe_lo, safe_hi),
            cycle_beats: cycle_beats.max(0.25),
            steps: steps.max(2),
        }
    }

    /// Evaluates the CC value at the given beat offset.
    #[must_use]
    pub fn lookup(&self, beat: f64) -> (u8, u8) {
        let cycle = self.cycle_beats;
        let wrapped = beat.rem_euclid(cycle);
        let progress = wrapped / cycle;
        let step = ((progress * self.steps as f64).floor() as usize).min(self.steps - 1);
        let t = tri(step, self.steps);
        let val = self.lo as f64 + (self.hi as f64 - self.lo as f64) * t;
        let clamped = clamp7(val);
        (self.controller, clamped)
    }
}

/// Quantized dual CC sweep generator mirroring `dualSweepCC` in `sweeps.js`.
///
/// Emits two CCs per onset on independent sweep trajectories with a 1/3-cycle phase offset.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DualSweepCc {
    pub controller_a: u8,
    pub lo_a: u8,
    pub hi_a: u8,
    pub controller_b: u8,
    pub lo_b: u8,
    pub hi_b: u8,
    pub cycle_beats: f64,
    pub steps: usize,
}

impl DualSweepCc {
    /// Creates a new `DualSweepCc` instance with automatic safety clamping on both controllers.
    #[must_use]
    pub fn new(
        controller_a: u8,
        lo_a: u8,
        hi_a: u8,
        controller_b: u8,
        lo_b: u8,
        hi_b: u8,
        cycle_beats: f64,
        steps: usize,
    ) -> Self {
        let (safe_lo_a, safe_hi_a) = safe_sweep_range(controller_a);
        let (safe_lo_b, safe_hi_b) = safe_sweep_range(controller_b);
        Self {
            controller_a,
            lo_a: lo_a.clamp(safe_lo_a, safe_hi_a),
            hi_a: hi_a.clamp(safe_lo_a, safe_hi_a),
            controller_b,
            lo_b: lo_b.clamp(safe_lo_b, safe_hi_b),
            hi_b: hi_b.clamp(safe_lo_b, safe_hi_b),
            cycle_beats: cycle_beats.max(0.25),
            steps: steps.max(3),
        }
    }

    /// Evaluates both CC values at the given beat offset.
    #[must_use]
    pub fn lookup(&self, beat: f64) -> ((u8, u8), (u8, u8)) {
        let cycle = self.cycle_beats;
        let wrapped = beat.rem_euclid(cycle);
        let progress = wrapped / cycle;
        let step_a = ((progress * self.steps as f64).floor() as usize).min(self.steps - 1);
        let step_b = (step_a + self.steps / 3) % self.steps;

        let t_a = tri(step_a, self.steps);
        let t_b = tri(step_b, self.steps);

        let val_a = self.lo_a as f64 + (self.hi_a as f64 - self.lo_a as f64) * t_a;
        let val_b = self.lo_b as f64 + (self.hi_b as f64 - self.lo_b as f64) * t_b;

        (
            (self.controller_a, clamp7(val_a)),
            (self.controller_b, clamp7(val_b)),
        )
    }
}

/// Inter-onset interval (IOI) rhythm preset templates.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum RhythmPreset {
    Half,
    Quarter,
    #[default]
    Eighth,
    Sixteenth,
}

impl RhythmPreset {
    /// Returns the inter-onset interval in beats.
    #[must_use]
    pub const fn interval_beats(&self) -> f64 {
        match self {
            Self::Half => 2.0,
            Self::Quarter => 1.0,
            Self::Eighth => 0.5,
            Self::Sixteenth => 0.25,
        }
    }

    /// Returns the corresponding onsets per beat.
    #[must_use]
    pub const fn onsets_per_beat(&self) -> f64 {
        match self {
            Self::Half => 0.5,
            Self::Quarter => 1.0,
            Self::Eighth => 2.0,
            Self::Sixteenth => 4.0,
        }
    }
}

/// Active density graph mode governing player rhythm rates.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum DensityGraphMode {
    /// `peakHold`: Sits each voice on its canonical native rung.
    #[default]
    PeakHold,
    /// `peakBuild`: 16-beat dynamic ramp doubling voice rates toward beat 8 and returning to 1x at beat 16.
    PeakBuild,
}

impl DensityGraphMode {
    /// Display name of the density graph mode.
    #[must_use]
    pub const fn display_name(&self) -> &'static str {
        match self {
            Self::PeakHold => "peakHold (Native Rungs)",
            Self::PeakBuild => "peakBuild (16-Beat Ramp)",
        }
    }

    /// Evaluates the rate multiplier for a given beat position in the 16-beat cycle.
    #[must_use]
    pub fn rate_multiplier(&self, beat: f64) -> f64 {
        match self {
            Self::PeakHold => 1.0,
            Self::PeakBuild => {
                let cycle = 16.0;
                let b = beat.rem_euclid(cycle);
                // Triangle build: 1.0 at b=0, 2.0 at b=8, 1.0 at b=16
                let ramp = if b <= 8.0 {
                    1.0 + (b / 8.0)
                } else {
                    1.0 + ((16.0 - b) / 8.0)
                };
                ramp.clamp(1.0, 2.0)
            }
        }
    }
}

/// Role classification for Konduktiva players.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PlayerRole {
    Note,
    ControlChange,
}

/// Internal state configuration for a single Konduktiva player.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KonduktivaPlayer {
    pub name: String,
    pub role: PlayerRole,
    pub enabled: bool,
    pub native_rate: f64,
    pub channel: u8,
    // Note voice specific fields
    pub octave: u8,
    pub velocities: Vec<u8>,
    pub duration_beats: f64,
    pub note_sequence: Vec<Vec<u8>>,
    // CC voice specific fields
    pub single_sweep: Option<SweepCc>,
    pub dual_sweep: Option<DualSweepCc>,
    // Dynamic runtime state
    pub event_count: u64,
    pub last_step_index: usize,
    pub next_onset_beat: f64,
    pub active_sounding_notes: Vec<u8>,
}

impl KonduktivaPlayer {
    /// Creates a note player matching the specifications in `players.js`.
    #[must_use]
    pub fn new_note_voice(
        name: impl Into<String>,
        native_rate: f64,
        octave: u8,
        velocities: Vec<u8>,
        duration_beats: f64,
        note_sequence: Vec<Vec<u8>>,
        channel: u8,
    ) -> Self {
        Self {
            name: name.into(),
            role: PlayerRole::Note,
            enabled: true,
            native_rate,
            channel,
            octave,
            velocities,
            duration_beats,
            note_sequence,
            single_sweep: None,
            dual_sweep: None,
            event_count: 0,
            last_step_index: 0,
            next_onset_beat: 0.0,
            active_sounding_notes: Vec::new(),
        }
    }

    /// Creates a single CC sweep lane player.
    #[must_use]
    pub fn new_cc_lane(
        name: impl Into<String>,
        native_rate: f64,
        sweep: SweepCc,
        channel: u8,
    ) -> Self {
        Self {
            name: name.into(),
            role: PlayerRole::ControlChange,
            enabled: true,
            native_rate,
            channel,
            octave: 0,
            velocities: Vec::new(),
            duration_beats: 0.0,
            note_sequence: Vec::new(),
            single_sweep: Some(sweep),
            dual_sweep: None,
            event_count: 0,
            last_step_index: 0,
            next_onset_beat: 0.0,
            active_sounding_notes: Vec::new(),
        }
    }

    /// Creates a dual CC sweep lane player.
    #[must_use]
    pub fn new_dual_cc_lane(
        name: impl Into<String>,
        native_rate: f64,
        dual_sweep: DualSweepCc,
        channel: u8,
    ) -> Self {
        Self {
            name: name.into(),
            role: PlayerRole::ControlChange,
            enabled: true,
            native_rate,
            channel,
            octave: 0,
            velocities: Vec::new(),
            duration_beats: 0.0,
            note_sequence: Vec::new(),
            single_sweep: None,
            dual_sweep: Some(dual_sweep),
            event_count: 0,
            last_step_index: 0,
            next_onset_beat: 0.0,
            active_sounding_notes: Vec::new(),
        }
    }
}

/// Real-time telemetry snapshot and sparkline history for a single CC sweep lane.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CcLaneTelemetry {
    pub name: String,
    pub controller_a: u8,
    pub controller_b: Option<u8>,
    pub current_a: u8,
    pub current_b: Option<u8>,
    pub min_seen_a: u8,
    pub max_seen_a: u8,
    pub safe_range_a: (u8, u8),
    pub safe_range_b: Option<(u8, u8)>,
    pub history_a: VecDeque<u8>,
    pub history_b: VecDeque<u8>,
    pub event_count: u64,
    pub last_update_beat: f64,
}

impl CcLaneTelemetry {
    /// Creates a new telemetry tracker for a single CC controller.
    #[must_use]
    pub fn new_single(name: impl Into<String>, controller: u8) -> Self {
        let range = safe_sweep_range(controller);
        Self {
            name: name.into(),
            controller_a: controller,
            controller_b: None,
            current_a: range.0,
            current_b: None,
            min_seen_a: range.0,
            max_seen_a: range.0,
            safe_range_a: range,
            safe_range_b: None,
            history_a: VecDeque::with_capacity(64),
            history_b: VecDeque::with_capacity(64),
            event_count: 0,
            last_update_beat: 0.0,
        }
    }

    /// Creates a new telemetry tracker for a dual CC pair.
    #[must_use]
    pub fn new_dual(name: impl Into<String>, controller_a: u8, controller_b: u8) -> Self {
        let range_a = safe_sweep_range(controller_a);
        let range_b = safe_sweep_range(controller_b);
        Self {
            name: name.into(),
            controller_a,
            controller_b: Some(controller_b),
            current_a: range_a.0,
            current_b: Some(range_b.0),
            min_seen_a: range_a.0,
            max_seen_a: range_a.0,
            safe_range_a: range_a,
            safe_range_b: Some(range_b),
            history_a: VecDeque::with_capacity(64),
            history_b: VecDeque::with_capacity(64),
            event_count: 0,
            last_update_beat: 0.0,
        }
    }

    /// Records a new incoming CC value, maintaining min/max and ring-buffer history.
    pub fn record_value_a(&mut self, val: u8, beat: f64) {
        self.current_a = val;
        self.min_seen_a = self.min_seen_a.min(val);
        self.max_seen_a = self.max_seen_a.max(val);
        if self.history_a.len() >= 64 {
            self.history_a.pop_front();
        }
        self.history_a.push_back(val);
        self.event_count = self.event_count.saturating_add(1);
        self.last_update_beat = beat;
    }

    /// Records a new incoming second CC value for dual-sweep lanes.
    pub fn record_value_b(&mut self, val: u8) {
        self.current_b = Some(val);
        if self.history_b.len() >= 64 {
            self.history_b.pop_front();
        }
        self.history_b.push_back(val);
    }
}

/// The complete Konduktiva live-coding companion bridge engine.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KonduktivaBridge {
    pub bpm: f64,
    pub playing: bool,
    pub current_beat: f64,
    pub loop_length_beats: f64,
    pub density_mode: DensityGraphMode,
    pub players: Vec<KonduktivaPlayer>,
    pub cc_telemetry: Vec<CcLaneTelemetry>,
    pub scale_pool: Vec<u8>,
    pub root_note: u8,
    #[serde(skip)]
    pub pending_note_offs: Vec<(f64, u8, u8)>, // (off_beat, channel, note)
}

impl Default for KonduktivaBridge {
    fn default() -> Self {
        Self::new()
    }
}

impl KonduktivaBridge {
    /// Creates and initializes a new `KonduktivaBridge` containing the canonical 8 Peak players.
    #[must_use]
    pub fn new() -> Self {
        let channel = 0; // Wire channel 0 = MIDI Channel 1
        let scale_pool = DEFAULT_SCALE_POOL.to_vec();

        // 1. Note Players (bass, chords, lead, arp)
        let bass = KonduktivaPlayer::new_note_voice(
            "bass",
            NATIVE_RATE_BASS,
            2,
            vec![98, 90],
            1.7,
            vec![
                vec![0],
                vec![7],
                vec![3],
                vec![5],
                vec![0],
                vec![10],
                vec![7],
                vec![3],
            ],
            channel,
        );

        let chords = KonduktivaPlayer::new_note_voice(
            "chords",
            NATIVE_RATE_CHORDS,
            4,
            vec![66, 60],
            1.7,
            vec![
                vec![0, 3, 7],
                vec![3, 7, 10],
                vec![5, 8, 12],
                vec![7, 10, 14],
                vec![0, 3, 7, 10],
            ],
            channel,
        );

        let lead_notes: Vec<Vec<u8>> = vec![
            vec![7],
            vec![10],
            vec![12],
            vec![10],
            vec![8],
            vec![7],
            vec![5],
            vec![7],
            vec![3],
            vec![5],
            vec![7],
            vec![12],
        ]
        .into_iter()
        .map(|v| vec![scale_pool[v[0] as usize % scale_pool.len()]])
        .collect();

        let lead = KonduktivaPlayer::new_note_voice(
            "lead",
            NATIVE_RATE_LEAD,
            5,
            vec![104, 88, 96, 84],
            0.45,
            lead_notes,
            channel,
        );

        let arp_notes: Vec<Vec<u8>> = vec![
            vec![0],
            vec![3],
            vec![7],
            vec![12],
            vec![7],
            vec![3],
            vec![10],
            vec![7],
            vec![5],
            vec![8],
            vec![12],
            vec![8],
        ]
        .into_iter()
        .map(|v| vec![scale_pool[v[0] as usize % scale_pool.len()]])
        .collect();

        let arp = KonduktivaPlayer::new_note_voice(
            "arp",
            NATIVE_RATE_ARP,
            5,
            vec![80, 64, 72, 60],
            0.2,
            arp_notes,
            channel,
        );

        // 2. CC Players (ccCutoff, ccReso, ccMod, ccFx)
        // CC 29 Cutoff: 28..118 over 8 beats in 32 steps
        let cc_cutoff = KonduktivaPlayer::new_cc_lane(
            "ccCutoff",
            NATIVE_RATE_CC_CUTOFF,
            SweepCc::new(29, 28, 118, 8.0, 32),
            channel,
        );

        // CC 79 Resonance: 6..62 (strictly clamped) over 6 beats in 24 steps
        let cc_reso = KonduktivaPlayer::new_cc_lane(
            "ccReso",
            NATIVE_RATE_CC_RESO,
            SweepCc::new(79, 6, 62, 6.0, 24),
            channel,
        );

        // CC 1 Mod Wheel: 0..100 over 4 beats in 16 steps
        let cc_mod = KonduktivaPlayer::new_cc_lane(
            "ccMod",
            NATIVE_RATE_CC_MOD,
            SweepCc::new(1, 0, 100, 4.0, 16),
            channel,
        );

        // Dual CC Fx: CC 112 Reverb (20..90) & CC 108 Delay (12..70) over 12 beats in 12 steps
        let cc_fx = KonduktivaPlayer::new_dual_cc_lane(
            "ccFx",
            NATIVE_RATE_CC_FX,
            DualSweepCc::new(112, 20, 90, 108, 12, 70, 12.0, 12),
            channel,
        );

        let players = vec![
            bass, chords, lead, arp, cc_cutoff, cc_reso, cc_mod, cc_fx,
        ];

        // 3. Telemetry trackers
        let telem_cutoff = CcLaneTelemetry::new_single("ccCutoff", 29);
        let telem_reso = CcLaneTelemetry::new_single("ccReso", 79);
        let telem_mod = CcLaneTelemetry::new_single("ccMod", 1);
        let telem_fx = CcLaneTelemetry::new_dual("ccFx", 112, 108);

        let cc_telemetry = vec![telem_cutoff, telem_reso, telem_mod, telem_fx];

        Self {
            bpm: 140.0,
            playing: false,
            current_beat: 0.0,
            loop_length_beats: 16.0,
            density_mode: DensityGraphMode::PeakHold,
            players,
            cc_telemetry,
            scale_pool,
            root_note: 60, // C4
            pending_note_offs: Vec::new(),
        }
    }

    /// Starts playback from current position.
    pub fn start(&mut self) {
        self.playing = true;
    }

    /// Stops playback.
    pub fn stop(&mut self) {
        self.playing = false;
    }

    /// Toggles playback between playing and stopped.
    pub fn toggle_playback(&mut self) {
        self.playing = !self.playing;
    }

    /// Resets playhead back to beat 0.0.
    pub fn reset_playhead(&mut self) {
        self.current_beat = 0.0;
        for p in &mut self.players {
            p.next_onset_beat = 0.0;
            p.last_step_index = 0;
            p.active_sounding_notes.clear();
        }
        self.pending_note_offs.clear();
    }

    /// Sets the active density graph mode (`PeakHold` vs `PeakBuild`).
    pub fn set_density_mode(&mut self, mode: DensityGraphMode) {
        self.density_mode = mode;
    }

    /// Toggles the enabled/muted state of a player by name.
    pub fn toggle_mute(&mut self, name: &str) -> bool {
        if let Some(player) = self.players.iter_mut().find(|p| p.name == name) {
            player.enabled = !player.enabled;
            player.enabled
        } else {
            false
        }
    }

    /// Advances the transport by seconds, advancing beats based on BPM.
    pub fn step_seconds(&mut self, dt_seconds: f64) -> Vec<MidiPacket> {
        if !self.playing || dt_seconds <= 0.0 {
            return Vec::new();
        }
        let delta_beats = dt_seconds * (self.bpm / 60.0);
        self.step_beats(delta_beats)
    }

    /// Advances the transport by a delta in beats and produces MIDI events.
    pub fn step_beats(&mut self, delta_beats: f64) -> Vec<MidiPacket> {
        if delta_beats <= 0.0 {
            return Vec::new();
        }

        let old_beat = self.current_beat;
        let new_beat = old_beat + delta_beats;
        self.current_beat = new_beat;

        let mut events = Vec::new();

        // 1. Process pending note-offs that expired between old_beat and new_beat
        let mut remaining_offs = Vec::new();
        for (off_beat, ch, note) in self.pending_note_offs.drain(..) {
            if off_beat <= new_beat {
                events.push(MidiPacket::NoteOff {
                    channel: ch,
                    note,
                    velocity: 0,
                });
            } else {
                remaining_offs.push((off_beat, ch, note));
            }
        }
        self.pending_note_offs = remaining_offs;

        // 2. Process player onsets
        let density_mult = self.density_mode.rate_multiplier(new_beat);

        for player in &mut self.players {
            if !player.enabled {
                continue;
            }

            let effective_rate = player.native_rate * density_mult;
            let ioi = 1.0 / effective_rate.max(0.125);

            // Trigger as many steps as occurred within this time slice
            while player.next_onset_beat <= new_beat {
                let onset_beat = player.next_onset_beat;
                let step_idx = player.last_step_index;

                match player.role {
                    PlayerRole::Note => {
                        if !player.note_sequence.is_empty() {
                            let pattern_idx = step_idx % player.note_sequence.len();
                            let vel_idx = if player.velocities.is_empty() {
                                80
                            } else {
                                player.velocities[step_idx % player.velocities.len()]
                            };

                            let offsets = &player.note_sequence[pattern_idx];
                            for &offset in offsets {
                                let midi_note = ((player.octave as i32 + 1) * 12 + offset as i32)
                                    .clamp(0, 127) as u8;
                                events.push(MidiPacket::NoteOn {
                                    channel: player.channel,
                                    note: midi_note,
                                    velocity: vel_idx,
                                });

                                let note_off_beat = onset_beat + player.duration_beats;
                                self.pending_note_offs.push((
                                    note_off_beat,
                                    player.channel,
                                    midi_note,
                                ));
                                player.active_sounding_notes.push(midi_note);
                            }
                        }
                    }
                    PlayerRole::ControlChange => {
                        if let Some(ref sweep) = player.single_sweep {
                            let (cc, val) = sweep.lookup(onset_beat);
                            events.push(MidiPacket::ControlChange {
                                channel: player.channel,
                                cc,
                                value: val,
                            });
                        } else if let Some(ref dual) = player.dual_sweep {
                            let ((cc_a, val_a), (cc_b, val_b)) = dual.lookup(onset_beat);
                            events.push(MidiPacket::ControlChange {
                                channel: player.channel,
                                cc: cc_a,
                                value: val_a,
                            });
                            events.push(MidiPacket::ControlChange {
                                channel: player.channel,
                                cc: cc_b,
                                value: val_b,
                            });
                        }
                    }
                }

                player.event_count = player.event_count.saturating_add(1);
                player.last_step_index = step_idx.wrapping_add(1);
                player.next_onset_beat += ioi;
            }
        }

        // 3. Update CC telemetry for UI sparklines and meters
        for evt in &events {
            if let MidiPacket::ControlChange { cc, value, .. } = *evt {
                for telem in &mut self.cc_telemetry {
                    if telem.controller_a == cc {
                        telem.record_value_a(value, new_beat);
                    } else if telem.controller_b == Some(cc) {
                        telem.record_value_b(value);
                    }
                }
            }
        }

        events
    }

    /// Emits emergency All-Notes-Off and Reset All Controllers MIDI packets.
    pub fn panic(&mut self) -> Vec<MidiPacket> {
        self.pending_note_offs.clear();
        for p in &mut self.players {
            p.active_sounding_notes.clear();
        }

        let channel = 0;
        vec![
            MidiPacket::ControlChange {
                channel,
                cc: 120, // All Sound Off
                value: 0,
            },
            MidiPacket::ControlChange {
                channel,
                cc: 123, // All Notes Off
                value: 0,
            },
            MidiPacket::ControlChange {
                channel,
                cc: 1, // Reset Mod Wheel
                value: 0,
            },
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_canonical_8_peak_players_initialization() {
        let bridge = KonduktivaBridge::new();
        assert_eq!(bridge.players.len(), 8);
        assert_eq!(bridge.cc_telemetry.len(), 4);

        for (i, expected_name) in PEAK_PLAYER_NAMES.iter().enumerate() {
            assert_eq!(&bridge.players[i].name, expected_name);
            assert!(bridge.players[i].enabled);
            assert_eq!(bridge.players[i].channel, 0); // MIDI Channel 1
        }

        // Check native rates
        assert_eq!(bridge.players[0].native_rate, NATIVE_RATE_BASS);
        assert_eq!(bridge.players[1].native_rate, NATIVE_RATE_CHORDS);
        assert_eq!(bridge.players[2].native_rate, NATIVE_RATE_LEAD);
        assert_eq!(bridge.players[3].native_rate, NATIVE_RATE_ARP);
        assert_eq!(bridge.players[4].native_rate, NATIVE_RATE_CC_CUTOFF);
        assert_eq!(bridge.players[5].native_rate, NATIVE_RATE_CC_RESO);
        assert_eq!(bridge.players[6].native_rate, NATIVE_RATE_CC_MOD);
        assert_eq!(bridge.players[7].native_rate, NATIVE_RATE_CC_FX);
    }

    #[test]
    fn test_tri_function_behavior() {
        assert_eq!(tri(0, 32), 0.0);
        assert_eq!(tri(16, 32), 1.0);
        assert_eq!(tri(32, 32), 0.0);
        assert!(tri(8, 32) > 0.49 && tri(8, 32) < 0.51);
    }

    #[test]
    fn test_sweep_cc_evaluation_and_safety_clamping() {
        // Safe resonance sweep (CC 79, max 62)
        let sweep = SweepCc::new(79, 6, 127, 8.0, 32);
        assert_eq!(sweep.controller, 79);
        assert_eq!(sweep.lo, 6);
        assert_eq!(sweep.hi, 62); // Clamped by safe_sweep_range

        let (cc, val_0) = sweep.lookup(0.0);
        assert_eq!(cc, 79);
        assert_eq!(val_0, 6);

        let (cc, val_mid) = sweep.lookup(4.0);
        assert_eq!(cc, 79);
        assert!(val_mid <= 62);
        assert!(val_mid >= 55);
    }

    #[test]
    fn test_dual_sweep_cc_evaluation() {
        let dual = DualSweepCc::new(112, 20, 90, 108, 12, 70, 12.0, 12);
        let ((ca, va), (cb, vb)) = dual.lookup(0.0);
        assert_eq!(ca, 112);
        assert_eq!(cb, 108);
        assert!(va >= 20 && va <= 90);
        assert!(vb >= 12 && vb <= 70);
    }

    #[test]
    fn test_density_graph_modes() {
        let hold = DensityGraphMode::PeakHold;
        assert_eq!(hold.rate_multiplier(0.0), 1.0);
        assert_eq!(hold.rate_multiplier(8.0), 1.0);
        assert_eq!(hold.rate_multiplier(16.0), 1.0);

        let build = DensityGraphMode::PeakBuild;
        assert_eq!(build.rate_multiplier(0.0), 1.0);
        assert_eq!(build.rate_multiplier(8.0), 2.0);
        assert_eq!(build.rate_multiplier(16.0), 1.0);
        assert_eq!(build.rate_multiplier(4.0), 1.5);
    }

    #[test]
    fn test_bridge_stepping_and_telemetry() {
        let mut bridge = KonduktivaBridge::new();
        bridge.start();

        // Step over 2 seconds at 140 BPM
        let events = bridge.step_seconds(2.0);
        assert!(!events.is_empty());

        let mut note_on_count = 0;
        let mut cc_count = 0;
        for evt in &events {
            match evt {
                MidiPacket::NoteOn { .. } => note_on_count += 1,
                MidiPacket::ControlChange { .. } => cc_count += 1,
                _ => {}
            }
        }

        assert!(note_on_count > 0);
        assert!(cc_count > 0);

        // Check telemetry
        for telem in &bridge.cc_telemetry {
            assert!(telem.event_count > 0);
            assert!(!telem.history_a.is_empty());
        }

        // Test panic
        let panic_pkts = bridge.panic();
        assert_eq!(panic_pkts.len(), 3);
        assert!(bridge.pending_note_offs.is_empty());
    }
}
