//! Novation Peak Modulation and Master Effects Engine.
//!
//! Models the complete modulation architecture and stereo digital effects chain:
//! - 3 ADSR envelopes (Amp, Mod 1, Mod 2) with deterministic curve generation
//! - Dual tempo-synced LFOs with 4 waveforms, fade envelopes, and sub-audio/audio ranges
//! - 16-slot dual-source Modulation Matrix (Source A x Source B -> Destination)
//! - 4-slot FX Modulation Matrix
//! - Animate 1 and 2 performance macro buttons with live parameter offset preview
//! - Arpeggiator with clock gate and polyphonic note tracking
//! - Pitch Glide (portamento) processor
//! - Master FX chain: Analogue Distortion, 3-mode Chorus, Sync Delay, and 3-type Reverb.

use serde::{Deserialize, Serialize};
use std::f64::consts::PI;
use synth_core::preset::SynthPatch;
use synth_ui_theme::widgets::EnvelopeCurveModel;

/// Default internal sample rate for modulation and FX evaluation in Hertz.
pub const MOD_FX_DEFAULT_SAMPLE_RATE: f64 = 48_000.0;

// =============================================================================
// 1. Envelope Stages, Curves, and 3x ADSR Engines
// =============================================================================

/// Operational stages for an ADSR envelope.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EnvelopeStage {
    /// Inactive or waiting for key trigger.
    Idle,
    /// Rising toward peak amplitude 1.0.
    Attack,
    /// Falling from peak 1.0 down to sustain level.
    Decay,
    /// Resting at sustained amplitude while gate is held.
    Sustain,
    /// Decaying toward 0.0 after gate release.
    Release,
}

/// A point along a deterministic ADSR envelope visualization curve.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct AdsrCurvePoint {
    /// Normalized time position in 0.0..=1.0.
    pub time_norm: f64,
    /// Instantaneous amplitude in 0.0..=1.0.
    pub amplitude: f64,
    /// Active envelope stage at this point.
    pub stage: EnvelopeStage,
}

/// Converts a 7-bit MIDI CC value (0..127) into an attack time in seconds (0.001s to 18.0s).
pub fn cc_to_attack_time(cc: f64) -> f64 {
    let norm = (cc / 127.0).clamp(0.0, 1.0);
    // Exponential response from 1ms up to 18s
    0.001 * (18000.0f64).powf(norm)
}

/// Converts a 7-bit MIDI CC value (0..127) into a decay time in seconds (0.002s to 24.0s).
pub fn cc_to_decay_time(cc: f64) -> f64 {
    let norm = (cc / 127.0).clamp(0.0, 1.0);
    0.002 * (12000.0f64).powf(norm)
}

/// Converts a 7-bit MIDI CC value (0..127) into a normalized sustain level in 0.0..=1.0.
pub fn cc_to_sustain_level(cc: f64) -> f64 {
    (cc / 127.0).clamp(0.0, 1.0)
}

/// Converts a 7-bit MIDI CC value (0..127) into a release time in seconds (0.002s to 24.0s).
pub fn cc_to_release_time(cc: f64) -> f64 {
    let norm = (cc / 127.0).clamp(0.0, 1.0);
    0.002 * (12000.0f64).powf(norm)
}

/// Generates a deterministic sequence of curve points for UI visualization and envelope preview.
pub fn generate_adsr_curve(
    attack_cc: f64,
    decay_cc: f64,
    sustain_cc: f64,
    release_cc: f64,
    num_points: usize,
) -> Vec<AdsrCurvePoint> {
    let count = num_points.max(16);
    let mut points = Vec::with_capacity(count);

    let a_time = cc_to_attack_time(attack_cc);
    let d_time = cc_to_decay_time(decay_cc);
    let s_level = cc_to_sustain_level(sustain_cc);
    let r_time = cc_to_release_time(release_cc);
    let hold_time = (a_time + d_time + r_time) * 0.25;

    let total_time = a_time + d_time + hold_time + r_time;
    let t_attack_end = a_time / total_time;
    let t_decay_end = (a_time + d_time) / total_time;
    let t_hold_end = (a_time + d_time + hold_time) / total_time;

    for i in 0..count {
        let t_norm = (i as f64) / ((count - 1) as f64);

        if t_norm <= t_attack_end {
            let seg_t = if t_attack_end > 1e-6 {
                t_norm / t_attack_end
            } else {
                1.0
            };
            // Exponential-like attack curve
            let amp = seg_t.powf(0.8).clamp(0.0, 1.0);
            points.push(AdsrCurvePoint {
                time_norm: t_norm,
                amplitude: amp,
                stage: EnvelopeStage::Attack,
            });
        } else if t_norm <= t_decay_end {
            let seg_t = (t_norm - t_attack_end) / (t_decay_end - t_attack_end).max(1e-6);
            // Exponential decay down to sustain level
            let amp = 1.0 - (1.0 - s_level) * (1.0 - (-3.0 * seg_t).exp()) / (1.0 - (-3.0f64).exp());
            points.push(AdsrCurvePoint {
                time_norm: t_norm,
                amplitude: amp.clamp(0.0, 1.0),
                stage: EnvelopeStage::Decay,
            });
        } else if t_norm <= t_hold_end {
            points.push(AdsrCurvePoint {
                time_norm: t_norm,
                amplitude: s_level,
                stage: EnvelopeStage::Sustain,
            });
        } else {
            let seg_t = (t_norm - t_hold_end) / (1.0 - t_hold_end).max(1e-6);
            // Exponential release down to zero
            let decay_factor = ((-3.0 * seg_t).exp() - (-3.0f64).exp()) / (1.0 - (-3.0f64).exp());
            let amp = s_level * decay_factor.max(0.0);
            points.push(AdsrCurvePoint {
                time_norm: t_norm,
                amplitude: amp.clamp(0.0, 1.0),
                stage: EnvelopeStage::Release,
            });
        }
    }

    points
}

/// Converts continuous controller values for an ADSR envelope to a shared `EnvelopeCurveModel`.
pub fn to_envelope_curve_model(
    attack_cc: f64,
    decay_cc: f64,
    sustain_cc: f64,
    release_cc: f64,
) -> EnvelopeCurveModel {
    EnvelopeCurveModel::new(
        (attack_cc / 127.0) as f32,
        (decay_cc / 127.0) as f32,
        (sustain_cc / 127.0) as f32,
        (release_cc / 127.0) as f32,
    )
    .with_curvatures(1.2, 1.5, 1.5)
}

/// Runtime ADSR envelope generator with exact stage tracking and exponential response.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AdsrEnvelope {
    /// Attack duration in seconds.
    pub attack_time: f64,
    /// Decay duration in seconds.
    pub decay_time: f64,
    /// Sustain amplitude level in 0.0..=1.0.
    pub sustain_level: f64,
    /// Release duration in seconds.
    pub release_time: f64,
    /// Current operational envelope stage.
    pub stage: EnvelopeStage,
    /// Current instantaneous amplitude in 0.0..=1.0.
    pub current_level: f64,
    /// Elapsed time in current stage in seconds.
    pub stage_time: f64,
    /// Level at start of release stage.
    pub release_start_level: f64,
    /// Sample rate in Hertz.
    pub sample_rate: f64,
}

impl Default for AdsrEnvelope {
    fn default() -> Self {
        Self::new(MOD_FX_DEFAULT_SAMPLE_RATE)
    }
}

impl AdsrEnvelope {
    /// Creates a new envelope generator initialized with standard default parameters.
    pub fn new(sample_rate: f64) -> Self {
        Self {
            attack_time: 0.01,
            decay_time: 0.50,
            sustain_level: 0.80,
            release_time: 0.30,
            stage: EnvelopeStage::Idle,
            current_level: 0.0,
            stage_time: 0.0,
            release_start_level: 0.0,
            sample_rate: sample_rate.max(1000.0),
        }
    }

    /// Creates an envelope initialized from standard 7-bit MIDI CC values.
    pub fn from_ccs(
        attack_cc: f64,
        decay_cc: f64,
        sustain_cc: f64,
        release_cc: f64,
        sample_rate: f64,
    ) -> Self {
        let mut env = Self::new(sample_rate);
        env.set_parameters_cc(attack_cc, decay_cc, sustain_cc, release_cc);
        env
    }

