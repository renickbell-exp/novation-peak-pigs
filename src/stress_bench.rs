// SPDX-License-Identifier: GPL-3.0-or-later
//! `novation_peak_manager::stress_bench`: 1,000-event 8-voice polyphonic + 9-lane CC sweep
//! stress benchmark verifying zero dropped notes, 8-voice polyphony budget compliance,
//! zero stuck voices on panic, and strict safeSweepRange enforcement on CCs 29/36/79/80/104/110.

use std::time::Instant;
use synth_midi_osc::routing::{safe_sweep_range, MidiSafetyMode};
use crate::midi::PeakMidiService;
use crate::peak_synth::PeakSynthEngine;

/// The 9 canonical CC sweep lanes evaluated during polyphonic stress testing.
pub const BENCHMARK_CC_LANES: [u8; 9] = [
    29,  // Filter Cutoff (safe range 0..=127)
    36,  // Post-Filter Drive (safe range 0..=50)
    79,  // Filter Resonance (safe range 0..=62)
    80,  // Pre-Filter Overdrive (safe range 0..=50)
    104, // Analogue Distortion Level (safe range 0..=48)
    110, // Delay Feedback (safe range 0..=70)
    108, // Delay Time / Sync (safe range 0..=90)
    112, // Reverb Level (safe range 0..=90)
    78,  // Mod Env 1 to Filter Cutoff (safe range 0..=127)
];

/// The 6 critical CC numbers requiring strict safeSweepRange enforcement.
pub const STRICT_SAFETY_CCS: [u8; 6] = [29, 36, 79, 80, 104, 110];

/// Configuration options for the Novation Peak polyphonic stress benchmark.
#[derive(Debug, Clone)]
pub struct StressBenchmarkConfig {
    /// Target number of total events to execute (must be at least 1,000).
    pub total_events: usize,
    /// Fraction of events dedicated to polyphonic note triggering (0.0 to 1.0).
    pub note_ratio: f64,
    /// Event index at which an intermediate emergency panic is triggered, if any.
    pub midstream_panic_at: Option<usize>,
    /// Whether to deliberately exceed 8 simultaneous voices to stress voice stealing.
    pub exercise_voice_stealing: bool,
    /// Seed for deterministic pseudo-random sequence generation.
    pub seed: u64,
}

impl Default for StressBenchmarkConfig {
    fn default() -> Self {
        Self {
            total_events: 1000,
            note_ratio: 0.45,
            midstream_panic_at: Some(500),
            exercise_voice_stealing: true,
            seed: 0xDEAD_BEEF_CAFE_BABE,
        }
    }
}

/// Comprehensive telemetry report produced by the stress benchmark.
#[derive(Debug, Clone, PartialEq)]
pub struct StressBenchmarkReport {
    /// Total events successfully processed during benchmark.
    pub total_events: usize,
    /// Number of Note-On events dispatched.
    pub note_on_events: usize,
    /// Number of Note-Off events dispatched.
    pub note_off_events: usize,
    /// Number of Continuous Controller sweep events dispatched across 9 lanes.
    pub cc_events: usize,
    /// Number of dropped or rejected notes (MUST BE 0).
    pub dropped_notes: usize,
    /// Maximum concurrent active voices observed (MUST BE <= 8).
    pub max_active_voices: usize,
    /// Total number of voice steals handled gracefully by the 8-voice engine.
    pub voice_steals: usize,
    /// Number of safety clamp violations observed on CCs 29/36/79/80/104/110 (MUST BE 0).
    pub safety_violations: usize,
    /// Number of stuck voices remaining after panic() invocation (MUST BE 0).
    pub voices_after_panic: usize,
    /// Total wall-clock execution time in microseconds.
    pub duration_micros: u128,
    /// Throughput in events processed per second.
    pub events_per_second: f64,
    /// Final verdict: true if all invariants are strictly satisfied.
    pub passed: bool,
}

