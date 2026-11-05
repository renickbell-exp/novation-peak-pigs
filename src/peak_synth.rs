// SPDX-License-Identifier: GPL-3.0-or-later
//! Novation Peak 8-Voice FPGA NCO Oscillator, 17-Wavetable, Ring/Noise Mixer
//! and Analogue State-Variable Filter Domain Engine.
//!
//! Directly backed by `synth_core::SynthPatch` and `synth_core::CommandHistory`
//! without wrapper structs or duplicate undo stacks.

use serde::{Deserialize, Serialize};
use std::f64::consts::PI;
use synth_core::error::SynthError;
use synth_core::history::{BatchSetParamsCommand, CommandHistory, SetParamCommand};
use synth_core::preset::SynthPatch;
use synth_core::profiles::NovationPeakProfile;

/// Total number of polyphonic hardware voices on Novation Peak.
pub const PEAK_VOICE_COUNT: usize = 8;

/// Total Oxford FPGA NCO oscillators per voice.
pub const PEAK_OSC_COUNT: usize = 3;

/// Total factory wavetables available in the Oxford FPGA architecture.
pub const PEAK_WAVETABLE_COUNT: usize = 17;

/// Total discrete single-cycle waves per wavetable (morph points).
pub const PEAK_WAVES_PER_TABLE: usize = 5;

/// Number of deterministic spectral harmonic partials computed per wave.
pub const PEAK_HARMONIC_COUNT: usize = 32;

/// Default internal sample rate for synthesis evaluation in Hertz.
pub const PEAK_DEFAULT_SAMPLE_RATE: f64 = 48_000.0;

/// Array of all 17 factory wavetable names on Novation Peak.
pub const WAVETABLE_NAMES: [&str; 17] = NovationPeakProfile::WAVETABLES;

/// Continuous controller (CC) to canonical parameter key mapping.
pub fn cc_to_param_key(cc: u8) -> Option<&'static str> {
    match cc {
        1 => Some("mod_wheel"),
        3 => Some("osc_1_range"),
        5 => Some("glide_time"),
        9 => Some("osc_1_mod_2_pitch"),
        12 => Some("osc_1_shape"),
        14 => Some("osc_1_coarse"),
        15 => Some("osc_1_fine"),
        16 => Some("osc_1_lfo_2_pitch"),
        17 => Some("osc_2_coarse"),
        18 => Some("osc_2_fine"),
        19 => Some("osc_2_lfo_2_pitch"),
        20 => Some("osc_3_coarse"),
        21 => Some("osc_3_fine"),
        22 => Some("osc_3_lfo_2_pitch"),
        23 => Some("osc_1_mix"),
        24 => Some("osc_2_mix"),
        25 => Some("osc_3_mix"),
        26 => Some("ring_mod_mix"),
        27 => Some("noise_mix"),
        28 => Some("lfo_1_to_filter_depth"),
        29 => Some("filter_cutoff"),
        30 => Some("lfo_1_rate"),
        31 => Some("lfo_2_rate"),
        33 => Some("osc_1_lfo_1_shape"),
        34 => Some("osc_1_vsync"),
        35 => Some("glide_on"),
        36 => Some("filter_post_drive"),
        37 => Some("osc_2_range"),
        38 => Some("osc_2_mod_2_pitch"),
        39 => Some("osc_2_shape"),
        40 => Some("osc_2_mod_1_shape"),
        41 => Some("osc_2_lfo_1_shape"),
        42 => Some("osc_2_vsync"),
        43 => Some("osc_3_mod_2_pitch"),
        44 => Some("osc_3_vsync"),
        65 => Some("osc_3_range"),
        71 => Some("osc_3_shape"),
        72 => Some("osc_3_mod_1_pitch"),
        73 => Some("osc_3_lfo_1_shape"),
        75 => Some("filter_track"),
        76 => Some("osc_3_to_filter"),
        78 => Some("mod_1_to_filter"),
        79 => Some("filter_resonance"),
        80 => Some("filter_overdrive"),
        81 => Some("lfo_1_sync_rate"),
        82 => Some("lfo_1_fade"),
        83 => Some("lfo_2_range"),
        84 => Some("lfo_2_sync_rate"),
        85 => Some("lfo_2_fade"),
        86 => Some("amp_envelope_attack"),
        87 => Some("amp_envelope_decay"),
        88 => Some("amp_envelope_sustain"),
        89 => Some("amp_envelope_release"),
        90 => Some("mod_envelope_1_attack"),
        91 => Some("mod_envelope_1_decay"),
        92 => Some("mod_envelope_1_sustain"),
        93 => Some("mod_envelope_1_release"),
        94 => Some("mod_envelope_2_attack"),
        95 => Some("mod_envelope_2_decay"),
        103 => Some("mod_envelope_2_release"),
        104 => Some("distortion_level"),
        105 => Some("chorus_level"),
        107 => Some("chorus_feedback"),
        108 => Some("delay_level"),
        109 => Some("delay_time"),
        110 => Some("delay_feedback"),
        112 => Some("reverb_level"),
        113 => Some("reverb_time"),
        114 => Some("animate_1_hold"),
        115 => Some("animate_2_hold"),
        116 => Some("arp_clock_gate"),
        117 => Some("mod_envelope_2_sustain"),
        118 => Some("chorus_rate"),
        119 => Some("osc_1_mod_1_shape"),
        _ => None,
    }
}

/// Canonical parameter key to continuous controller (CC) number mapping.
pub fn param_key_to_cc(key: &str) -> Option<u8> {
    match key {
        "mod_wheel" => Some(1),
        "osc_1_range" => Some(3),
        "glide_time" => Some(5),
        "osc_1_mod_2_pitch" => Some(9),
        "osc_1_shape" => Some(12),
        "osc_1_coarse" => Some(14),
        "osc_1_fine" => Some(15),
        "osc_1_lfo_2_pitch" => Some(16),
        "osc_2_coarse" => Some(17),
        "osc_2_fine" => Some(18),
        "osc_2_lfo_2_pitch" => Some(19),
        "osc_3_coarse" => Some(20),
        "osc_3_fine" => Some(21),
        "osc_3_lfo_2_pitch" => Some(22),
        "osc_1_mix" => Some(23),
        "osc_2_mix" => Some(24),
        "osc_3_mix" => Some(25),
        "ring_mod_mix" => Some(26),
        "noise_mix" => Some(27),
        "lfo_1_to_filter_depth" => Some(28),
        "filter_cutoff" => Some(29),
        "lfo_1_rate" => Some(30),
        "lfo_2_rate" => Some(31),
        "osc_1_lfo_1_shape" => Some(33),
        "osc_1_vsync" => Some(34),
        "glide_on" => Some(35),
        "filter_post_drive" => Some(36),
        "osc_2_range" => Some(37),
        "osc_2_mod_2_pitch" => Some(38),
        "osc_2_shape" => Some(39),
        "osc_2_mod_1_shape" => Some(40),
        "osc_2_lfo_1_shape" => Some(41),
        "osc_2_vsync" => Some(42),
        "osc_3_mod_2_pitch" => Some(43),
        "osc_3_vsync" => Some(44),
        "osc_3_range" => Some(65),
        "osc_3_shape" => Some(71),
        "osc_3_mod_1_pitch" => Some(72),
        "osc_3_lfo_1_shape" => Some(73),
        "filter_track" => Some(75),
        "osc_3_to_filter" => Some(76),
        "mod_1_to_filter" => Some(78),
        "filter_resonance" => Some(79),
        "filter_overdrive" => Some(80),
        "lfo_1_sync_rate" => Some(81),
        "lfo_1_fade" => Some(82),
        "lfo_2_range" => Some(83),
        "lfo_2_sync_rate" => Some(84),
        "lfo_2_fade" => Some(85),
        "amp_envelope_attack" => Some(86),
        "amp_envelope_decay" => Some(87),
        "amp_envelope_sustain" => Some(88),
        "amp_envelope_release" => Some(89),
        "mod_envelope_1_attack" => Some(90),
        "mod_envelope_1_decay" => Some(91),
        "mod_envelope_1_sustain" => Some(92),
        "mod_envelope_1_release" => Some(93),
        "mod_envelope_2_attack" => Some(94),
        "mod_envelope_2_decay" => Some(95),
        "mod_envelope_2_release" => Some(103),
        "distortion_level" => Some(104),
        "chorus_level" => Some(105),
        "chorus_feedback" => Some(107),
        "delay_level" => Some(108),
        "delay_time" => Some(109),
        "delay_feedback" => Some(110),
        "reverb_level" => Some(112),
        "reverb_time" => Some(113),
        "animate_1_hold" => Some(114),
        "animate_2_hold" => Some(115),
        "arp_clock_gate" => Some(116),
        "mod_envelope_2_sustain" => Some(117),
        "chorus_rate" => Some(118),
        "osc_1_mod_1_shape" => Some(119),
        _ => None,
    }
}

/// Helper converting range control value (0..3 or 0..127) to octave shift integer.
/// Footing standard: 0 = 16' (-1 oct), 1 = 8' (0 oct), 2 = 4' (+1 oct), 3 = 2' (+2 oct).
pub fn range_to_octave_shift(range_val: f64) -> i32 {
    let step = if range_val <= 3.0 {
        range_val.round() as i32
    } else {
        ((range_val / 32.0).floor() as i32).clamp(0, 3)
    };
    step - 1
}

// =============================================================================
// 1. Wavetable Engine & Deterministic Harmonic Spectrum Calculator
// =============================================================================