    /// Updates envelope parameters from 7-bit MIDI CC values.
    pub fn set_parameters_cc(
        &mut self,
        attack_cc: f64,
        decay_cc: f64,
        sustain_cc: f64,
        release_cc: f64,
    ) {
        self.attack_time = cc_to_attack_time(attack_cc);
        self.decay_time = cc_to_decay_time(decay_cc);
        self.sustain_level = cc_to_sustain_level(sustain_cc);
        self.release_time = cc_to_release_time(release_cc);
    }

    /// Triggers note-on gate.
    pub fn gate_on(&mut self) {
        self.stage = EnvelopeStage::Attack;
        self.stage_time = 0.0;
    }

    /// Triggers note-off gate.
    pub fn gate_off(&mut self) {
        if self.stage != EnvelopeStage::Idle {
            self.stage = EnvelopeStage::Release;
            self.release_start_level = self.current_level;
            self.stage_time = 0.0;
        }
    }

    /// Resets envelope to idle silence.
    pub fn reset(&mut self) {
        self.stage = EnvelopeStage::Idle;
        self.current_level = 0.0;
        self.stage_time = 0.0;
        self.release_start_level = 0.0;
    }

    /// Evaluates envelope by one sample step and returns the instantaneous amplitude.
    pub fn process_sample(&mut self) -> f64 {
        let dt = 1.0 / self.sample_rate;
        self.step(dt)
    }

    /// Evaluates envelope by an arbitrary time delta `dt` in seconds.
    pub fn step(&mut self, dt: f64) -> f64 {
        self.stage_time += dt;

        match self.stage {
            EnvelopeStage::Idle => {
                self.current_level = 0.0;
            }
            EnvelopeStage::Attack => {
                if self.attack_time <= 1e-4 {
                    self.current_level = 1.0;
                    self.stage = EnvelopeStage::Decay;
                    self.stage_time = 0.0;
                } else {
                    let progress = (self.stage_time / self.attack_time).clamp(0.0, 1.0);
                    // Convex attack slope
                    self.current_level = progress.powf(0.8);
                    if progress >= 1.0 {
                        self.current_level = 1.0;
                        self.stage = EnvelopeStage::Decay;
                        self.stage_time = 0.0;
                    }
                }
            }
            EnvelopeStage::Decay => {
                if self.decay_time <= 1e-4 {
                    self.current_level = self.sustain_level;
                    self.stage = EnvelopeStage::Sustain;
                    self.stage_time = 0.0;
                } else {
                    let progress = (self.stage_time / self.decay_time).clamp(0.0, 1.0);
                    // Exponential decay curve
                    let factor = (1.0 - (-3.0 * progress).exp()) / (1.0 - (-3.0f64).exp());
                    self.current_level = 1.0 - (1.0 - self.sustain_level) * factor;
                    if progress >= 1.0 {
                        self.current_level = self.sustain_level;
                        self.stage = EnvelopeStage::Sustain;
                        self.stage_time = 0.0;
                    }
                }
            }
            EnvelopeStage::Sustain => {
                self.current_level = self.sustain_level;
            }
            EnvelopeStage::Release => {
                if self.release_time <= 1e-4 {
                    self.current_level = 0.0;
                    self.stage = EnvelopeStage::Idle;
                    self.stage_time = 0.0;
                } else {
                    let progress = (self.stage_time / self.release_time).clamp(0.0, 1.0);
                    let factor = ((-3.0 * progress).exp() - (-3.0f64).exp()) / (1.0 - (-3.0f64).exp());
                    self.current_level = (self.release_start_level * factor).max(0.0);
                    if progress >= 1.0 || self.current_level < 1e-4 {
                        self.current_level = 0.0;
                        self.stage = EnvelopeStage::Idle;
                        self.stage_time = 0.0;
                    }
                }
            }
        }

        self.current_level
    }

    /// Returns whether the envelope is completely silent and idle.
    pub fn is_idle(&self) -> bool {
        self.stage == EnvelopeStage::Idle
    }
}

// =============================================================================
// 2. Dual Tempo-Synced LFOs
// =============================================================================

/// LFO periodic waveform shape options.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LfoWaveform {
    /// Smooth symmetrical triangle wave.
    Triangle,
    /// Descending sawtooth ramp.
    Sawtooth,
    /// Bipolar square wave (-1.0 or +1.0).
    Square,
    /// Stepped random sample and hold.
    SampleAndHold,
}

impl LfoWaveform {
    /// Derives LFO waveform from a continuous value or NRPN parameter.
    pub fn from_f64(val: f64) -> Self {
        match (val.round() as i32).clamp(0, 3) {
            0 => Self::Triangle,
            1 => Self::Sawtooth,
            2 => Self::Square,
            3 => Self::SampleAndHold,
            _ => Self::Triangle,
        }
    }
}

/// Operational frequency range for Peak LFOs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LfoRange {
    /// Sub-audio modulation rate (0.01 Hz to 20.0 Hz).
    Low,
    /// Audio-rate modulation for FM and metallic effects (1.0 Hz to 1600.0 Hz).
    High,
}

impl LfoRange {
    /// Selects LFO range from CC value (CC 83).
    pub fn from_cc(val: f64) -> Self {
        if val >= 64.0 {
            Self::High
        } else {
            Self::Low
        }
    }
}

/// Musical tempo-synced clock divisions for LFO rate synchronization.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LfoSyncDivision {
    Div32_1,
    Div16_1,
    Div8_1,
    Div4_1,
    Div2_1,
    Div1_1,
    Div1_2,
    Div1_2D,
    Div1_2T,
    Div1_4,
    Div1_4D,
    Div1_4T,
    Div1_8,
    Div1_8D,
    Div1_8T,
    Div1_16,
    Div1_16D,
    Div1_16T,
    Div1_32,
    Div1_32T,
}

impl LfoSyncDivision {
    /// Returns duration in quarter-note beats for this division.
    pub fn beats(&self) -> f64 {
        match self {
            Self::Div32_1 => 128.0,
            Self::Div16_1 => 64.0,
            Self::Div8_1 => 32.0,
            Self::Div4_1 => 16.0,
            Self::Div2_1 => 8.0,
            Self::Div1_1 => 4.0,
            Self::Div1_2 => 2.0,
            Self::Div1_2D => 3.0,
            Self::Div1_2T => 4.0 / 3.0,
            Self::Div1_4 => 1.0,
            Self::Div1_4D => 1.5,
            Self::Div1_4T => 2.0 / 3.0,
            Self::Div1_8 => 0.5,
            Self::Div1_8D => 0.75,
            Self::Div1_8T => 1.0 / 3.0,
            Self::Div1_16 => 0.25,
            Self::Div1_16D => 0.375,
            Self::Div1_16T => 1.0 / 6.0,
            Self::Div1_32 => 0.125,
            Self::Div1_32T => 1.0 / 12.0,
        }
    }

    /// Selects sync division from 7-bit MIDI CC value (CC 81 or CC 84).
    pub fn from_cc(cc: f64) -> Self {
        let idx = ((cc / 127.0) * 19.0).round() as usize;
        match idx.clamp(0, 19) {
            0 => Self::Div32_1,
            1 => Self::Div16_1,
            2 => Self::Div8_1,
            3 => Self::Div4_1,
            4 => Self::Div2_1,
            5 => Self::Div1_1,
            6 => Self::Div1_2,
            7 => Self::Div1_2D,
            8 => Self::Div1_2T,
            9 => Self::Div1_4,
            10 => Self::Div1_4D,
            11 => Self::Div1_4T,
            12 => Self::Div1_8,
            13 => Self::Div1_8D,
            14 => Self::Div1_8T,
            15 => Self::Div1_16,
            16 => Self::Div1_16D,
            17 => Self::Div1_16T,
            18 => Self::Div1_32,
            19 => Self::Div1_32T,
            _ => Self::Div1_4,
        }
    }
}