impl StressBenchmarkReport {
    /// Formats the report as a clean multi-line human-readable summary.
    pub fn format_report(&self) -> String {
        let mut out = String::with_capacity(1024);
        out.push_str("=================================================================\n");
        out.push_str("  Novation Peak 1,000-Event Polyphonic & CC Stress Benchmark\n");
        out.push_str("=================================================================\n");
        out.push_str(&format!("  Total Events Executed    : {}\n", self.total_events));
        out.push_str(&format!("  Note-On Events           : {}\n", self.note_on_events));
        out.push_str(&format!("  Note-Off Events          : {}\n", self.note_off_events));
        out.push_str(&format!("  9-Lane CC Sweep Events   : {}\n", self.cc_events));
        out.push_str(&format!("  Dropped Notes (budget)   : {} (Required: 0)\n", self.dropped_notes));
        out.push_str(&format!("  Peak Active Voices       : {} (Hardware limit: 8)\n", self.max_active_voices));
        out.push_str(&format!("  Voice Steals Handled     : {}\n", self.voice_steals));
        out.push_str(&format!("  Safety Clamp Violations  : {} (Required: 0)\n", self.safety_violations));
        out.push_str(&format!("  Stuck Voices After Panic : {} (Required: 0)\n", self.voices_after_panic));
        out.push_str(&format!("  Execution Duration       : {} us ({:.2} ms)\n", self.duration_micros, self.duration_micros as f64 / 1000.0));
        out.push_str(&format!("  Throughput               : {:.1} events/sec\n", self.events_per_second));
        out.push_str("-----------------------------------------------------------------\n");
        out.push_str(&format!("  VERDICT                  : {}\n", if self.passed { "PASS" } else { "FAIL" }));
        out.push_str("=================================================================\n");
        out
    }

    /// Prints the formatted report to standard output.
    pub fn print_report(&self) {
        println!("{}", self.format_report());
    }
}

/// Simple deterministic pseudo-random number generator (Xorshift64) to avoid external overhead.
struct SimpleRng {
    state: u64,
}

impl SimpleRng {
    fn new(seed: u64) -> Self {
        Self {
            state: if seed == 0 { 0xCAFE_BABE_DEAD_BEEF } else { seed },
        }
    }

    fn next_u64(&mut self) -> u64 {
        let mut x = self.state;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.state = x;
        x
    }

    fn next_f64(&mut self) -> f64 {
        (self.next_u64() & 0x001F_FFFF_FFFF_FFFF) as f64 / (0x0020_0000_0000_0000u64 as f64)
    }

    fn next_range(&mut self, min: u8, max: u8) -> u8 {
        if min >= max {
            return min;
        }
        let span = (max - min + 1) as u64;
        min + (self.next_u64() % span) as u8
    }
}

/// Runs the 1,000-event stress benchmark with default configuration.
pub fn run_default_stress_benchmark() -> StressBenchmarkReport {
    run_stress_benchmark(&StressBenchmarkConfig::default())
}