/// Harmonic spectrum descriptor representing the first 32 partials of a waveform.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HarmonicSpectrum {
    /// Wavetable index (0..16).
    pub table_index: usize,
    /// Continuous shape parameter value (0.0..127.0).
    pub shape: f64,
    /// 32 harmonic amplitudes (normalized, partials 1 through 32).
    pub harmonics: [f64; PEAK_HARMONIC_COUNT],
}

impl HarmonicSpectrum {
    /// Calculates spectral centroid (weighted harmonic average index).
    pub fn spectral_centroid(&self) -> f64 {
        let sum_amp: f64 = self.harmonics.iter().map(|a| a.abs()).sum();
        if sum_amp <= 1e-12 {
            return 1.0;
        }
        let weighted: f64 = self
            .harmonics
            .iter()
            .enumerate()
            .map(|(i, &a)| (i + 1) as f64 * a.abs())
            .sum();
        weighted / sum_amp
    }

    /// Calculates total energy (sum of squared harmonic amplitudes).
    pub fn total_energy(&self) -> f64 {
        self.harmonics.iter().map(|&a| a * a).sum()
    }

    /// Returns the strongest harmonic partial and its amplitude.
    pub fn peak_harmonic(&self) -> (usize, f64) {
        let mut best_idx = 0;
        let mut best_val = -1.0;
        for (i, &a) in self.harmonics.iter().enumerate() {
            if a > best_val {
                best_val = a;
                best_idx = i + 1;
            }
        }
        (best_idx, best_val.max(0.0))
    }
}

/// Computes the deterministic harmonic signature for wave index `w` (0..4)
/// within the specified wavetable index `t` (0..16).
pub fn wavetable_wave_harmonics(table_idx: usize, wave_idx: usize) -> [f64; PEAK_HARMONIC_COUNT] {
    let t = table_idx % PEAK_WAVETABLE_COUNT;
    let w = wave_idx.min(PEAK_WAVES_PER_TABLE - 1);
    let mut h = [0.0; PEAK_HARMONIC_COUNT];

    match t {
        // 0: Sine-Tri (Pure sine morphing to rounded and sharp triangle)
        0 => {
            h[0] = 1.0;
            let exponent = 2.4 - (w as f64) * 0.2;
            let tri_mix = (w as f64) / 4.0;
            for i in 1..PEAK_HARMONIC_COUNT {
                let n = (i + 1) as f64;
                if (i + 1) % 2 != 0 {
                    let sign = if ((i + 1 - 1) / 2) % 2 == 0 {
                        1.0
                    } else {
                        -1.0
                    };
                    h[i] = tri_mix * (sign / n.powf(exponent));
                }
            }
        }

        // 1: Sine-Saw (Pure sine morphing to warm, standard, and bright sawtooth)
        1 => {
            let saw_exponent = 2.0 - (w as f64) * 0.28;
            for i in 0..PEAK_HARMONIC_COUNT {
                let n = (i + 1) as f64;
                if i == 0 {
                    h[i] = 1.0;
                } else {
                    let saw_mix = (w as f64) / 4.0;
                    h[i] = saw_mix * (1.0 / n.powf(saw_exponent));
                }
            }
        }

        // 2: Classic (Sawtooth -> Square -> 30% Pulse -> 15% Pulse -> Multi-wave)
        2 => {
            let pulse_duty = match w {
                0 => 0.50, // Sawtooth
                1 => 0.50, // Square
                2 => 0.35, // Pulse 35%
                3 => 0.20, // Pulse 20%
                _ => 0.10, // Pulse 10%
            };
            if w == 0 {
                for i in 0..PEAK_HARMONIC_COUNT {
                    h[i] = 1.0 / ((i + 1) as f64);
                }
            } else {
                for i in 0..PEAK_HARMONIC_COUNT {
                    let n = (i + 1) as f64;
                    h[i] = ((2.0 / (n * PI)) * (n * PI * pulse_duty).sin()).abs();
                }
            }
        }

        // 3: Saw-Pulse (Sawtooth smoothly morphing into pulse widths)
        3 => {
            let pw = 0.50 - (w as f64) * 0.08;
            for i in 0..PEAK_HARMONIC_COUNT {
                let n = (i + 1) as f64;
                let saw_part = 1.0 / n;
                let pulse_part = ((2.0 / (n * PI)) * (n * PI * pw).sin()).abs();
                let blend = (w as f64) / 4.0;
                h[i] = (1.0 - blend) * saw_part + blend * pulse_part;
            }
        }

        // 4: Harmonic 1 (Natural harmonic series with progressive overtone boost)
        4 => {
            let active_partials = 2 + w * 4;
            for i in 0..PEAK_HARMONIC_COUNT {
                if i < active_partials {
                    let n = (i + 1) as f64;
                    h[i] = 1.0 / (1.0 + 0.18 * n);
                }
            }
        }

        // 5: Harmonic 2 (Octaves, fifths, and high bell partials)
        5 => {
            let octaves = [1, 2, 3, 4, 6, 8, 12, 16, 24, 32];
            for &partial in &octaves {
                if partial <= PEAK_HARMONIC_COUNT {
                    let decay = 1.0 / (1.0 + (partial as f64) * 0.1);
                    let boost = 1.0 + (w as f64) * 0.25;
                    h[partial - 1] = decay * boost;
                }
            }
        }

        // 6: Chebyshev (Chebyshev polynomial harmonic distortion stages)
        6 => {
            h[0] = 0.8;
            let target_partial = (w + 1).min(PEAK_HARMONIC_COUNT);
            h[target_partial - 1] = 1.0;
            if target_partial + 1 <= PEAK_HARMONIC_COUNT {
                h[target_partial] = 0.45;
            }
            if target_partial + 2 <= PEAK_HARMONIC_COUNT {
                h[target_partial + 1] = 0.25;
            }
        }

        // 7: Double (Dual beating partials simulating detuned twin oscillators)
        7 => {
            for i in 0..PEAK_HARMONIC_COUNT {
                let n = (i + 1) as f64;
                let phase_beat = (n * (w as f64 + 1.0) * 0.45).cos().abs();
                h[i] = (1.0 / n.powf(1.1)) * (0.5 + 0.5 * phase_beat);
            }
        }

        // 8: Metal (Inharmonic metallic bell spectra with prime partial peaks)
        8 => {
            let primes = [1, 3, 7, 11, 13, 17, 19, 23, 29, 31];
            for (idx, &p) in primes.iter().enumerate() {
                if p <= PEAK_HARMONIC_COUNT {
                    let spread = 1.0 + (w as f64) * 0.3 * (idx as f64);
                    h[p - 1] = (1.0 / (idx as f64 + 1.0).powf(0.85)) * (spread % 1.2 + 0.4);
                }
            }
        }

        // 9: Resonant 1 (Mid-frequency resonant formant peak sweep)
        9 => {
            let center_partial = 3.0 + (w as f64) * 2.5;
            for i in 0..PEAK_HARMONIC_COUNT {
                let n = (i + 1) as f64;
                let dist = (n - center_partial).abs();
                let resonance = (-dist * dist / 3.5).exp();
                h[i] = (1.0 / n.powf(1.2)) + 1.5 * resonance;
            }
        }

        // 10: Resonant 2 (High-frequency resonance peak sweep)
        10 => {
            let center_partial = 8.0 + (w as f64) * 3.5;
            for i in 0..PEAK_HARMONIC_COUNT {
                let n = (i + 1) as f64;
                let dist = (n - center_partial).abs();
                let resonance = (-dist * dist / 4.5).exp();
                h[i] = (1.0 / n.powf(1.4)) + 1.8 * resonance;
            }
        }

        // 11: Formant 1 (Vowel formants /a/ -> /e/ -> /i/ -> /o/ -> /u/)
        11 => {
            let (f1, f2) = match w {
                0 => (4.0, 8.0),   // /a/
                1 => (3.0, 12.0),  // /e/
                2 => (2.0, 16.0),  // /i/
                3 => (3.0, 6.0),   // /o/
                _ => (2.0, 5.0),   // /u/
            };
            for i in 0..PEAK_HARMONIC_COUNT {
                let n = (i + 1) as f64;
                let d1 = (n - f1).abs();
                let d2 = (n - f2).abs();
                let p1 = (-d1 * d1 / 2.0).exp();
                let p2 = (-d2 * d2 / 3.0).exp() * 0.7;
                h[i] = (p1 + p2 + 0.08 / n).max(0.0);
            }
        }

        // 12: Formant 2 (Dual nasal/throat acoustic formant clusters)
        12 => {
            let base_cluster = 3.0 + (w as f64) * 1.5;
            let upper_cluster = 14.0 - (w as f64) * 1.2;
            for i in 0..PEAK_HARMONIC_COUNT {
                let n = (i + 1) as f64;
                let c1 = (-(n - base_cluster).powi(2) / 2.5).exp();
                let c2 = (-(n - upper_cluster).powi(2) / 3.5).exp() * 0.6;
                h[i] = c1 + c2 + 0.05 / n;
            }
        }

        // 13: Vocal (Choral acoustic spectral envelope)
        13 => {
            for i in 0..PEAK_HARMONIC_COUNT {
                let n = (i + 1) as f64;
                let vocal_tilt = (-0.12 * n).exp();
                let formant = (-(n - (4.0 + (w as f64) * 2.0)).powi(2) / 3.0).exp();
                h[i] = vocal_tilt * (0.6 + 1.2 * formant);
            }
        }

        // 14: Bell (Tubular bell / chime strike partials)
        14 => {
            let bell_partials = [1, 2, 4, 7, 11, 15, 19, 23, 27, 31];
            for (idx, &bp) in bell_partials.iter().enumerate() {
                if bp <= PEAK_HARMONIC_COUNT {
                    let weight = 1.0 / (idx as f64 + 1.0).powf(0.7);
                    let strike = 1.0 + (w as f64) * 0.2;
                    h[bp - 1] = weight * strike;
                }
            }
        }

        // 15: Organ (Tonewheel organ drawbar combinations)
        15 => {
            let drawbars = [1, 2, 3, 4, 6, 8, 10, 12, 16];
            let levels = match w {
                0 => [0.8, 1.0, 0.0, 0.6, 0.0, 0.0, 0.0, 0.0, 0.0], // Flute (8' + 4')
                1 => [0.8, 0.8, 0.6, 0.8, 0.4, 0.4, 0.0, 0.0, 0.0], // Gospel
                2 => [1.0, 0.8, 0.8, 0.8, 0.6, 0.5, 0.3, 0.0, 0.0], // Principal
                3 => [1.0, 1.0, 1.0, 1.0, 0.9, 0.9, 0.8, 0.7, 0.6], // Full Organ
                _ => [0.5, 0.5, 0.0, 0.8, 0.9, 1.0, 1.0, 0.9, 0.8], // Screaming Lead
            };
            for (i, &harmonic) in drawbars.iter().enumerate() {
                if harmonic <= PEAK_HARMONIC_COUNT {
                    h[harmonic - 1] = levels[i];
                }
            }
        }

        // 16: Digi (Digital wavetable stepped bitcrush harmonics)
        _ => {
            for i in 0..PEAK_HARMONIC_COUNT {
                let n = (i + 1) as f64;
                let step = (((i + w) % 4) as f64) * 0.25;
                h[i] = (1.0 / (n.powf(0.85))) * (0.35 + step);
            }
        }
    }

    // Normalize so highest harmonic peak is 1.0
    let mut max_val: f64 = 0.0;
    for &val in &h {
        if val.abs() > max_val {
            max_val = val.abs();
        }
    }
    if max_val > 1e-12 {
        for val in &mut h {
            *val /= max_val;
        }
    }
    h
}