/// Runtime LFO processor supporting tempo synchronization, key trigger, and fade-in envelopes.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LfoProcessor {
    /// Selected periodic waveform.
    pub waveform: LfoWaveform,
    /// Frequency range mode.
    pub range: LfoRange,
    /// Rate in continuous controller units (0..127).
    pub rate_cc: f64,
    /// Sync rate in continuous controller units (0..127).
    pub sync_rate_cc: f64,
    /// Whether tempo sync is currently active.
    pub sync_enabled: bool,
    /// Fade-in duration setting in continuous controller units (0..127).
    pub fade_cc: f64,
    /// Instantaneous phase in [0.0, 1.0).
    pub phase: f64,
    /// Current evaluated output sample in [-1.0, 1.0].
    pub current_value: f64,
    /// Elapsed fade time since last key-on in seconds.
    pub fade_elapsed: f64,
    /// Sample and hold latch value.
    pub sh_value: f64,
    /// Internal PRNG state for sample and hold.
    pub prng_state: u64,
    /// Sample rate in Hertz.
    pub sample_rate: f64,
}

impl Default for LfoProcessor {
    fn default() -> Self {
        Self::new(LfoWaveform::Triangle, MOD_FX_DEFAULT_SAMPLE_RATE)
    }
}

impl LfoProcessor {
    /// Creates a new LFO processor with the specified waveform.
    pub fn new(waveform: LfoWaveform, sample_rate: f64) -> Self {
        Self {
            waveform,
            range: LfoRange::Low,
            rate_cc: 64.0,
            sync_rate_cc: 0.0,
            sync_enabled: false,
            fade_cc: 0.0,
            phase: 0.0,
            current_value: 0.0,
            fade_elapsed: 100.0, // Fully faded in by default
            sh_value: 0.0,
            prng_state: 0xDEAD_BEEF_CAFE_BABE,
            sample_rate: sample_rate.max(1000.0),
        }
    }

    /// Resets LFO phase to zero.
    pub fn reset_phase(&mut self) {
        self.phase = 0.0;
    }

    /// Triggers note-on event, restarting the fade envelope and key-sync phase if enabled.
    pub fn trigger_key_on(&mut self) {
        self.fade_elapsed = 0.0;
        self.phase = 0.0;
        self.step_sample_and_hold();
    }

    /// Internal helper to advance sample and hold value.
    fn step_sample_and_hold(&mut self) {
        let mut x = self.prng_state;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.prng_state = x;
        self.sh_value = ((x as f64) / (u64::MAX as f64)) * 2.0 - 1.0;
    }

    /// Computes effective frequency in Hertz based on rate, sync mode, range, and tempo.
    pub fn calculate_frequency_hz(&self, bpm: f64) -> f64 {
        if self.sync_enabled {
            let div = LfoSyncDivision::from_cc(self.sync_rate_cc);
            let beats = div.beats();
            let beats_per_sec = bpm.max(20.0) / 60.0;
            // Frequency = 1.0 / period_in_seconds
            let period_sec = beats / beats_per_sec;
            1.0 / period_sec.max(1e-4)
        } else {
            let norm = (self.rate_cc / 127.0).clamp(0.0, 1.0);
            match self.range {
                LfoRange::Low => {
                    // 0.01 Hz to 20 Hz exponential
                    0.01 * (2000.0f64).powf(norm)
                }
                LfoRange::High => {
                    // 1.0 Hz to 1600 Hz audio-rate exponential
                    1.0 * (1600.0f64).powf(norm)
                }
            }
        }
    }

    /// Advances LFO evaluation by time step `dt` and returns current output in [-1.0, 1.0].
    pub fn step(&mut self, dt: f64, bpm: f64) -> f64 {
        let freq_hz = self.calculate_frequency_hz(bpm);
        let old_phase = self.phase;
        self.phase = (self.phase + freq_hz * dt).fract();

        // Sample and hold triggers on phase cycle wrap
        if self.phase < old_phase {
            self.step_sample_and_hold();
        }

        // Generate raw waveform
        let raw = match self.waveform {
            LfoWaveform::Triangle => {
                if self.phase < 0.25 {
                    self.phase * 4.0
                } else if self.phase < 0.75 {
                    1.0 - (self.phase - 0.25) * 4.0
                } else {
                    -1.0 + (self.phase - 0.75) * 4.0
                }
            }
            LfoWaveform::Sawtooth => 1.0 - 2.0 * self.phase,
            LfoWaveform::Square => {
                if self.phase < 0.5 {
                    1.0
                } else {
                    -1.0
                }
            }
            LfoWaveform::SampleAndHold => self.sh_value,
        };

        // Apply fade envelope if configured
        self.fade_elapsed += dt;
        let fade_multiplier = if self.fade_cc > 1.0 {
            let fade_duration = 0.05 * (100.0f64).powf(self.fade_cc / 127.0);
            (self.fade_elapsed / fade_duration).clamp(0.0, 1.0)
        } else {
            1.0
        };

        self.current_value = raw * fade_multiplier;
        self.current_value
    }
}

// -----------------------------------------------------------------------------
// Direct LFO & Envelope Destination Depth Converters
// -----------------------------------------------------------------------------

/// Computes pitch modulation in semitones from an LFO value and bipolar depth (-64..+63).
pub fn calc_lfo_pitch_mod(lfo_val: f64, depth_bipolar: f64) -> f64 {
    // Up to +/- 12 semitones of vibrato/FM
    lfo_val * (depth_bipolar / 64.0) * 12.0
}

/// Computes wavetable shape offset from an LFO value and bipolar depth (-64..+63).
pub fn calc_lfo_shape_mod(lfo_val: f64, depth_bipolar: f64) -> f64 {
    // Up to +/- 64 shape offset
    lfo_val * (depth_bipolar / 64.0) * 64.0
}

/// Computes filter cutoff frequency shift in semitones from an LFO value and bipolar depth.
pub fn calc_lfo_filter_mod(lfo_val: f64, depth_bipolar: f64) -> f64 {
    // Up to +/- 36 semitones (3 octaves) of filter sweep
    lfo_val * (depth_bipolar / 64.0) * 36.0
}

// =============================================================================
// 3. 16-Slot Modulation Matrix & 4-Slot FX Modulation Matrix
// =============================================================================

/// Novation Peak modulation sources.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ModSource {
    Off,
    Direct,
    ModWheel,
    Aftertouch,
    PitchBend,
    Expression,
    Breath,
    Velocity,
    KeyTrack,
    Lfo1,
    Lfo2,
    AmpEnv,
    ModEnv1,
    ModEnv2,
    Animate1,
    Animate2,
}

impl ModSource {
    /// Maps from NRPN source index (0..15) to `ModSource`.
    pub fn from_nrpn(val: u8) -> Self {
        match val {
            0 => Self::Off,
            1 => Self::Direct,
            2 => Self::ModWheel,
            3 => Self::Aftertouch,
            4 => Self::PitchBend,
            5 => Self::Expression,
            6 => Self::Breath,
            7 => Self::Velocity,
            8 => Self::KeyTrack,
            9 => Self::Lfo1,
            10 => Self::Lfo2,
            11 => Self::AmpEnv,
            12 => Self::ModEnv1,
            13 => Self::ModEnv2,
            14 => Self::Animate1,
            15 => Self::Animate2,
            _ => Self::Off,
        }
    }

    /// Converts `ModSource` to NRPN index.
    pub fn to_nrpn(&self) -> u8 {
        match self {
            Self::Off => 0,
            Self::Direct => 1,
            Self::ModWheel => 2,
            Self::Aftertouch => 3,
            Self::PitchBend => 4,
            Self::Expression => 5,
            Self::Breath => 6,
            Self::Velocity => 7,
            Self::KeyTrack => 8,
            Self::Lfo1 => 9,
            Self::Lfo2 => 10,
            Self::AmpEnv => 11,
            Self::ModEnv1 => 12,
            Self::ModEnv2 => 13,
            Self::Animate1 => 14,
            Self::Animate2 => 15,
        }
    }
}

/// Novation Peak primary modulation matrix destinations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ModDestination {
    Off,
    Osc1Pitch,
    Osc2Pitch,
    Osc3Pitch,
    OscAllPitch,
    Osc1Shape,
    Osc2Shape,
    Osc3Shape,
    OscAllShape,
    Osc1Vsync,
    Osc2Vsync,
    Osc3Vsync,
    OscAllVsync,
    Osc1Mix,
    Osc2Mix,
    Osc3Mix,
    RingModMix,
    NoiseMix,
    FilterCutoff,
    FilterResonance,
    FilterOverdrive,
    FilterPostDrive,
    Lfo1Rate,
    Lfo2Rate,
    AmpEnvAttack,
    AmpEnvDecay,
    AmpEnvRelease,
    ModEnv1Attack,
    ModEnv1Decay,
    ModEnv1Release,
    ModEnv2Attack,
    ModEnv2Decay,
    ModEnv2Release,
}