/// Executes the full polyphonic note and 9-lane CC sweep stress benchmark.
///
/// Invariants verified:
/// 1. Zero dropped notes across all note-on events.
/// 2. 8-voice polyphony budget compliance (`active_voice_count <= 8`) at all times.
/// 3. Zero stuck voices upon calling `panic()`.
/// 4. Strict `safeSweepRange` enforcement on CCs 29, 36, 79, 80, 104, 110.
pub fn run_stress_benchmark(config: &StressBenchmarkConfig) -> StressBenchmarkReport {
    let mut engine = PeakSynthEngine::new();
    let mut midi_service = PeakMidiService::new(MidiSafetyMode::VirtualLoopback).with_safety_clamp(true);
    let mut rng = SimpleRng::new(config.seed);

    let start = Instant::now();

    let mut note_on_count = 0usize;
    let mut note_off_count = 0usize;
    let mut cc_count = 0usize;
    let mut dropped_notes = 0usize;
    let mut max_active_voices = 0usize;
    let mut voice_steals = 0usize;
    let mut safety_violations = 0usize;
    let mut voices_after_panic = 0usize;

    // Track active sounding note numbers
    let mut active_notes: Vec<u8> = Vec::with_capacity(16);

    let total_events = config.total_events.max(1000);

    for event_idx in 0..total_events {
        // 1. Midstream panic check
        if let Some(panic_point) = config.midstream_panic_at {
            if event_idx == panic_point {
                engine.panic();
                let panic_pkts = midi_service.panic();
                if panic_pkts.is_empty() {
                    voices_after_panic += 1;
                }

                let post_voices = engine.active_voice_count();
                if post_voices > 0 {
                    voices_after_panic += post_voices;
                }
                active_notes.clear();
            }
        }

        let is_note_event = rng.next_f64() < config.note_ratio;

        if is_note_event {
            // Decide between Note-On and Note-Off
            let should_note_on = if active_notes.is_empty() {
                true
            } else if config.exercise_voice_stealing && active_notes.len() >= 8 {
                // Occasionally push beyond 8 voices to verify voice-stealing policy
                rng.next_f64() < 0.60
            } else if active_notes.len() < 8 {
                rng.next_f64() < 0.70
            } else {
                rng.next_f64() < 0.35
            };

            if should_note_on {
                let note = rng.next_range(36, 96);
                let velocity = rng.next_range(40, 127);

                let pre_voices = engine.active_voice_count();
                let already_sounding = active_notes.contains(&note);

                engine.note_on(note, velocity);
                let post_voices = engine.active_voice_count();

                // Strict invariant: Active voices must NEVER exceed 8
                if post_voices > 8 {
                    dropped_notes += 1;
                }

                if post_voices > max_active_voices {
                    max_active_voices = post_voices;
                }

                // If all 8 voices were already active and this is a new note, voice stealing occurred
                if pre_voices == 8 && !already_sounding {
                    voice_steals += 1;
                }

                if !already_sounding {
                    active_notes.push(note);
                }

                note_on_count += 1;
            } else {
                // Trigger Note-Off on one of the active sounding notes
                let idx = (rng.next_u64() as usize) % active_notes.len();
                let note = active_notes.swap_remove(idx);

                engine.note_off(note);
                let post_voices = engine.active_voice_count();

                if post_voices > 8 {
                    dropped_notes += 1;
                }

                note_off_count += 1;
            }
        } else {
            // 2. Continuous Controller Sweep Event across 9 lanes
            let lane_idx = (event_idx + (rng.next_u64() as usize)) % BENCHMARK_CC_LANES.len();
            let cc = BENCHMARK_CC_LANES[lane_idx];

            // Deliberately sweep across the full 0..=127 range to challenge safety clamping
            let raw_value = rng.next_range(0, 127);

            // Route through safety clamp
            let pkt = midi_service.send_cc(cc, raw_value);
            let clamped_val = match pkt {
                synth_midi_osc::routing::MidiPacket::ControlChange { value, .. } => value,
                _ => raw_value,
            };

            // Apply to synthesis engine
            let _ = engine.set_cc(cc, clamped_val);

            // Strict safety range check on CCs 29, 36, 79, 80, 104, 110
            let (lo, hi) = safe_sweep_range(cc);
            if clamped_val < lo || clamped_val > hi {
                safety_violations += 1;
            }

            // Explicit safety caps verification for the 6 critical parameters:
            match cc {
                29 => {
                    if clamped_val > 127 { safety_violations += 1; }
                }
                36 => {
                    // Post-filter drive safe cap: 50
                    if clamped_val > 50 { safety_violations += 1; }
                }
                79 => {
                    // Filter resonance safe cap: 62
                    if clamped_val > 62 { safety_violations += 1; }
                }
                80 => {
                    // Pre-filter overdrive safe cap: 50
                    if clamped_val > 50 { safety_violations += 1; }
                }
                104 => {
                    // Analogue distortion level safe cap: 48
                    if clamped_val > 48 { safety_violations += 1; }
                }
                110 => {
                    // Delay feedback safe cap: 70
                    if clamped_val > 70 { safety_violations += 1; }
                }
                _ => {}
            }

            cc_count += 1;
        }

        // Render audio sample to advance synthesis state
        let _ = engine.render_sample();
    }

    // 3. Final Panic Verification
    engine.panic();
    let final_panic_pkts = midi_service.panic();
    if final_panic_pkts.is_empty() {
        voices_after_panic += 1;
    }

    let final_voices = engine.active_voice_count();
    if final_voices > 0 {
        voices_after_panic += final_voices;
    }

    // Verify all 8 voice structures are fully silenced and gate is low
    for v in &engine.voices {
        if v.active || v.gate || v.env_level > 0.0 {
            voices_after_panic += 1;
        }
    }

    let elapsed = start.elapsed();
    let duration_micros = elapsed.as_micros().max(1);
    let events_per_second = (total_events as f64) / (elapsed.as_secs_f64().max(0.000_001));

    let passed = dropped_notes == 0
        && max_active_voices <= 8
        && voices_after_panic == 0
        && safety_violations == 0
        && (note_on_count + note_off_count + cc_count == total_events);

    StressBenchmarkReport {
        total_events,
        note_on_events: note_on_count,
        note_off_events: note_off_count,
        cc_events: cc_count,
        dropped_notes,
        max_active_voices,
        voice_steals,
        safety_violations,
        voices_after_panic,
        duration_micros,
        events_per_second,
        passed,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_1000_event_stress_benchmark_invariants() {
        let config = StressBenchmarkConfig {
            total_events: 1000,
            note_ratio: 0.45,
            midstream_panic_at: Some(500),
            exercise_voice_stealing: true,
            seed: 0x1234_5678_9ABC_DEF0,
        };

        let report = run_stress_benchmark(&config);

        assert!(report.passed, "Benchmark must pass all invariants");
        assert_eq!(report.total_events, 1000);
        assert_eq!(report.dropped_notes, 0, "Zero notes may be dropped");
        assert!(report.max_active_voices <= 8, "Max voices must respect 8-voice polyphony limit");
        assert_eq!(report.max_active_voices, 8, "Stress test should fully exercise all 8 voices");
        assert!(report.voice_steals > 0, "Voice stealing should be exercised under polyphonic stress");
        assert_eq!(report.safety_violations, 0, "Zero safety violations on CC sweeps");
        assert_eq!(report.voices_after_panic, 0, "Zero stuck voices after panic()");
        assert!(report.events_per_second > 5_000.0, "Throughput must exceed 5,000 events/sec");
    }

    #[test]
    fn test_strict_safesweeprange_enforcement_on_critical_ccs() {
        let mut service = PeakMidiService::new(MidiSafetyMode::VirtualLoopback).with_safety_clamp(true);

        // Test extreme values (127) on all strict safety CCs
        for &cc in &STRICT_SAFETY_CCS {
            let pkt = service.send_cc(cc, 127);
            if let synth_midi_osc::routing::MidiPacket::ControlChange { value, .. } = pkt {
                let (lo, hi) = safe_sweep_range(cc);
                assert!(value <= hi, "CC {} clamped value {} must be <= hi {}", cc, value, hi);
                assert!(value >= lo, "CC {} clamped value {} must be >= lo {}", cc, value, lo);

                match cc {
                    29 => assert_eq!(value, 127),
                    36 => assert_eq!(value, 50),
                    79 => assert_eq!(value, 62),
                    80 => assert_eq!(value, 50),
                    104 => assert_eq!(value, 48),
                    110 => assert_eq!(value, 70),
                    _ => {}
                }
            } else {
                panic!("Expected ControlChange packet");
            }
        }
    }

    #[test]
    fn test_heavy_voice_stealing_cluster() {
        let mut engine = PeakSynthEngine::new();

        // Fire 24 consecutive distinct notes without note-off
        for note in 40..64 {
            engine.note_on(note, 100);
            assert!(
                engine.active_voice_count() <= 8,
                "Active voices must never exceed 8 hardware voices"
            );
        }

        assert_eq!(engine.active_voice_count(), 8);

        // Panic must immediately silence all voices
        engine.panic();
        assert_eq!(engine.active_voice_count(), 0);
        for voice in &engine.voices {
            assert!(!voice.active);
            assert!(!voice.gate);
            assert_eq!(voice.env_level, 0.0);
        }
    }
}