/// Calculates the interpolated harmonic spectrum across the 5 waves of wavetable `table_idx`
/// for continuous shape parameter `shape` in `0.0..127.0`.
pub fn calculate_harmonic_spectrum(table_idx: usize, shape: f64) -> HarmonicSpectrum {
    let clamped_shape = shape.clamp(0.0, 127.0);
    let pos = (clamped_shape / 127.0) * ((PEAK_WAVES_PER_TABLE - 1) as f64);
    let idx0 = (pos.floor() as usize).min(PEAK_WAVES_PER_TABLE - 2);
    let idx1 = idx0 + 1;
    let frac = pos - (idx0 as f64);

    let h0 = wavetable_wave_harmonics(table_idx, idx0);
    let h1 = wavetable_wave_harmonics(table_idx, idx1);

    let mut interpolated = [0.0; PEAK_HARMONIC_COUNT];
    for i in 0..PEAK_HARMONIC_COUNT {
        interpolated[i] = (1.0 - frac) * h0[i] + frac * h1[i];
    }

    HarmonicSpectrum {
        table_index: table_idx % PEAK_WAVETABLE_COUNT,
        shape: clamped_shape,
        harmonics: interpolated,
    }
}

/// Generates a single-cycle time-domain waveform buffer of `num_samples` points
/// for the specified wavetable and shape interpolation using inverse Fourier additive synthesis.
pub fn calculate_wavetable_slice(table_idx: usize, shape: f64, num_samples: usize) -> Vec<f32> {
    let spectrum = calculate_harmonic_spectrum(table_idx, shape);
    let points = num_samples.max(32);
    let mut out = vec![0.0f32; points];

    let mut max_abs = 0.0f32;
    for (n, sample) in out.iter_mut().enumerate() {
        let phase_fund = (2.0 * PI * (n as f64)) / (points as f64);
        let mut sum = 0.0f64;
        for (h_idx, &amp) in spectrum.harmonics.iter().enumerate() {
            if amp > 1e-6 {
                let harmonic_num = (h_idx + 1) as f64;
                sum += amp * (harmonic_num * phase_fund).sin();
            }
        }
        let val = sum as f32;
        *sample = val;
        let abs_val = val.abs();
        if abs_val > max_abs {
            max_abs = abs_val;
        }
    }

    // Peak normalize to [-1.0, 1.0]
    if max_abs > 1e-6 {
        for sample in &mut out {
            *sample /= max_abs;
        }
    }
    out
}

// =============================================================================
// 2. Virtual Sync (VSync) & Voice Divergence / Drift
// =============================================================================

/// Calculates virtual hard-sync slave phase based on master phase and VSync control amount.
/// `vsync` in `0.0..127.0` maps to a frequency ratio from `1.0` (un-synced) to `16.0` (deep sync).
pub fn apply_vsync(master_phase: f64, vsync: f64) -> f64 {
    let clamped_sync = vsync.clamp(0.0, 127.0);
    if clamped_sync <= 0.001 {
        return master_phase.fract();
    }
    let ratio = 1.0 + (clamped_sync / 127.0) * 15.0;
    (master_phase * ratio).fract()
}

/// Voice divergence profile modeling analog component tolerances and drift.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct VoiceDivergence {
    /// 0-indexed voice index (0..7).
    pub voice_index: usize,
    /// Pitch detuning micro-offset in cents.
    pub pitch_drift_cents: f64,
    /// Cutoff frequency micro-offset in semitones.
    pub cutoff_drift_semitones: f64,
    /// Initial oscillator phase offset in radians.
    pub initial_phase: f64,
    /// Analogue resonance Q factor scaling factor.
    pub resonance_drift_factor: f64,
}

/// Computes deterministic voice divergence micro-variations for voice index (0..7).
pub fn calculate_voice_divergence(voice_idx: usize, drift_depth: f64) -> VoiceDivergence {
    let v = voice_idx % PEAK_VOICE_COUNT;
    let depth_ratio = (drift_depth.clamp(0.0, 127.0) / 127.0).max(0.0);

    // Deterministic golden-ratio and prime micro-offsets
    let pitch_drift_table = [-4.2, 3.1, -1.8, 4.5, -3.0, 2.2, -5.0, 3.8];
    let cutoff_drift_table = [0.15, -0.22, 0.08, -0.18, 0.25, -0.10, 0.12, -0.09];
    let phase_table = [
        0.0,
        0.785_398,
        1.570_796,
        2.356_194,
        3.141_592,
        3.926_991,
        4.712_389,
        5.497_787,
    ];
    let res_table = [1.02, 0.98, 1.01, 0.99, 1.03, 0.97, 1.00, 1.02];

    VoiceDivergence {
        voice_index: v,
        pitch_drift_cents: pitch_drift_table[v] * depth_ratio,
        cutoff_drift_semitones: cutoff_drift_table[v] * depth_ratio,
        initial_phase: phase_table[v],
        resonance_drift_factor: 1.0 + (res_table[v] - 1.0) * depth_ratio,
    }
}

// =============================================================================
// 3. Oxford FPGA NCO Oscillator Signal Generator
// =============================================================================

/// Single oscillator runtime state.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct OscVoiceState {
    /// Current master phase in 0.0..1.0.
    pub phase: f64,
}

impl Default for OscVoiceState {
    fn default() -> Self {
        Self { phase: 0.0 }
    }
}

// =============================================================================
// 4. Mixer Engine (Osc 1/2/3, Ring Mod 1x2, Noise)
// =============================================================================

/// Computes combined audio mix from the 3 oscillators, ring mod, and noise source.
pub fn mix_signals(
    osc1: f64,
    osc2: f64,
    osc3: f64,
    noise: f64,
    osc1_lvl: f64,
    osc2_lvl: f64,
    osc3_lvl: f64,
    ring_lvl: f64,
    noise_lvl: f64,
) -> f64 {
    let o1_g = osc1_lvl.clamp(0.0, 127.0) / 127.0;
    let o2_g = osc2_lvl.clamp(0.0, 127.0) / 127.0;
    let o3_g = osc3_lvl.clamp(0.0, 127.0) / 127.0;
    let ring_g = ring_lvl.clamp(0.0, 127.0) / 127.0;
    let noise_g = noise_lvl.clamp(0.0, 127.0) / 127.0;

    let ring_product = osc1 * osc2;

    let raw_sum =
        osc1 * o1_g + osc2 * o2_g + osc3 * o3_g + ring_product * ring_g + noise * noise_g;

    // Normalization headroom scaling
    let total_weights = o1_g + o2_g + o3_g + ring_g + noise_g;
    if total_weights > 1.0 {
        raw_sum / (1.0 + (total_weights - 1.0) * 0.4)
    } else {
        raw_sum
    }
}

// =============================================================================
// 5. Analogue State-Variable Filter (SVF) Model
// =============================================================================

/// Operational mode of the Novation Peak analogue state-variable filter.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FilterMode {
    /// Low-Pass mode (standard subtractive synth).
    LowPass,
    /// Band-Pass mode (mid-frequency band).
    BandPass,
    /// High-Pass mode (treble emphasis / bass cut).
    HighPass,
}