impl ModDestination {
    /// Maps from NRPN destination index to `ModDestination`.
    pub fn from_nrpn(val: u8) -> Self {
        match val {
            0 => Self::Off,
            1 => Self::Osc1Pitch,
            2 => Self::Osc2Pitch,
            3 => Self::Osc3Pitch,
            4 => Self::OscAllPitch,
            5 => Self::Osc1Shape,
            6 => Self::Osc2Shape,
            7 => Self::Osc3Shape,
            8 => Self::OscAllShape,
            9 => Self::Osc1Vsync,
            10 => Self::Osc2Vsync,
            11 => Self::Osc3Vsync,
            12 => Self::OscAllVsync,
            13 => Self::Osc1Mix,
            14 => Self::Osc2Mix,
            15 => Self::Osc3Mix,
            16 => Self::RingModMix,
            17 => Self::NoiseMix,
            18 => Self::FilterCutoff,
            19 => Self::FilterResonance,
            20 => Self::FilterOverdrive,
            21 => Self::FilterPostDrive,
            22 => Self::Lfo1Rate,
            23 => Self::Lfo2Rate,
            24 => Self::AmpEnvAttack,
            25 => Self::AmpEnvDecay,
            26 => Self::AmpEnvRelease,
            27 => Self::ModEnv1Attack,
            28 => Self::ModEnv1Decay,
            29 => Self::ModEnv1Release,
            30 => Self::ModEnv2Attack,
            31 => Self::ModEnv2Decay,
            32 => Self::ModEnv2Release,
            _ => Self::Off,
        }
    }

    /// Converts destination to NRPN index.
    pub fn to_nrpn(&self) -> u8 {
        match self {
            Self::Off => 0,
            Self::Osc1Pitch => 1,
            Self::Osc2Pitch => 2,
            Self::Osc3Pitch => 3,
            Self::OscAllPitch => 4,
            Self::Osc1Shape => 5,
            Self::Osc2Shape => 6,
            Self::Osc3Shape => 7,
            Self::OscAllShape => 8,
            Self::Osc1Vsync => 9,
            Self::Osc2Vsync => 10,
            Self::Osc3Vsync => 11,
            Self::OscAllVsync => 12,
            Self::Osc1Mix => 13,
            Self::Osc2Mix => 14,
            Self::Osc3Mix => 15,
            Self::RingModMix => 16,
            Self::NoiseMix => 17,
            Self::FilterCutoff => 18,
            Self::FilterResonance => 19,
            Self::FilterOverdrive => 20,
            Self::FilterPostDrive => 21,
            Self::Lfo1Rate => 22,
            Self::Lfo2Rate => 23,
            Self::AmpEnvAttack => 24,
            Self::AmpEnvDecay => 25,
            Self::AmpEnvRelease => 26,
            Self::ModEnv1Attack => 27,
            Self::ModEnv1Decay => 28,
            Self::ModEnv1Release => 29,
            Self::ModEnv2Attack => 30,
            Self::ModEnv2Decay => 31,
            Self::ModEnv2Release => 32,
        }
    }
}

/// Instantaneous control and modulation values fed into matrix evaluation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModSourceValues {
    pub mod_wheel: f64,
    pub aftertouch: f64,
    pub pitch_bend: f64,
    pub expression: f64,
    pub breath: f64,
    pub velocity: f64,
    pub key_track: f64,
    pub lfo1: f64,
    pub lfo2: f64,
    pub amp_env: f64,
    pub mod_env1: f64,
    pub mod_env2: f64,
    pub animate1: f64,
    pub animate2: f64,
}

impl Default for ModSourceValues {
    fn default() -> Self {
        Self {
            mod_wheel: 0.0,
            aftertouch: 0.0,
            pitch_bend: 0.0,
            expression: 0.0,
            breath: 0.0,
            velocity: 0.8,
            key_track: 0.0,
            lfo1: 0.0,
            lfo2: 0.0,
            amp_env: 0.0,
            mod_env1: 0.0,
            mod_env2: 0.0,
            animate1: 0.0,
            animate2: 0.0,
        }
    }
}

impl ModSourceValues {
    /// Retrieves normalized value for the given modulation source.
    pub fn get(&self, source: ModSource) -> f64 {
        match source {
            ModSource::Off => 0.0,
            ModSource::Direct => 1.0,
            ModSource::ModWheel => self.mod_wheel,
            ModSource::Aftertouch => self.aftertouch,
            ModSource::PitchBend => self.pitch_bend,
            ModSource::Expression => self.expression,
            ModSource::Breath => self.breath,
            ModSource::Velocity => self.velocity,
            ModSource::KeyTrack => self.key_track,
            ModSource::Lfo1 => self.lfo1,
            ModSource::Lfo2 => self.lfo2,
            ModSource::AmpEnv => self.amp_env,
            ModSource::ModEnv1 => self.mod_env1,
            ModSource::ModEnv2 => self.mod_env2,
            ModSource::Animate1 => self.animate1,
            ModSource::Animate2 => self.animate2,
        }
    }
}

/// A single routing slot in the 16-slot Modulation Matrix.
/// Evaluates: `EffectiveMod = SourceA * SourceB * (depth / 64.0)`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModMatrixSlot {
    /// Primary modulation source.
    pub source_a: ModSource,
    /// Secondary scaling modulation source (or Direct for 1.0 constant).
    pub source_b: ModSource,
    /// Destination parameter receiving modulation.
    pub destination: ModDestination,
    /// Bipolar modulation depth in -64.0..=+63.0.
    pub depth: f64,
}

impl Default for ModMatrixSlot {
    fn default() -> Self {
        Self {
            source_a: ModSource::Off,
            source_b: ModSource::Direct,
            destination: ModDestination::Off,
            depth: 0.0,
        }
    }
}

impl ModMatrixSlot {
    /// Evaluates delta offset produced by this slot given current source values.
    pub fn evaluate(&self, sources: &ModSourceValues) -> f64 {
        if self.destination == ModDestination::Off || self.depth.abs() < 1e-6 {
            return 0.0;
        }

        let val_a = sources.get(self.source_a);
        let val_b = sources.get(self.source_b);
        val_a * val_b * (self.depth / 64.0)
    }
}

/// 16-Slot Novation Peak Modulation Matrix engine.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModMatrix {
    /// 16 modulation slots.
    pub slots: [ModMatrixSlot; 16],
}

impl Default for ModMatrix {
    fn default() -> Self {
        Self {
            slots: Default::default(),
        }
    }
}

impl ModMatrix {
    /// Creates a new empty modulation matrix.
    pub fn new() -> Self {
        Self::default()
    }

    /// Evaluates all 16 slots and accumulates net modulation delta for a specified destination.
    pub fn evaluate_destination(&self, dest: ModDestination, sources: &ModSourceValues) -> f64 {
        let mut sum = 0.0;
        for slot in &self.slots {
            if slot.destination == dest {
                sum += slot.evaluate(sources);
            }
        }
        sum
    }
}

// -----------------------------------------------------------------------------
// 4-Slot Dedicated FX Modulation Matrix
// -----------------------------------------------------------------------------

/// Destinations targeted by the 4-slot FX Modulation Matrix.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FxModDestination {
    Off,
    DistortionLevel,
    ChorusLevel,
    ChorusRate,
    DelayLevel,
    DelayTime,
    DelayFeedback,
    ReverbLevel,
    ReverbTime,
}

/// A routing slot in the 4-slot dedicated FX Modulation Matrix.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FxModMatrixSlot {
    /// Modulation source.
    pub source: ModSource,
    /// FX parameter destination.
    pub destination: FxModDestination,
    /// Bipolar modulation depth in -64.0..=+63.0.
    pub depth: f64,
}

impl Default for FxModMatrixSlot {
    fn default() -> Self {
        Self {
            source: ModSource::Off,
            destination: FxModDestination::Off,
            depth: 0.0,
        }
    }
}