impl FilterMode {
    /// Parses filter mode from normalized numeric value (0 = LP, 1 = BP, 2 = HP).
    pub fn from_f64(val: f64) -> Self {
        let rounded = val.round() as i32;
        match rounded {
            1 => Self::BandPass,
            2 => Self::HighPass,
            _ => Self::LowPass,
        }
    }
}

/// Slope attenuation steepness of the filter.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FilterSlope {
    /// 12 dB/octave attenuation (2-pole single SVF stage).
    Slope12dB,
    /// 24 dB/octave attenuation (4-pole cascaded dual SVF stages).
    Slope24dB,
}

impl FilterSlope {
    /// Parses filter slope from numeric value (0 = 12dB, 1 = 24dB).
    pub fn from_f64(val: f64) -> Self {
        if val >= 0.5 {
            Self::Slope24dB
        } else {
            Self::Slope12dB
        }
    }
}

/// Converts Cutoff CC value (0..127) exponentially to frequency in Hertz (20.0 Hz to 20,000 Hz).
pub fn cutoff_cc_to_hz(cc_val: f64) -> f64 {
    let norm = (cc_val.clamp(0.0, 127.0)) / 127.0;
    20.0 * 1000.0f64.powf(norm)
}

/// Converts Resonance CC value (0..127) to filter quality factor Q (0.5 to 20.0).
pub fn resonance_cc_to_q(cc_val: f64) -> f64 {
    let norm = (cc_val.clamp(0.0, 127.0)) / 127.0;
    0.5 + norm * norm * 19.5
}

/// Discrete point along an analytical filter frequency response curve.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct FilterResponsePoint {
    /// Frequency in Hertz.
    pub frequency_hz: f64,
    /// Amplitude gain in decibels (dB).
    pub magnitude_db: f64,
    /// Phase shift in degrees.
    pub phase_deg: f64,
}

/// Computes the exact analytical frequency response curve across `num_points` logarithmic frequencies
/// for the specified cutoff, resonance Q, filter mode, and slope.
pub fn calculate_filter_response_curve(
    cutoff_hz: f64,
    resonance_q: f64,
    mode: FilterMode,
    slope: FilterSlope,
    num_points: usize,
) -> Vec<FilterResponsePoint> {
    let count = num_points.max(8);
    let mut curve = Vec::with_capacity(count);

    let min_freq: f64 = 20.0;
    let max_freq: f64 = 20_000.0;
    let log_min = min_freq.ln();
    let log_max = max_freq.ln();

    let q = resonance_q.max(0.2);
    let omega_0 = 2.0 * PI * cutoff_hz.max(10.0);

    for i in 0..count {
        let frac = (i as f64) / ((count - 1) as f64);
        let freq = (log_min + frac * (log_max - log_min)).exp();
        let omega = 2.0 * PI * freq;

        // SVF Transfer function denominator D(s) at s = j*omega
        // D(j*omega) = (omega_0^2 - omega^2) + j*(omega_0 * omega / Q)
        let re_d = omega_0 * omega_0 - omega * omega;
        let im_d = (omega_0 * omega) / q;
        let denom_sq = re_d * re_d + im_d * im_d;

        // Numerator magnitude squared based on filter mode
        let num_sq = match mode {
            FilterMode::LowPass => omega_0.powi(4),
            FilterMode::BandPass => (omega_0 * omega / q).powi(2),
            FilterMode::HighPass => omega.powi(4),
        };

        let linear_gain_12db = (num_sq / denom_sq.max(1e-12)).sqrt();
        let linear_gain = match slope {
            FilterSlope::Slope12dB => linear_gain_12db,
            FilterSlope::Slope24dB => linear_gain_12db * linear_gain_12db,
        };

        let mag_db = 20.0 * (linear_gain.max(1e-6)).log10();

        // Phase angle in degrees
        let phase_d = im_d.atan2(re_d);
        let phase_n = match mode {
            FilterMode::LowPass => 0.0,
            FilterMode::BandPass => PI / 2.0,
            FilterMode::HighPass => PI,
        };
        let phase_12db = phase_n - phase_d;
        let phase_total = match slope {
            FilterSlope::Slope12dB => phase_12db,
            FilterSlope::Slope24dB => phase_12db * 2.0,
        };
        let phase_deg = (phase_total * 180.0 / PI).rem_euclid(360.0);

        curve.push(FilterResponsePoint {
            frequency_hz: freq,
            magnitude_db: mag_db,
            phase_deg,
        });
    }

    curve
}

/// Real-time Cytomic / trapezoidal digital state-variable filter (SVF).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct StateVariableFilter {
    s1_stage1: f64,
    s2_stage1: f64,
    s1_stage2: f64,
    s2_stage2: f64,
}

impl Default for StateVariableFilter {
    fn default() -> Self {
        Self {
            s1_stage1: 0.0,
            s2_stage1: 0.0,
            s1_stage2: 0.0,
            s2_stage2: 0.0,
        }
    }
}

impl StateVariableFilter {
    /// Resets internal filter integrator state memories to zero.
    pub fn reset(&mut self) {
        self.s1_stage1 = 0.0;
        self.s2_stage1 = 0.0;
        self.s1_stage2 = 0.0;
        self.s2_stage2 = 0.0;
    }

    /// Evaluates a single filter processing stage.
    fn process_single_stage(
        input: f64,
        s1: &mut f64,
        s2: &mut f64,
        cutoff_hz: f64,
        q: f64,
        mode: FilterMode,
        sample_rate: f64,
    ) -> f64 {
        let f_clamped = cutoff_hz.clamp(20.0, sample_rate * 0.48);
        let g = (PI * f_clamped / sample_rate).tan().clamp(1e-6, 40.0);
        let k = 1.0 / q.max(0.2);

        let a1 = 1.0 / (1.0 + g * (g + k));
        let a2 = g * a1;
        let a3 = g * a2;

        let v3 = input - *s2;
        let v1 = a1 * *s1 + a2 * v3;
        let v2 = *s2 + a2 * *s1 + a3 * v3;

        *s1 = 2.0 * v1 - *s1;
        *s2 = 2.0 * v2 - *s2;

        match mode {
            FilterMode::LowPass => v2,
            FilterMode::BandPass => v1,
            FilterMode::HighPass => input - k * v1 - v2,
        }
    }

    /// Processes an input audio sample through pre-filter overdrive, SVF (12dB or 24dB cascade),
    /// and post-filter drive.
    pub fn process(
        &mut self,
        input: f64,
        cutoff_hz: f64,
        resonance_q: f64,
        mode: FilterMode,
        slope: FilterSlope,
        pre_overdrive: f64,
        post_drive: f64,
        sample_rate: f64,
    ) -> f64 {
        // Pre-Filter Overdrive: hyperbolic tangent saturation
        let pre_gain = 1.0 + (pre_overdrive.clamp(0.0, 127.0) / 127.0) * 8.0;
        let saturated_in = (input * pre_gain).tanh() / pre_gain.sqrt();

        // Stage 1 (12dB)
        let stage1_out = Self::process_single_stage(
            saturated_in,
            &mut self.s1_stage1,
            &mut self.s2_stage1,
            cutoff_hz,
            resonance_q,
            mode,
            sample_rate,
        );

        let filtered_out = match slope {
            FilterSlope::Slope12dB => stage1_out,
            FilterSlope::Slope24dB => Self::process_single_stage(
                stage1_out,
                &mut self.s1_stage2,
                &mut self.s2_stage2,
                cutoff_hz,
                resonance_q,
                mode,
                sample_rate,
            ),
        };

        // Post-Filter Drive: post-resonance saturation
        let post_gain = 1.0 + (post_drive.clamp(0.0, 127.0) / 127.0) * 6.0;
        (filtered_out * post_gain).tanh()
    }
}

// =============================================================================
// 6. Voice Engine & 8-Voice State Representation
// =============================================================================

/// Runtime state for a single hardware voice.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PeakVoice {
    /// 0-indexed voice identifier (0..7).
    pub voice_index: usize,
    /// Whether this voice is currently sounding.
    pub active: bool,
    /// Currently assigned MIDI note number (0..127).
    pub note: u8,
    /// Note-on velocity (0..127).
    pub velocity: u8,
    /// Voice age allocation counter for voice stealing.
    pub age: u64,
    /// Master phases for the 3 Oxford FPGA NCO oscillators.
    pub osc_phases: [f64; PEAK_OSC_COUNT],
    /// State-variable filter state.
    pub filter: StateVariableFilter,
    /// Voice divergence micro-variations.
    pub divergence: VoiceDivergence,
    /// Noise generator state.
    pub noise_state: u64,
    /// Envelope amplitude level for audition / audio rendering.
    pub env_level: f64,
    /// Whether the note gate is currently held down.
    pub gate: bool,
}

impl PeakVoice {
    /// Creates a new inactive voice with the given index.
    pub fn new(voice_index: usize) -> Self {
        Self {
            voice_index,
            active: false,
            note: 60,
            velocity: 0,
            age: 0,
            osc_phases: [0.0; PEAK_OSC_COUNT],
            filter: StateVariableFilter::default(),
            divergence: calculate_voice_divergence(voice_index, 15.0),
            noise_state: 0x1234_5678_9ABC_DEF0 ^ ((voice_index as u64) + 1),
            env_level: 0.0,
            gate: false,
        }
    }

    /// Generates next deterministic uniform noise sample in [-1.0, 1.0] using Xorshift64.
    pub fn next_noise(&mut self) -> f64 {
        let mut x = self.noise_state;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.noise_state = x;
        ((x as f64) / (u64::MAX as f64)) * 2.0 - 1.0
    }
}

// =============================================================================
// 7. Core Novation Peak Synthesis Domain Engine
// =============================================================================

/// Complete 8-Voice Novation Peak synthesis domain engine.
/// Directly backed by `synth_core::SynthPatch` and `synth_core::CommandHistory`.
#[derive(Debug)]
pub struct PeakSynthEngine {
    /// Canonical patch document storing all parameter values.
    pub patch: SynthPatch,
    /// Undo/redo command history stack.
    pub history: CommandHistory,
    /// 8 polyphonic voice instances.
    pub voices: [PeakVoice; PEAK_VOICE_COUNT],
    /// Evaluation sample rate in Hertz.
    pub sample_rate: f64,
    /// Monotonic counter used for voice-stealing timestamps.
    pub note_counter: u64,
}

impl Default for PeakSynthEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl PeakSynthEngine {
    /// Creates a new synthesis domain engine initialized with the default factory patch.
    pub fn new() -> Self {
        Self::with_patch(create_default_peak_patch())
    }

    /// Creates a new synthesis domain engine using a provided patch.
    pub fn with_patch(patch: SynthPatch) -> Self {
        let voices = [
            PeakVoice::new(0),
            PeakVoice::new(1),
            PeakVoice::new(2),
            PeakVoice::new(3),
            PeakVoice::new(4),
            PeakVoice::new(5),
            PeakVoice::new(6),
            PeakVoice::new(7),
        ];

        let mut engine = Self {
            patch,
            history: CommandHistory::new(CommandHistory::DEFAULT_MAX_DEPTH),
            voices,
            sample_rate: PEAK_DEFAULT_SAMPLE_RATE,
            note_counter: 0,
        };
        engine.update_divergence_from_patch();
        engine
    }

    /// Read-only reference to the internal `SynthPatch`.
    pub fn patch(&self) -> &SynthPatch {
        &self.patch
    }

    /// Mutable reference to the internal `SynthPatch`.
    pub fn patch_mut(&mut self) -> &mut SynthPatch {
        &mut self.patch
    }

    /// Read-only reference to the command history stack.
    pub fn history(&self) -> &CommandHistory {
        &self.history
    }

    /// Mutable reference to the command history stack.
    pub fn history_mut(&mut self) -> &mut CommandHistory {
        &mut self.history
    }

    /// Updates voice divergence tables across all 8 voices based on current `voice_drift` parameter.
    pub fn update_divergence_from_patch(&mut self) {
        let drift_depth = self.get_param_or("voice_drift", 15.0);
        for v in &mut self.voices {
            v.divergence = calculate_voice_divergence(v.voice_index, drift_depth);
        }
    }

    /// Retrieves a parameter value by key, returning `0.0` if not found.
    pub fn get_param(&self, key: &str) -> f64 {
        self.patch.get_param(key).unwrap_or(0.0)
    }

    /// Retrieves a parameter value by key, returning a specified default if not found.
    pub fn get_param_or(&self, key: &str, default: f64) -> f64 {
        self.patch.get_param(key).unwrap_or(default)
    }

    /// Sets a parameter value, pushing a `SetParamCommand` to `synth_core::CommandHistory`.
    pub fn set_param(&mut self, key: impl Into<String>, value: f64) -> Result<(), SynthError> {
        let k = key.into();
        let old_val = self.get_param(&k);
        let cmd = Box::new(SetParamCommand::new(k.clone(), old_val, value));
        self.history.push_and_apply(cmd, &mut self.patch)?;

        if k == "voice_drift" {
            self.update_divergence_from_patch();
        }
        Ok(())
    }

    /// Sets a parameter by standard MIDI CC number.
    pub fn set_cc(&mut self, cc: u8, value: u8) -> Result<(), SynthError> {
        if let Some(param_key) = cc_to_param_key(cc) {
            self.set_param(param_key, value as f64)
        } else {
            // Unmapped CC stored under generic key
            let key = format!("cc_{cc}");
            self.set_param(key, value as f64)
        }
    }

    /// Applies a batch of parameter changes atomically under a single undo step.
    pub fn batch_set_params(
        &mut self,
        changes: Vec<(String, f64, f64)>,
        description: impl Into<String>,
    ) -> Result<(), SynthError> {
        let cmd = Box::new(BatchSetParamsCommand::new(changes, description));
        self.history.push_and_apply(cmd, &mut self.patch)?;
        self.update_divergence_from_patch();
        Ok(())
    }

    /// Rolls back the most recent patch modification.
    pub fn undo(&mut self) -> Result<Option<String>, SynthError> {
        let desc = self.history.undo(&mut self.patch)?;
        self.update_divergence_from_patch();
        Ok(desc)
    }

    /// Re-applies the most recently rolled-back patch modification.
    pub fn redo(&mut self) -> Result<Option<String>, SynthError> {
        let desc = self.history.redo(&mut self.patch)?;
        self.update_divergence_from_patch();
        Ok(desc)
    }

    /// Returns whether undo operations are available.
    pub fn can_undo(&self) -> bool {
        self.history.can_undo()
    }

    /// Returns whether redo operations are available.
    pub fn can_redo(&self) -> bool {
        self.history.can_redo()
    }

    /// Returns the number of currently sounding polyphonic voices.
    pub fn active_voice_count(&self) -> usize {
        self.voices.iter().filter(|v| v.active).count()
    }

    /// Returns voice divergence profile for voice `voice_idx`.
    pub fn voice_divergence(&self, voice_idx: usize) -> VoiceDivergence {
        self.voices[voice_idx % PEAK_VOICE_COUNT].divergence
    }

    // -------------------------------------------------------------------------
    // Polyphonic Voice Allocation
    // -------------------------------------------------------------------------

    /// Triggers a note-on event across the 8-voice allocation engine.
    pub fn note_on(&mut self, note: u8, velocity: u8) {
        if velocity == 0 {
            self.note_off(note);
            return;
        }

        self.note_counter += 1;
        let counter = self.note_counter;

        // 1. If this exact note is already sounding on a voice, re-trigger it
        for voice in &mut self.voices {
            if voice.active && voice.note == note {
                voice.velocity = velocity;
                voice.age = counter;
                voice.gate = true;
                voice.env_level = 1.0;
                return;
            }
        }

        // 2. Look for an inactive free voice
        if let Some(voice) = self.voices.iter_mut().find(|v| !v.active) {
            voice.active = true;
            voice.note = note;
            voice.velocity = velocity;
            voice.age = counter;
            voice.gate = true;
            voice.env_level = 1.0;
            voice.filter.reset();
            return;
        }

        // 3. All 8 voices active: Voice-steal the oldest sounding voice
        let mut oldest_idx = 0;
        let mut min_age = u64::MAX;
        for (i, v) in self.voices.iter().enumerate() {
            if v.age < min_age {
                min_age = v.age;
                oldest_idx = i;
            }
        }

        let voice = &mut self.voices[oldest_idx];
        voice.active = true;
        voice.note = note;
        voice.velocity = velocity;
        voice.age = counter;
        voice.gate = true;
        voice.env_level = 1.0;
        voice.filter.reset();
    }

    /// Triggers a note-off event, releasing active voice(s) playing `note`.
    pub fn note_off(&mut self, note: u8) {
        for voice in &mut self.voices {
            if voice.active && voice.note == note {
                voice.gate = false;
                voice.active = false;
                voice.env_level = 0.0;
            }
        }
    }

    /// Releases all 8 polyphonic voices immediately.
    pub fn all_notes_off(&mut self) {
        for voice in &mut self.voices {
            voice.gate = false;
            voice.active = false;
            voice.env_level = 0.0;
            voice.filter.reset();
        }
    }

    // -------------------------------------------------------------------------
    // Audio Synthesis Evaluation
    // -------------------------------------------------------------------------

    /// Calculates oscillator frequency in Hertz taking range footing, coarse tune,
    /// fine tune, and voice pitch divergence into account.
    pub fn calculate_osc_freq(
        &self,
        osc_index: usize,
        note: u8,
        divergence_cents: f64,
    ) -> f64 {
        let osc_num = osc_index + 1;
        let range_key = format!("osc_{osc_num}_range");
        let coarse_key = format!("osc_{osc_num}_coarse");
        let fine_key = format!("osc_{osc_num}_fine");

        let range_val = self.get_param_or(&range_key, 1.0);
        let oct_shift = range_to_octave_shift(range_val);

        let coarse = self.get_param_or(&coarse_key, 0.0);
        let fine = self.get_param_or(&fine_key, 0.0);

        let total_semitones = (note as f64) - 69.0
            + (oct_shift as f64) * 12.0
            + coarse
            + (fine + divergence_cents) / 100.0;

        440.0 * 2.0f64.powf(total_semitones / 12.0)
    }