impl FxModMatrixSlot {
    /// Evaluates delta offset for this FX slot.
    pub fn evaluate(&self, sources: &ModSourceValues) -> f64 {
        if self.destination == FxModDestination::Off || self.depth.abs() < 1e-6 {
            return 0.0;
        }
        let val = sources.get(self.source);
        val * (self.depth / 64.0)
    }
}

/// 4-Slot Dedicated Novation Peak FX Modulation Matrix.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FxModMatrix {
    /// 4 FX modulation slots.
    pub slots: [FxModMatrixSlot; 4],
}

impl Default for FxModMatrix {
    fn default() -> Self {
        Self {
            slots: Default::default(),
        }
    }
}

impl FxModMatrix {
    /// Evaluates all 4 FX slots and accumulates net delta for an FX destination.
    pub fn evaluate_destination(&self, dest: FxModDestination, sources: &ModSourceValues) -> f64 {
        let mut sum = 0.0;
        for slot in &self.slots {
            if slot.destination == dest {
                sum += slot.evaluate(sources);
            }
        }
        sum
    }
}

// =============================================================================
// 4. Animate 1 & 2 Performance Macro Buttons with Live Preview
// =============================================================================

/// Live parameter offset preview item showing baseline, animated delta, and effective value.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ParamOffsetPreview {
    /// Synthesizer parameter destination.
    pub destination: ModDestination,
    /// Baseline unmodulated parameter value.
    pub base_value: f64,
    /// Net live offset added by Animate 1 and 2.
    pub offset: f64,
    /// Resulting effective value.
    pub effective_value: f64,
}

/// Computes live parameter offset preview when Animate 1 and/or Animate 2 buttons are pressed.
pub fn calculate_animate_offsets(
    mod_matrix: &ModMatrix,
    fx_matrix: &FxModMatrix,
    patch: &SynthPatch,
    animate_1_pressed: bool,
    animate_2_pressed: bool,
) -> (Vec<ParamOffsetPreview>, Vec<(FxModDestination, f64, f64, f64)>) {
    let mut sources = ModSourceValues::default();
    sources.animate1 = if animate_1_pressed { 1.0 } else { 0.0 };
    sources.animate2 = if animate_2_pressed { 1.0 } else { 0.0 };

    let test_dests = [
        ModDestination::Osc1Pitch,
        ModDestination::Osc2Pitch,
        ModDestination::Osc3Pitch,
        ModDestination::OscAllPitch,
        ModDestination::Osc1Shape,
        ModDestination::Osc2Shape,
        ModDestination::Osc3Shape,
        ModDestination::FilterCutoff,
        ModDestination::FilterResonance,
        ModDestination::FilterOverdrive,
        ModDestination::FilterPostDrive,
        ModDestination::Lfo1Rate,
        ModDestination::Lfo2Rate,
    ];

    let mut param_previews = Vec::new();
    for &dest in &test_dests {
        let delta = mod_matrix.evaluate_destination(dest, &sources);
        if delta.abs() > 1e-4 {
            let base = match dest {
                ModDestination::FilterCutoff => patch.get_param("filter_cutoff").unwrap_or(100.0),
                ModDestination::FilterResonance => patch.get_param("filter_resonance").unwrap_or(20.0),
                ModDestination::Osc1Shape => patch.get_param("osc_1_shape").unwrap_or(64.0),
                ModDestination::Osc2Shape => patch.get_param("osc_2_shape").unwrap_or(64.0),
                ModDestination::Osc3Shape => patch.get_param("osc_3_shape").unwrap_or(64.0),
                _ => 0.0,
            };
            param_previews.push(ParamOffsetPreview {
                destination: dest,
                base_value: base,
                offset: delta,
                effective_value: base + delta * 64.0,
            });
        }
    }

    let fx_dests = [
        FxModDestination::DistortionLevel,
        FxModDestination::ChorusLevel,
        FxModDestination::DelayLevel,
        FxModDestination::ReverbLevel,
    ];

    let mut fx_previews = Vec::new();
    for &dest in &fx_dests {
        let delta = fx_matrix.evaluate_destination(dest, &sources);
        if delta.abs() > 1e-4 {
            let base = match dest {
                FxModDestination::DistortionLevel => patch.get_param("distortion_level").unwrap_or(0.0),
                FxModDestination::ChorusLevel => patch.get_param("chorus_level").unwrap_or(0.0),
                FxModDestination::DelayLevel => patch.get_param("delay_level").unwrap_or(0.0),
                FxModDestination::ReverbLevel => patch.get_param("reverb_level").unwrap_or(0.0),
                _ => 0.0,
            };
            fx_previews.push((dest, base, delta, base + delta * 64.0));
        }
    }

    (param_previews, fx_previews)
}

// =============================================================================
// 5. Arpeggiator & Glide (Portamento) Engine
// =============================================================================

/// Arpeggiator playback sequencing mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ArpMode {
    Up,
    Down,
    UpDown1,
    UpDown2,
    AsPlayed,
    Random,
    Chord,
}

/// Arpeggiator clock division rates.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ArpDivision {
    Div1_4,
    Div1_4T,
    Div1_8,
    Div1_8T,
    Div1_16,
    Div1_16T,
    Div1_32,
    Div1_32T,
}

impl ArpDivision {
    /// Duration in beats.
    pub fn beats(&self) -> f64 {
        match self {
            Self::Div1_4 => 1.0,
            Self::Div1_4T => 2.0 / 3.0,
            Self::Div1_8 => 0.5,
            Self::Div1_8T => 1.0 / 3.0,
            Self::Div1_16 => 0.25,
            Self::Div1_16T => 1.0 / 6.0,
            Self::Div1_32 => 0.125,
            Self::Div1_32T => 1.0 / 12.0,
        }
    }
}

/// Arpeggiator event emitted during real-time step evaluation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArpEvent {
    NoteOn { note: u8, velocity: u8 },
    NoteOff { note: u8 },
}

/// Real-time arpeggiator engine with clock gate duty-cycle control (CC 116).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ArpEngine {
    pub enabled: bool,
    pub mode: ArpMode,
    pub division: ArpDivision,
    pub octaves: u8,
    /// Gate length duty cycle (CC 116, 0..127).
    pub gate_clock_cc: u8,
    /// Currently held input MIDI notes.
    pub held_notes: Vec<u8>,
    /// Elapsed step time in seconds.
    pub step_timer: f64,
    /// Current step index in sequence.
    pub sequence_index: usize,
    /// Active sounding note.
    pub current_note: Option<u8>,
    /// Gate open status.
    pub gate_open: bool,
}

impl Default for ArpEngine {
    fn default() -> Self {
        Self {
            enabled: false,
            mode: ArpMode::Up,
            division: ArpDivision::Div1_16,
            octaves: 1,
            gate_clock_cc: 64, // ~50% gate duty cycle
            held_notes: Vec::new(),
            step_timer: 0.0,
            sequence_index: 0,
            current_note: None,
            gate_open: false,
        }
    }
}

impl ArpEngine {
    /// Adds a note to held notes buffer.
    pub fn note_on(&mut self, note: u8) {
        if !self.held_notes.contains(&note) {
            self.held_notes.push(note);
        }
    }

    /// Removes a note from held notes buffer.
    pub fn note_off(&mut self, note: u8) {
        self.held_notes.retain(|&n| n != note);
    }

    /// Clears all held notes.
    pub fn all_notes_off(&mut self) {
        self.held_notes.clear();
        self.current_note = None;
        self.gate_open = false;
    }

    /// Evaluates arpeggiator state by `dt` seconds and returns any generated NoteOn or NoteOff event.
    pub fn step(&mut self, dt: f64, bpm: f64) -> Option<ArpEvent> {
        if !self.enabled || self.held_notes.is_empty() {
            if let Some(note) = self.current_note.take() {
                self.gate_open = false;
                return Some(ArpEvent::NoteOff { note });
            }
            return None;
        }

        let step_duration = self.division.beats() * (60.0 / bpm.max(20.0));
        let gate_ratio = (self.gate_clock_cc as f64) / 127.0;
        let gate_duration = step_duration * gate_ratio.clamp(0.05, 0.98);

        self.step_timer += dt;

        // Check if gate needs to close before next step
        if self.gate_open && self.step_timer >= gate_duration {
            self.gate_open = false;
            if let Some(note) = self.current_note {
                return Some(ArpEvent::NoteOff { note });
            }
        }

        // Trigger next step
        if self.step_timer >= step_duration {
            self.step_timer -= step_duration;

            let mut sorted = self.held_notes.clone();
            sorted.sort_unstable();

            let total_steps = sorted.len() * (self.octaves as usize).max(1);
            let note_idx = self.sequence_index % sorted.len();
            let oct_idx = (self.sequence_index / sorted.len()) % (self.octaves as usize).max(1);

            let note = (sorted[note_idx] as usize + oct_idx * 12).min(127) as u8;

            self.sequence_index = (self.sequence_index + 1) % total_steps.max(1);
            self.current_note = Some(note);
            self.gate_open = true;

            return Some(ArpEvent::NoteOn { note, velocity: 100 });
        }

        None
    }
}