    /// Evaluates a single Oxford FPGA oscillator output sample for a voice.
    pub fn evaluate_osc_sample(
        &self,
        osc_index: usize,
        phase: f64,
    ) -> f64 {
        let osc_num = osc_index + 1;
        let wt_key = format!("osc_{osc_num}_wavetable");
        let shape_key = format!("osc_{osc_num}_shape");
        let vsync_key = format!("osc_{osc_num}_vsync");

        let wt_idx = (self.get_param_or(&wt_key, 0.0).round() as usize) % PEAK_WAVETABLE_COUNT;
        let shape = self.get_param_or(&shape_key, 64.0);
        let vsync = self.get_param_or(&vsync_key, 0.0);

        let synced_phase = apply_vsync(phase, vsync);
        let spectrum = calculate_harmonic_spectrum(wt_idx, shape);

        let phase_rad = synced_phase * 2.0 * PI;
        let mut sample = 0.0;
        for (h_idx, &amp) in spectrum.harmonics.iter().enumerate() {
            if amp > 1e-6 {
                let h_num = (h_idx + 1) as f64;
                sample += amp * (h_num * phase_rad).sin();
            }
        }
        sample
    }

/// Evaluates oscillator sample from precomputed harmonic spectrum and vsync amount.
pub fn evaluate_osc_from_spectrum(phase: f64, vsync: f64, spectrum: &HarmonicSpectrum) -> f64 {
    let synced_phase = apply_vsync(phase, vsync);
    let phase_rad = synced_phase * 2.0 * PI;
    let mut sample = 0.0;
    for (h_idx, &amp) in spectrum.harmonics.iter().enumerate() {
        if amp > 1e-6 {
            let h_num = (h_idx + 1) as f64;
            sample += amp * (h_num * phase_rad).sin();
        }
    }
    sample
}

/// Computes oscillator frequency from note, octave shift, coarse tune, fine tune, and divergence.
pub fn calc_osc_frequency(
    note: u8,
    oct_shift: i32,
    coarse: f64,
    fine: f64,
    divergence_cents: f64,
) -> f64 {
    let total_semitones = (note as f64) - 69.0
        + (oct_shift as f64) * 12.0
        + coarse
        + (fine + divergence_cents) / 100.0;
    440.0 * 2.0f64.powf(total_semitones / 12.0)
}

    /// Renders a single stereo-summed audio sample across all 8 polyphonic voices.
    pub fn render_sample(&mut self) -> f64 {
        let sample_rate = self.sample_rate.max(1000.0);
        let dt = 1.0 / sample_rate;

        // Mixer levels
        let osc1_lvl = self.get_param_or("osc_1_mix", 127.0);
        let osc2_lvl = self.get_param_or("osc_2_mix", 0.0);
        let osc3_lvl = self.get_param_or("osc_3_mix", 0.0);
        let ring_lvl = self.get_param_or("ring_mod_mix", 0.0);
        let noise_lvl = self.get_param_or("noise_mix", 0.0);

        // Pre-compute oscillator tuning and spectra
        let oct1 = range_to_octave_shift(self.get_param_or("osc_1_range", 1.0));
        let coarse1 = self.get_param_or("osc_1_coarse", 0.0);
        let fine1 = self.get_param_or("osc_1_fine", 0.0);
        let wt1 = (self.get_param_or("osc_1_wavetable", 0.0).round() as usize) % PEAK_WAVETABLE_COUNT;
        let shape1 = self.get_param_or("osc_1_shape", 64.0);
        let vsync1 = self.get_param_or("osc_1_vsync", 0.0);
        let spec1 = calculate_harmonic_spectrum(wt1, shape1);

        let oct2 = range_to_octave_shift(self.get_param_or("osc_2_range", 1.0));
        let coarse2 = self.get_param_or("osc_2_coarse", 0.0);
        let fine2 = self.get_param_or("osc_2_fine", 0.0);
        let wt2 = (self.get_param_or("osc_2_wavetable", 1.0).round() as usize) % PEAK_WAVETABLE_COUNT;
        let shape2 = self.get_param_or("osc_2_shape", 64.0);
        let vsync2 = self.get_param_or("osc_2_vsync", 0.0);
        let spec2 = calculate_harmonic_spectrum(wt2, shape2);

        let oct3 = range_to_octave_shift(self.get_param_or("osc_3_range", 1.0));
        let coarse3 = self.get_param_or("osc_3_coarse", 0.0);
        let fine3 = self.get_param_or("osc_3_fine", 0.0);
        let wt3 = (self.get_param_or("osc_3_wavetable", 2.0).round() as usize) % PEAK_WAVETABLE_COUNT;
        let shape3 = self.get_param_or("osc_3_shape", 64.0);
        let vsync3 = self.get_param_or("osc_3_vsync", 0.0);
        let spec3 = calculate_harmonic_spectrum(wt3, shape3);

        // Filter settings
        let base_cutoff_cc = self.get_param_or("filter_cutoff", 100.0);
        let base_cutoff_hz = cutoff_cc_to_hz(base_cutoff_cc);
        let resonance_cc = self.get_param_or("filter_resonance", 20.0);
        let base_q = resonance_cc_to_q(resonance_cc);
        let key_track_ratio = self.get_param_or("filter_track", 64.0).clamp(0.0, 127.0) / 127.0;
        let osc3_fm_depth = self.get_param_or("osc_3_to_filter", 0.0).clamp(0.0, 127.0) / 127.0;
        let mod_env_depth = self.get_param_or("mod_1_to_filter", 0.0);
        let pre_drive = self.get_param_or("filter_overdrive", 0.0);
        let post_drive = self.get_param_or("filter_post_drive", 0.0);
        let filter_mode = FilterMode::from_f64(self.get_param_or("filter_mode", 0.0));
        let filter_slope = FilterSlope::from_f64(self.get_param_or("filter_slope", 1.0));

        let mut mix_accum = 0.0;

        for voice in &mut self.voices {
            if !voice.active {
                continue;
            }

            // 1. Compute oscillator frequencies and evaluate samples
            let f1 = Self::calc_osc_frequency(voice.note, oct1, coarse1, fine1, voice.divergence.pitch_drift_cents);
            let f2 = Self::calc_osc_frequency(voice.note, oct2, coarse2, fine2, voice.divergence.pitch_drift_cents);
            let f3 = Self::calc_osc_frequency(voice.note, oct3, coarse3, fine3, voice.divergence.pitch_drift_cents);

            voice.osc_phases[0] = (voice.osc_phases[0] + f1 * dt).fract();
            voice.osc_phases[1] = (voice.osc_phases[1] + f2 * dt).fract();
            voice.osc_phases[2] = (voice.osc_phases[2] + f3 * dt).fract();

            let s1 = Self::evaluate_osc_from_spectrum(voice.osc_phases[0], vsync1, &spec1);
            let s2 = Self::evaluate_osc_from_spectrum(voice.osc_phases[1], vsync2, &spec2);
            let s3 = Self::evaluate_osc_from_spectrum(voice.osc_phases[2], vsync3, &spec3);
            let noise_sample = voice.next_noise();

            // 2. Mixer
            let pre_filter_mix = mix_signals(
                s1, s2, s3, noise_sample, osc1_lvl, osc2_lvl, osc3_lvl, ring_lvl, noise_lvl,
            );

            // 3. Analogue Filter Cutoff Modulation
            // Key tracking shift
            let key_semitones = ((voice.note as f64) - 60.0) * key_track_ratio;
            // Osc 3 -> Filter FM modulation
            let fm_shift = s3 * osc3_fm_depth * 1.5;
            // Mod Env 1 modulation
            let env_shift = (mod_env_depth / 64.0) * voice.env_level * 2.0;

            let total_cutoff_shift = key_semitones
                + voice.divergence.cutoff_drift_semitones
                + fm_shift
                + env_shift;
            let effective_cutoff_hz = (base_cutoff_hz * 2.0f64.powf(total_cutoff_shift / 12.0))
                .clamp(20.0, sample_rate * 0.48);
            let effective_q = base_q * voice.divergence.resonance_drift_factor;

            // 4. Analogue State-Variable Filter (with Pre-Drive & Post-Drive)
            let filtered = voice.filter.process(
                pre_filter_mix,
                effective_cutoff_hz,
                effective_q,
                filter_mode,
                filter_slope,
                pre_drive,
                post_drive,
                sample_rate,
            );

            let voice_output = filtered * ((voice.velocity as f64) / 127.0) * voice.env_level;
            mix_accum += voice_output;
        }

        // Master gain scaling
        (mix_accum * 0.25).clamp(-1.0, 1.0)
    }

    /// Renders a block of audio samples into the provided output slice.
    pub fn render_block(&mut self, output: &mut [f32]) {
        for sample in output.iter_mut() {
            *sample = self.render_sample() as f32;
        }
    }

    /// Calculates the analytical filter response curve using current patch parameters.
    pub fn calculate_filter_response_curve(&self, num_points: usize) -> Vec<FilterResponsePoint> {
        let cutoff_cc = self.get_param_or("filter_cutoff", 100.0);
        let cutoff_hz = cutoff_cc_to_hz(cutoff_cc);
        let resonance_cc = self.get_param_or("filter_resonance", 20.0);
        let q = resonance_cc_to_q(resonance_cc);
        let mode = FilterMode::from_f64(self.get_param_or("filter_mode", 0.0));
        let slope = FilterSlope::from_f64(self.get_param_or("filter_slope", 1.0));

        calculate_filter_response_curve(cutoff_hz, q, mode, slope, num_points)
    }

    /// Calculates the harmonic spectrum for a given oscillator index (0, 1, or 2).
    pub fn calculate_osc_harmonic_spectrum(&self, osc_index: usize) -> HarmonicSpectrum {
        let osc_num = osc_index + 1;
        let wt_key = format!("osc_{osc_num}_wavetable");
        let shape_key = format!("osc_{osc_num}_shape");

        let wt_idx = (self.get_param_or(&wt_key, 0.0).round() as usize) % PEAK_WAVETABLE_COUNT;
        let shape = self.get_param_or(&shape_key, 64.0);
        calculate_harmonic_spectrum(wt_idx, shape)
    }