/// Real-time pitch portamento (glide) slewing processor.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GlideProcessor {
    /// Portamento time setting in 7-bit MIDI CC (CC 5).
    pub glide_time_cc: f64,
    /// Glide enabled switch (CC 35, 0..63 off, 64..127 on).
    pub enabled: bool,
    /// Instantaneous slewed pitch in semitones.
    pub current_semitone: f64,
    /// Target pitch in semitones.
    pub target_semitone: f64,
}

impl Default for GlideProcessor {
    fn default() -> Self {
        Self {
            glide_time_cc: 0.0,
            enabled: false,
            current_semitone: 60.0,
            target_semitone: 60.0,
        }
    }
}

impl GlideProcessor {
    /// Sets new target note pitch.
    pub fn set_target_note(&mut self, note: u8) {
        let target = note as f64;
        self.target_semitone = target;
        if !self.enabled || self.glide_time_cc < 1.0 {
            self.current_semitone = target;
        }
    }

    /// Evaluates glide pitch slewing over time delta `dt`.
    pub fn step(&mut self, dt: f64) -> f64 {
        if !self.enabled || self.glide_time_cc < 1.0 {
            self.current_semitone = self.target_semitone;
            return self.current_semitone;
        }

        // Exponential glide duration from 5ms to 5.0 seconds
        let duration = 0.005 * (1000.0f64).powf(self.glide_time_cc / 127.0);
        let alpha = 1.0 - (-dt / duration.max(1e-4)).exp();
        self.current_semitone += (self.target_semitone - self.current_semitone) * alpha;
        self.current_semitone
    }
}

// =============================================================================
// 6. Master FX Chain: Distortion, 3-Mode Chorus, Delay, 3-Type Reverb
// =============================================================================

/// Analogue distortion processor modeling the Peak post-filter distortion circuit.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AnalogueDistortion {
    /// Distortion level (CC 104, 0..127).
    pub level_cc: f64,
}

impl Default for AnalogueDistortion {
    fn default() -> Self {
        Self { level_cc: 0.0 }
    }
}

impl AnalogueDistortion {
    /// Processes a single audio sample through analogue soft-clipping saturation.
    pub fn process(&self, sample: f64) -> f64 {
        if self.level_cc <= 0.0 {
            return sample;
        }

        let drive = 1.0 + (self.level_cc / 127.0) * 8.0;
        let driven = sample * drive;
        // Hyperbolic tangent soft saturation with makeup gain scaling
        let saturated = driven.tanh();
        let makeup = 1.0 / (1.0 + (drive - 1.0) * 0.25);
        saturated * makeup
    }
}

/// Operating mode for the stereo chorus effect.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChorusMode {
    /// Classic subtle stereo chorus.
    Classic,
    /// Thick multi-voice ensemble.
    Ensemble,
    /// Flanger mode with comb filtering and feedback.
    Flanger,
}

/// Stereo modulated delay chorus processor.
#[derive(Debug, Clone, PartialEq)]
pub struct ChorusFx {
    pub mode: ChorusMode,
    pub level_cc: f64,    // CC 105
    pub feedback_cc: f64, // CC 107
    pub rate_cc: f64,     // CC 118
    buffer_left: Vec<f64>,
    buffer_right: Vec<f64>,
    write_idx: usize,
    lfo_phase: f64,
    sample_rate: f64,
}

impl Default for ChorusFx {
    fn default() -> Self {
        Self::new(MOD_FX_DEFAULT_SAMPLE_RATE)
    }
}

impl ChorusFx {
    pub fn new(sample_rate: f64) -> Self {
        let buf_size = 4096;
        Self {
            mode: ChorusMode::Classic,
            level_cc: 0.0,
            feedback_cc: 0.0,
            rate_cc: 64.0,
            buffer_left: vec![0.0; buf_size],
            buffer_right: vec![0.0; buf_size],
            write_idx: 0,
            lfo_phase: 0.0,
            sample_rate: sample_rate.max(1000.0),
        }
    }

    pub fn process(&mut self, in_l: f64, in_r: f64) -> (f64, f64) {
        if self.level_cc <= 0.0 {
            return (in_l, in_r);
        }

        let wet_mix = (self.level_cc / 127.0).clamp(0.0, 1.0);
        let feedback = (self.feedback_cc / 127.0).clamp(0.0, 0.85);

        // Rate in Hertz: 0.1 Hz to 5.0 Hz
        let rate_hz = 0.1 * (50.0f64).powf(self.rate_cc / 127.0);
        let dt = 1.0 / self.sample_rate;
        self.lfo_phase = (self.lfo_phase + rate_hz * dt).fract();

        // Quadrature stereo LFO (90 degree phase shift between channels)
        let lfo_l = (self.lfo_phase * 2.0 * PI).sin();
        let lfo_r = (self.lfo_phase * 2.0 * PI + 0.5 * PI).sin();

        let (base_delay_ms, mod_depth_ms) = match self.mode {
            ChorusMode::Classic => (15.0, 5.0),
            ChorusMode::Ensemble => (25.0, 10.0),
            ChorusMode::Flanger => (3.0, 2.5),
        };

        let delay_samples_l = ((base_delay_ms + mod_depth_ms * lfo_l) * 0.001 * self.sample_rate)
            .clamp(1.0, 3000.0);
        let delay_samples_r = ((base_delay_ms + mod_depth_ms * lfo_r) * 0.001 * self.sample_rate)
            .clamp(1.0, 3000.0);

        let buf_len = self.buffer_left.len();

        // Read interpolated left
        let read_pos_l = (self.write_idx as f64 - delay_samples_l + buf_len as f64) % (buf_len as f64);
        let idx_l = read_pos_l as usize % buf_len;
        let frac_l = read_pos_l.fract();
        let next_idx_l = (idx_l + 1) % buf_len;
        let wet_l = self.buffer_left[idx_l] * (1.0 - frac_l) + self.buffer_left[next_idx_l] * frac_l;

        // Read interpolated right
        let read_pos_r = (self.write_idx as f64 - delay_samples_r + buf_len as f64) % (buf_len as f64);
        let idx_r = read_pos_r as usize % buf_len;
        let frac_r = read_pos_r.fract();
        let next_idx_r = (idx_r + 1) % buf_len;
        let wet_r = self.buffer_right[idx_r] * (1.0 - frac_r) + self.buffer_right[next_idx_r] * frac_r;

        // Feedback and write
        self.buffer_left[self.write_idx] = in_l + wet_l * feedback;
        self.buffer_right[self.write_idx] = in_r + wet_r * feedback;
        self.write_idx = (self.write_idx + 1) % buf_len;

        let out_l = in_l * (1.0 - wet_mix * 0.5) + wet_l * wet_mix;
        let out_r = in_r * (1.0 - wet_mix * 0.5) + wet_r * wet_mix;
        (out_l, out_r)
    }
}

/// Tempo-synced stereo delay processor with lowpass damping.
#[derive(Debug, Clone, PartialEq)]
pub struct SyncDelayFx {
    pub level_cc: f64,    // CC 108
    pub time_cc: f64,     // CC 109
    pub feedback_cc: f64, // CC 110
    buffer_left: Vec<f64>,
    buffer_right: Vec<f64>,
    write_idx: usize,
    filter_state_l: f64,
    filter_state_r: f64,
    sample_rate: f64,
}

impl Default for SyncDelayFx {
    fn default() -> Self {
        Self::new(MOD_FX_DEFAULT_SAMPLE_RATE)
    }
}

impl SyncDelayFx {
    pub fn new(sample_rate: f64) -> Self {
        // 2 seconds buffer at 48kHz = 96,000 samples
        let max_samples = (sample_rate * 2.0).ceil() as usize;
        Self {
            level_cc: 0.0,
            time_cc: 64.0,
            feedback_cc: 0.0,
            buffer_left: vec![0.0; max_samples],
            buffer_right: vec![0.0; max_samples],
            write_idx: 0,
            filter_state_l: 0.0,
            filter_state_r: 0.0,
            sample_rate: sample_rate.max(1000.0),
        }
    }

    pub fn process(&mut self, in_l: f64, in_r: f64) -> (f64, f64) {
        if self.level_cc <= 0.0 {
            return (in_l, in_r);
        }

        let wet_mix = (self.level_cc / 127.0).clamp(0.0, 1.0);
        let feedback = (self.feedback_cc / 127.0).clamp(0.0, 0.90);

        // Delay time in seconds: 0.02s (20ms) to 1.5s
        let delay_sec = 0.02 + (self.time_cc / 127.0) * 1.48;
        let delay_samples = (delay_sec * self.sample_rate).clamp(10.0, (self.buffer_left.len() - 1) as f64);

        let buf_len = self.buffer_left.len();
        let read_idx = ((self.write_idx as f64 - delay_samples + buf_len as f64) as usize) % buf_len;

        let raw_delay_l = self.buffer_left[read_idx];
        let raw_delay_r = self.buffer_right[read_idx];

        // Lowpass damping filter in feedback path (~4.5 kHz)
        self.filter_state_l += (raw_delay_l - self.filter_state_l) * 0.45;
        self.filter_state_r += (raw_delay_r - self.filter_state_r) * 0.45;

        // Cross-feedback / ping-pong warmth
        self.buffer_left[self.write_idx] = in_l + self.filter_state_r * feedback;
        self.buffer_right[self.write_idx] = in_r + self.filter_state_l * feedback;
        self.write_idx = (self.write_idx + 1) % buf_len;

        let out_l = in_l + self.filter_state_l * wet_mix;
        let out_r = in_r + self.filter_state_r * wet_mix;
        (out_l, out_r)
    }
}

/// Operating room type for the algorithmic reverberator.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReverbType {
    /// Intimate acoustic room.
    Room,
    /// Medium-large concert hall.
    Hall,
    /// Massive arena / cathedral wash.
    Arena,
}

/// Algorithmic Schroeder reverberator with parallel feedback comb filters and all-pass diffusers.
#[derive(Debug, Clone, PartialEq)]
pub struct ReverbFx {
    pub reverb_type: ReverbType,
    pub level_cc: f64, // CC 112
    pub time_cc: f64,  // CC 113
    comb_buffers: Vec<Vec<f64>>,
    comb_indices: Vec<usize>,
    allpass_buffers: Vec<Vec<f64>>,
    allpass_indices: Vec<usize>,
    comb_damp: Vec<f64>,
}

impl Default for ReverbFx {
    fn default() -> Self {
        Self::new()
    }
}

impl ReverbFx {
    pub fn new() -> Self {
        // Prime delay lengths for comb filters
        let comb_lens = [1116, 1188, 1277, 1356, 1422, 1491, 1557, 1617];
        let mut comb_buffers = Vec::new();
        for &len in &comb_lens {
            comb_buffers.push(vec![0.0; len]);
        }

        // All-pass diffuser lengths
        let allpass_lens = [556, 441, 341, 225];
        let mut allpass_buffers = Vec::new();
        for &len in &allpass_lens {
            allpass_buffers.push(vec![0.0; len]);
        }

        Self {
            reverb_type: ReverbType::Hall,
            level_cc: 0.0,
            time_cc: 64.0,
            comb_buffers,
            comb_indices: vec![0; 8],
            allpass_buffers,
            allpass_indices: vec![0; 4],
            comb_damp: vec![0.0; 8],
        }
    }

    pub fn process(&mut self, in_l: f64, in_r: f64) -> (f64, f64) {
        if self.level_cc <= 0.0 {
            return (in_l, in_r);
        }

        let wet_mix = (self.level_cc / 127.0).clamp(0.0, 1.0);
        let decay_norm = (self.time_cc / 127.0).clamp(0.0, 1.0);

        let (feedback_base, damping) = match self.reverb_type {
            ReverbType::Room => (0.65 + decay_norm * 0.20, 0.40),
            ReverbType::Hall => (0.75 + decay_norm * 0.20, 0.25),
            ReverbType::Arena => (0.85 + decay_norm * 0.12, 0.15),
        };

        let input_mono = (in_l + in_r) * 0.5;
        let mut comb_out_l = 0.0;
        let mut comb_out_r = 0.0;

        for i in 0..8 {
            let buf_len = self.comb_buffers[i].len();
            let idx = self.comb_indices[i];
            let read_val = self.comb_buffers[i][idx];

            self.comb_damp[i] = read_val * (1.0 - damping) + self.comb_damp[i] * damping;
            self.comb_buffers[i][idx] = input_mono + self.comb_damp[i] * feedback_base;
            self.comb_indices[i] = (idx + 1) % buf_len;

            if i % 2 == 0 {
                comb_out_l += read_val;
            } else {
                comb_out_r += read_val;
            }
        }

        // Diffuse through all-pass stages
        let mut diff_l = comb_out_l * 0.25;
        let mut diff_r = comb_out_r * 0.25;

        for (ap_idx, ap_buf) in self.allpass_buffers.iter_mut().enumerate() {
            let buf_len = ap_buf.len();
            let idx = self.allpass_indices[ap_idx];
            let buf_out = ap_buf[idx];

            let input_val = if ap_idx % 2 == 0 { diff_l } else { diff_r };
            let new_val = input_val + buf_out * 0.5;
            ap_buf[idx] = new_val;
            self.allpass_indices[ap_idx] = (idx + 1) % buf_len;

            let diffused = -input_val + new_val * 0.5;
            if ap_idx % 2 == 0 {
                diff_l = diffused;
            } else {
                diff_r = diffused;
            }
        }

        let out_l = in_l + diff_l * wet_mix;
        let out_r = in_r + diff_r * wet_mix;
        (out_l, out_r)
    }
}

/// Complete Master Effects chain combining analogue distortion, chorus, delay, and reverb.
#[derive(Debug, Clone, PartialEq)]
pub struct MasterFxChain {
    pub distortion: AnalogueDistortion,
    pub chorus: ChorusFx,
    pub delay: SyncDelayFx,
    pub reverb: ReverbFx,
}

impl Default for MasterFxChain {
    fn default() -> Self {
        Self::new(MOD_FX_DEFAULT_SAMPLE_RATE)
    }
}

impl MasterFxChain {
    /// Creates a new Master FX chain at the given sample rate.
    pub fn new(sample_rate: f64) -> Self {
        Self {
            distortion: AnalogueDistortion::default(),
            chorus: ChorusFx::new(sample_rate),
            delay: SyncDelayFx::new(sample_rate),
            reverb: ReverbFx::new(),
        }
    }

    /// Updates all effect parameters from patch continuous controllers.
    pub fn update_from_patch(&mut self, patch: &SynthPatch) {
        self.distortion.level_cc = patch.get_param("distortion_level").unwrap_or(0.0);
        self.chorus.level_cc = patch.get_param("chorus_level").unwrap_or(0.0);
        self.chorus.feedback_cc = patch.get_param("chorus_feedback").unwrap_or(0.0);
        self.chorus.rate_cc = patch.get_param("chorus_rate").unwrap_or(64.0);
        self.delay.level_cc = patch.get_param("delay_level").unwrap_or(0.0);
        self.delay.time_cc = patch.get_param("delay_time").unwrap_or(64.0);
        self.delay.feedback_cc = patch.get_param("delay_feedback").unwrap_or(0.0);
        self.reverb.level_cc = patch.get_param("reverb_level").unwrap_or(0.0);
        self.reverb.time_cc = patch.get_param("reverb_time").unwrap_or(64.0);
    }