    /// Calculates a single-cycle wavetable waveform slice for a given oscillator index.
    pub fn calculate_osc_wavetable_slice(
        &self,
        osc_index: usize,
        num_samples: usize,
    ) -> Vec<f32> {
        let osc_num = osc_index + 1;
        let wt_key = format!("osc_{osc_num}_wavetable");
        let shape_key = format!("osc_{osc_num}_shape");

        let wt_idx = (self.get_param_or(&wt_key, 0.0).round() as usize) % PEAK_WAVETABLE_COUNT;
        let shape = self.get_param_or(&shape_key, 64.0);
        calculate_wavetable_slice(wt_idx, shape, num_samples)
    }
}

/// Creates a default factory initialization patch for Novation Peak.
pub fn create_default_peak_patch() -> SynthPatch {
    let mut patch = SynthPatch::new("Init Peak Patch")
        .with_category("Init")
        .with_bank_slot("A", 1);

    // Oscillator 1
    patch.set_param("osc_1_range", 1.0); // 8'
    patch.set_param("osc_1_coarse", 0.0);
    patch.set_param("osc_1_fine", 0.0);
    patch.set_param("osc_1_shape", 64.0);
    patch.set_param("osc_1_vsync", 0.0);
    patch.set_param("osc_1_wavetable", 0.0); // Sine-Tri
    patch.set_param("osc_1_mod_2_pitch", 0.0);
    patch.set_param("osc_1_lfo_2_pitch", 0.0);
    patch.set_param("osc_1_lfo_1_shape", 0.0);
    patch.set_param("osc_1_mod_1_shape", 0.0);

    // Oscillator 2
    patch.set_param("osc_2_range", 1.0); // 8'
    patch.set_param("osc_2_coarse", 0.0);
    patch.set_param("osc_2_fine", 0.0);
    patch.set_param("osc_2_shape", 64.0);
    patch.set_param("osc_2_vsync", 0.0);
    patch.set_param("osc_2_wavetable", 1.0); // Sine-Saw
    patch.set_param("osc_2_mod_2_pitch", 0.0);
    patch.set_param("osc_2_lfo_2_pitch", 0.0);
    patch.set_param("osc_2_lfo_1_shape", 0.0);
    patch.set_param("osc_2_mod_1_shape", 0.0);

    // Oscillator 3
    patch.set_param("osc_3_range", 1.0); // 8'
    patch.set_param("osc_3_coarse", 0.0);
    patch.set_param("osc_3_fine", 0.0);
    patch.set_param("osc_3_shape", 64.0);
    patch.set_param("osc_3_vsync", 0.0);
    patch.set_param("osc_3_wavetable", 2.0); // Classic
    patch.set_param("osc_3_mod_2_pitch", 0.0);
    patch.set_param("osc_3_lfo_2_pitch", 0.0);
    patch.set_param("osc_3_lfo_1_shape", 0.0);
    patch.set_param("osc_3_mod_1_pitch", 0.0);

    // Mixer
    patch.set_param("osc_1_mix", 127.0);
    patch.set_param("osc_2_mix", 0.0);
    patch.set_param("osc_3_mix", 0.0);
    patch.set_param("ring_mod_mix", 0.0);
    patch.set_param("noise_mix", 0.0);

    // Filter
    patch.set_param("filter_cutoff", 100.0);
    patch.set_param("filter_resonance", 20.0);
    patch.set_param("filter_track", 64.0);
    patch.set_param("osc_3_to_filter", 0.0);
    patch.set_param("mod_1_to_filter", 0.0);
    patch.set_param("filter_overdrive", 0.0);
    patch.set_param("filter_post_drive", 0.0);
    patch.set_param("filter_mode", 0.0); // LowPass
    patch.set_param("filter_slope", 1.0); // 24dB

    // Voice Drift
    patch.set_param("voice_drift", 15.0);

    // Envelopes
    patch.set_param("amp_envelope_attack", 5.0);
    patch.set_param("amp_envelope_decay", 64.0);
    patch.set_param("amp_envelope_sustain", 127.0);
    patch.set_param("amp_envelope_release", 32.0);

    patch.set_param("mod_envelope_1_attack", 0.0);
    patch.set_param("mod_envelope_1_decay", 64.0);
    patch.set_param("mod_envelope_1_sustain", 0.0);
    patch.set_param("mod_envelope_1_release", 32.0);

    patch
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_engine_initialization_and_voices_baseline() {
        let engine = PeakSynthEngine::new();
        assert_eq!(engine.voices.len(), PEAK_VOICE_COUNT);
        assert_eq!(engine.active_voice_count(), 0);
        assert!(!engine.can_undo());
        assert!(!engine.can_redo());
        assert_eq!(engine.get_param("osc_1_range"), 1.0);
        assert_eq!(engine.get_param("osc_1_mix"), 127.0);
        assert_eq!(engine.get_param("filter_cutoff"), 100.0);
    }

    #[test]
    fn test_all_three_oscillators_parameters_and_ranges() {
        let mut engine = PeakSynthEngine::new();

        // Osc 1
        engine.set_cc(3, 2).expect("set osc 1 range"); // Range CC3 -> 4' (+1 oct)
        engine.set_cc(14, 7).expect("set osc 1 coarse"); // Coarse CC14
        engine.set_cc(15, 25).expect("set osc 1 fine"); // Fine CC15
        engine.set_cc(12, 80).expect("set osc 1 shape"); // Shape CC12
        engine.set_cc(34, 45).expect("set osc 1 vsync"); // VSync CC34

        // Osc 2
        engine.set_cc(37, 0).expect("set osc 2 range"); // Range CC37 -> 16' (-1 oct)
        engine.set_cc(17, 12).expect("set osc 2 coarse"); // Coarse CC17
        engine.set_cc(18, 50).expect("set osc 2 fine"); // Fine CC18
        engine.set_cc(39, 100).expect("set osc 2 shape"); // Shape CC39
        engine.set_cc(42, 60).expect("set osc 2 vsync"); // VSync CC42

        // Osc 3
        engine.set_cc(65, 3).expect("set osc 3 range"); // Range CC65 -> 2' (+2 oct)
        engine.set_cc(20, 5).expect("set osc 3 coarse"); // Coarse CC20
        engine.set_cc(21, 10).expect("set osc 3 fine"); // Fine CC21
        engine.set_cc(71, 127).expect("set osc 3 shape"); // Shape CC71
        engine.set_cc(44, 90).expect("set osc 3 vsync"); // VSync CC44

        assert_eq!(engine.get_param("osc_1_range"), 2.0);
        assert_eq!(engine.get_param("osc_1_coarse"), 7.0);
        assert_eq!(engine.get_param("osc_1_fine"), 25.0);
        assert_eq!(engine.get_param("osc_1_shape"), 80.0);
        assert_eq!(engine.get_param("osc_1_vsync"), 45.0);

        assert_eq!(engine.get_param("osc_2_range"), 0.0);
        assert_eq!(engine.get_param("osc_2_coarse"), 12.0);
        assert_eq!(engine.get_param("osc_2_fine"), 50.0);
        assert_eq!(engine.get_param("osc_2_shape"), 100.0);
        assert_eq!(engine.get_param("osc_2_vsync"), 60.0);

        assert_eq!(engine.get_param("osc_3_range"), 3.0);
        assert_eq!(engine.get_param("osc_3_coarse"), 5.0);
        assert_eq!(engine.get_param("osc_3_fine"), 10.0);
        assert_eq!(engine.get_param("osc_3_shape"), 127.0);
        assert_eq!(engine.get_param("osc_3_vsync"), 90.0);

        // Frequencies verify proper octave shifts
        let f1 = engine.calculate_osc_freq(0, 69, 0.0); // 440 * 2^(1) = 880 + coarse
        let f2 = engine.calculate_osc_freq(1, 69, 0.0); // 440 * 2^(-1) = 220 + coarse
        let f3 = engine.calculate_osc_freq(2, 69, 0.0); // 440 * 2^(2) = 1760 + coarse

        assert!(f1 > 700.0);
        assert!(f2 < 500.0);
        assert!(f3 > 1500.0);
    }

    #[test]
    fn test_17_wavetables_and_5_waves_deterministic_spectra() {
        assert_eq!(WAVETABLE_NAMES.len(), 17);

        for t in 0..PEAK_WAVETABLE_COUNT {
            for w in 0..PEAK_WAVES_PER_TABLE {
                let harmonics = wavetable_wave_harmonics(t, w);
                assert_eq!(harmonics.len(), PEAK_HARMONIC_COUNT);
                let sum: f64 = harmonics.iter().sum();
                assert!(sum > 0.0, "Wavetable {t} wave {w} should have positive energy");

                let (peak_h, peak_amp) = HarmonicSpectrum {
                    table_index: t,
                    shape: (w as f64) * 31.75,
                    harmonics,
                }
                .peak_harmonic();
                assert!(peak_h >= 1);
                assert!(peak_amp >= 0.8, "Normalized peak harmonic should be near 1.0");
            }
        }
    }

    #[test]
    fn test_wavetable_shape_smooth_interpolation() {
        let spec0 = calculate_harmonic_spectrum(1, 0.0);
        let spec64 = calculate_harmonic_spectrum(1, 64.0);
        let spec127 = calculate_harmonic_spectrum(1, 127.0);

        let c0 = spec0.spectral_centroid();
        let c64 = spec64.spectral_centroid();
        let c127 = spec127.spectral_centroid();

        // In Sine-Saw, higher shape adds higher harmonics, increasing the centroid
        assert!(c64 >= c0, "Spectral centroid should increase from sine to saw");
        assert!(c127 >= c64, "Spectral centroid should increase to bright saw");

        // Verify wavetable slice waveform generator
        let slice = calculate_wavetable_slice(1, 64.0, 256);
        assert_eq!(slice.len(), 256);
        for &s in &slice {
            assert!(s >= -1.05 && s <= 1.05);
        }
    }

    #[test]
    fn test_vsync_hard_sync_behavior() {
        // Without vsync, phase is identity
        assert_eq!(apply_vsync(0.25, 0.0), 0.25);
        assert_eq!(apply_vsync(0.75, 0.0), 0.75);

        // With vsync = 127.0 (ratio = 16.0), master phase 0.1 wraps into 1.6 % 1.0 = 0.6
        let synced = apply_vsync(0.1, 127.0);
        assert!((synced - 0.6).abs() < 1e-4);
    }

    #[test]
    fn test_voice_divergence_and_drift() {
        let div_off = calculate_voice_divergence(0, 0.0);
        assert_eq!(div_off.pitch_drift_cents, 0.0);
        assert_eq!(div_off.cutoff_drift_semitones, 0.0);

        let mut pitches = Vec::new();
        for v in 0..PEAK_VOICE_COUNT {
            let div = calculate_voice_divergence(v, 50.0);
            pitches.push(div.pitch_drift_cents);
        }

        // Voice divergence must be distinct across the 8 voices
        for i in 0..PEAK_VOICE_COUNT {
            for j in (i + 1)..PEAK_VOICE_COUNT {
                assert_ne!(
                    pitches[i], pitches[j],
                    "Voices {i} and {j} should have distinct pitch drift"
                );
            }
        }
    }

    #[test]
    fn test_mixer_routing_levels_ring_mod_and_noise() {
        // Pure Osc 1
        let m1 = mix_signals(0.8, 0.5, 0.2, 0.1, 127.0, 0.0, 0.0, 0.0, 0.0);
        assert!((m1 - 0.8).abs() < 1e-4);

        // Ring Mod (Osc 1 * Osc 2)
        let m_ring = mix_signals(0.5, 0.4, 0.0, 0.0, 0.0, 0.0, 0.0, 127.0, 0.0);
        assert!((m_ring - 0.2).abs() < 1e-4); // 0.5 * 0.4 = 0.2

        // Noise Mix
        let m_noise = mix_signals(0.0, 0.0, 0.0, 0.75, 0.0, 0.0, 0.0, 0.0, 127.0);
        assert!((m_noise - 0.75).abs() < 1e-4);
    }

    #[test]
    fn test_analogue_state_variable_filter_modes_and_slopes() {
        let cutoff = 1000.0;
        let q = 1.0;

        let lp_12 = calculate_filter_response_curve(
            cutoff,
            q,
            FilterMode::LowPass,
            FilterSlope::Slope12dB,
            64,
        );
        let lp_24 = calculate_filter_response_curve(
            cutoff,
            q,
            FilterMode::LowPass,
            FilterSlope::Slope24dB,
            64,
        );

        // At low frequencies (e.g. 50 Hz), LowPass magnitude should be close to 0 dB
        assert!(lp_12[0].magnitude_db > -3.0);
        assert!(lp_24[0].magnitude_db > -3.0);

        // At high frequencies (e.g. 10 kHz), 24dB slope must attenuate much more than 12dB slope!
        let high_idx = 60;
        assert!(
            lp_24[high_idx].magnitude_db < lp_12[high_idx].magnitude_db - 15.0,
            "24dB filter slope must provide substantially steeper attenuation than 12dB slope"
        );

        // Test BandPass mode: peak should be near cutoff (1000 Hz)
        let bp_curve = calculate_filter_response_curve(
            cutoff,
            q,
            FilterMode::BandPass,
            FilterSlope::Slope12dB,
            64,
        );
        let mut max_bp_db = -999.0;
        let mut peak_freq = 0.0;
        for p in &bp_curve {
            if p.magnitude_db > max_bp_db {
                max_bp_db = p.magnitude_db;
                peak_freq = p.frequency_hz;
            }
        }
        assert!(
            (peak_freq - 1000.0).abs() < 300.0,
            "BandPass peak should center near cutoff frequency"
        );

        // Test HighPass mode: high frequencies pass with ~0 dB, low frequencies heavily attenuated
        let hp_curve = calculate_filter_response_curve(
            cutoff,
            q,
            FilterMode::HighPass,
            FilterSlope::Slope12dB,
            64,
        );
        assert!(hp_curve[0].magnitude_db < -20.0);
        assert!(hp_curve.last().unwrap().magnitude_db > -3.0);
    }

    #[test]
    fn test_filter_key_tracking_fm_and_mod_env() {
        let mut engine = PeakSynthEngine::new();
        engine.set_cc(29, 64).expect("cutoff"); // ~632 Hz
        engine.set_cc(75, 127).expect("key track full"); // Full 1:1 key tracking

        // Note 60 is middle C (0 offset)
        let f60 = engine.calculate_osc_freq(0, 60, 0.0);
        let f72 = engine.calculate_osc_freq(0, 72, 0.0);
        assert!((f72 / f60 - 2.0).abs() < 1e-4);

        // Test Osc 3 -> Filter FM parameter
        engine.set_cc(76, 90).expect("filter fm");
        assert_eq!(engine.get_param("osc_3_to_filter"), 90.0);

        // Test Mod Env 1 -> Filter parameter
        engine.set_cc(78, 40).expect("mod env 1 to filter");
        assert_eq!(engine.get_param("mod_1_to_filter"), 40.0);
    }

    #[test]
    fn test_filter_pre_overdrive_and_post_drive() {
        let mut filter = StateVariableFilter::default();

        // Clean passthrough
        let clean = filter.process(
            0.5,
            10000.0,
            0.707,
            FilterMode::LowPass,
            FilterSlope::Slope12dB,
            0.0,
            0.0,
            48000.0,
        );

        // Reset filter
        filter.reset();

        // With overdrive (CC80) and post drive (CC36)
        let driven = filter.process(
            0.5,
            10000.0,
            0.707,
            FilterMode::LowPass,
            FilterSlope::Slope12dB,
            100.0,
            100.0,
            48000.0,
        );

        // Driven signal undergoes nonlinear saturation
        assert!(driven.abs() <= 1.0);
        assert_ne!(clean, driven);
    }

    #[test]
    fn test_command_history_undo_redo_integration() {
        let mut engine = PeakSynthEngine::new();
        assert_eq!(engine.get_param("filter_cutoff"), 100.0);

        // Modify cutoff through set_cc
        engine.set_cc(29, 50).expect("set cutoff");
        assert_eq!(engine.get_param("filter_cutoff"), 50.0);
        assert!(engine.can_undo());
        assert!(!engine.can_redo());

        // Modify resonance through set_cc
        engine.set_cc(79, 90).expect("set resonance");
        assert_eq!(engine.get_param("filter_resonance"), 90.0);

        // Undo resonance
        let desc = engine.undo().expect("undo resonance");
        assert!(desc.is_some());
        assert_eq!(engine.get_param("filter_resonance"), 20.0);
        assert_eq!(engine.get_param("filter_cutoff"), 50.0);
        assert!(engine.can_redo());

        // Undo cutoff
        engine.undo().expect("undo cutoff");
        assert_eq!(engine.get_param("filter_cutoff"), 100.0);

        // Redo cutoff
        engine.redo().expect("redo cutoff");
        assert_eq!(engine.get_param("filter_cutoff"), 50.0);

        // Redo resonance
        engine.redo().expect("redo resonance");
        assert_eq!(engine.get_param("filter_resonance"), 90.0);
    }

    #[test]
    fn test_8_voice_polyphonic_allocation_and_rendering() {
        let mut engine = PeakSynthEngine::new();
        assert_eq!(engine.active_voice_count(), 0);

        // Trigger 8 distinct notes
        for note in 60..68 {
            engine.note_on(note, 100);
        }
        assert_eq!(engine.active_voice_count(), 8);

        // 9th note triggers voice-stealing of the oldest note (60)
        engine.note_on(68, 110);
        assert_eq!(engine.active_voice_count(), 8);
        let notes: Vec<u8> = engine.voices.iter().map(|v| v.note).collect();
        assert!(!notes.contains(&60), "Oldest note 60 should have been stolen");
        assert!(notes.contains(&68), "New note 68 must be sounding");

        // Release note 61
        engine.note_off(61);
        assert_eq!(engine.active_voice_count(), 7);

        // Render audio buffer
        let mut buffer = [0.0f32; 128];
        engine.render_block(&mut buffer);

        // Buffer must contain non-zero bounded audio samples
        let mut has_audio = false;
        for &sample in &buffer {
            assert!(sample >= -1.05 && sample <= 1.05, "Sample must be bounded");
            assert!(!sample.is_nan(), "Sample must not be NaN");
            if sample.abs() > 1e-4 {
                has_audio = true;
            }
        }
        assert!(has_audio, "Rendered block must contain audio signal");

        // All notes off
        engine.all_notes_off();
        assert_eq!(engine.active_voice_count(), 0);
    }
}