    /// Processes stereo audio through the complete master effect chain.
    pub fn process(&mut self, in_l: f64, in_r: f64) -> (f64, f64) {
        // 1. Post-filter Analogue Distortion
        let dist_l = self.distortion.process(in_l);
        let dist_r = self.distortion.process(in_r);

        // 2. Stereo 3-Mode Chorus
        let (cho_l, cho_r) = self.chorus.process(dist_l, dist_r);

        // 3. Tempo-Synced Delay
        let (del_l, del_r) = self.delay.process(cho_l, cho_r);

        // 4. Stereo Algorithmic Reverb
        let (rev_l, rev_r) = self.reverb.process(del_l, del_r);

        // Safety peak soft limiting
        (rev_l.clamp(-1.2, 1.2), rev_r.clamp(-1.2, 1.2))
    }
}

// =============================================================================
// 7. Unit Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_3_adsr_envelopes_parameters_and_curve_generation() {
        // Test deterministic curve generator for Amp envelope
        let curve = generate_adsr_curve(10.0, 50.0, 80.0, 40.0, 32);
        assert_eq!(curve.len(), 32);
        assert_eq!(curve[0].stage, EnvelopeStage::Attack);
        assert!(curve[0].amplitude <= 0.1);

        // Peak near attack end
        let max_amp = curve.iter().map(|p| p.amplitude).fold(0.0f64, f64::max);
        assert!(max_amp >= 0.95, "Envelope must reach peak near 1.0");

        // Convert to UI theme curve model
        let model = to_envelope_curve_model(10.0, 50.0, 80.0, 40.0);
        assert!((model.attack - (10.0 / 127.0) as f32).abs() < 1e-4);
        assert!((model.sustain - (80.0 / 127.0) as f32).abs() < 1e-4);
    }

    #[test]
    fn test_adsr_envelope_sample_processing_and_stages() {
        let mut env = AdsrEnvelope::from_ccs(0.0, 40.0, 64.0, 30.0, 48000.0);
        assert!(env.is_idle());

        // Note on
        env.gate_on();
        assert_eq!(env.stage, EnvelopeStage::Attack);

        // Since attack CC is 0 (1ms), step a few samples
        for _ in 0..100 {
            env.process_sample();
        }
        assert!(env.current_level > 0.8, "Level should have attacked");

        // Release note
        env.gate_off();
        assert_eq!(env.stage, EnvelopeStage::Release);

        // Run until release completes
        for _ in 0..100_000 {
            env.process_sample();
            if env.is_idle() {
                break;
            }
        }
        assert!(env.is_idle(), "Envelope should return to idle silence");
    }

    #[test]
    fn test_dual_tempo_synced_lfos_waveforms_and_fade() {
        let mut lfo1 = LfoProcessor::new(LfoWaveform::Triangle, 48000.0);
        lfo1.rate_cc = 64.0;
        lfo1.fade_cc = 0.0;

        let sample1 = lfo1.step(0.001, 120.0);
        assert!(sample1 >= -1.0 && sample1 <= 1.0);

        // Tempo-sync LFO 2 with 1/8 note division
        let mut lfo2 = LfoProcessor::new(LfoWaveform::Square, 48000.0);
        lfo2.sync_enabled = true;
        lfo2.sync_rate_cc = 70.0; // around 1/8 note
        let freq_sync = lfo2.calculate_frequency_hz(120.0);
        assert!(freq_sync > 0.5 && freq_sync < 20.0);

        // Test audio-range High mode
        lfo2.sync_enabled = false;
        lfo2.range = LfoRange::High;
        lfo2.rate_cc = 127.0;
        let freq_audio = lfo2.calculate_frequency_hz(120.0);
        assert!(freq_audio >= 1500.0, "High LFO range should reach audio rates");
    }

    #[test]
    fn test_lfo_depths_to_pitch_shape_and_filter() {
        let lfo_val = 1.0;

        // Bipolar depth at maximum +63
        let pitch_mod = calc_lfo_pitch_mod(lfo_val, 63.0);
        assert!((pitch_mod - 11.8).abs() < 0.5);

        let shape_mod = calc_lfo_shape_mod(lfo_val, 63.0);
        assert!((shape_mod - 63.0).abs() < 1.0);

        let filter_mod = calc_lfo_filter_mod(lfo_val, 63.0);
        assert!((filter_mod - 35.4).abs() < 1.0);
    }

    #[test]
    fn test_16_slot_dual_source_modulation_matrix() {
        let mut matrix = ModMatrix::new();

        // Slot 1: ModWheel x Direct -> FilterCutoff with +64 depth
        matrix.slots[0] = ModMatrixSlot {
            source_a: ModSource::ModWheel,
            source_b: ModSource::Direct,
            destination: ModDestination::FilterCutoff,
            depth: 64.0,
        };

        // Slot 2: Animate 1 x Lfo1 -> Osc1Pitch with -32 depth
        matrix.slots[1] = ModMatrixSlot {
            source_a: ModSource::Animate1,
            source_b: ModSource::Lfo1,
            destination: ModDestination::Osc1Pitch,
            depth: -32.0,
        };

        let mut sources = ModSourceValues::default();
        sources.mod_wheel = 0.5;
        sources.animate1 = 1.0;
        sources.lfo1 = 0.8;

        let cutoff_delta = matrix.evaluate_destination(ModDestination::FilterCutoff, &sources);
        assert!((cutoff_delta - 0.5).abs() < 1e-4);

        let pitch_delta = matrix.evaluate_destination(ModDestination::Osc1Pitch, &sources);
        // 1.0 * 0.8 * (-32 / 64) = -0.4
        assert!((pitch_delta - (-0.4)).abs() < 1e-4);
    }

    #[test]
    fn test_4_slot_fx_modulation_matrix() {
        let mut fx_matrix = FxModMatrix::default();

        // Slot 0: Expression -> ReverbLevel with +64 depth
        fx_matrix.slots[0] = FxModMatrixSlot {
            source: ModSource::Expression,
            destination: FxModDestination::ReverbLevel,
            depth: 64.0,
        };

        let mut sources = ModSourceValues::default();
        sources.expression = 0.75;

        let delta = fx_matrix.evaluate_destination(FxModDestination::ReverbLevel, &sources);
        assert!((delta - 0.75).abs() < 1e-4);
    }

    #[test]
    fn test_animate_1_and_2_macros_and_live_offset_preview() {
        let mut mod_matrix = ModMatrix::new();
        mod_matrix.slots[0] = ModMatrixSlot {
            source_a: ModSource::Animate1,
            source_b: ModSource::Direct,
            destination: ModDestination::FilterCutoff,
            depth: 32.0,
        };

        let fx_matrix = FxModMatrix::default();
        let patch = SynthPatch::new("Test");

        let (param_previews, _) = calculate_animate_offsets(&mod_matrix, &fx_matrix, &patch, true, false);
        assert_eq!(param_previews.len(), 1);
        assert_eq!(param_previews[0].destination, ModDestination::FilterCutoff);
        assert!((param_previews[0].offset - 0.5).abs() < 1e-4);
    }

    #[test]
    fn test_arpeggiator_clock_gate_and_modes() {
        let mut arp = ArpEngine::default();
        arp.enabled = true;
        arp.gate_clock_cc = 64;
        arp.note_on(60);
        arp.note_on(64);
        arp.note_on(67);

        // Step by 0.5 seconds at 120 BPM
        let mut note_events = 0;
        for _ in 0..100 {
            if let Some(_evt) = arp.step(0.01, 120.0) {
                note_events += 1;
            }
        }
        assert!(note_events > 0, "Arpeggiator must emit note events");
    }

    #[test]
    fn test_glide_portamento_slewing() {
        let mut glide = GlideProcessor::default();
        glide.enabled = true;
        glide.glide_time_cc = 64.0;
        glide.current_semitone = 60.0;
        glide.set_target_note(72);

        assert_eq!(glide.current_semitone, 60.0);
        let slewed = glide.step(0.05);
        assert!(slewed > 60.0 && slewed < 72.0, "Pitch must slew toward target");
    }

    #[test]
    fn test_master_fx_chain_distortion_chorus_delay_reverb() {
        let mut fx = MasterFxChain::new(48000.0);
        fx.distortion.level_cc = 64.0;
        fx.chorus.level_cc = 50.0;
        fx.delay.level_cc = 40.0;
        fx.reverb.level_cc = 60.0;

        let in_sample = 0.5;
        let (out_l, out_r) = fx.process(in_sample, in_sample);

        assert!(out_l.abs() <= 1.2 && out_r.abs() <= 1.2);
        assert!(!out_l.is_nan() && !out_r.is_nan());
    }
}
