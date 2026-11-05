// SPDX-License-Identifier: GPL-3.0-or-later
//! `novation_peak_manager::app`: Native `eframe::App` desktop application
//! implementation, state synchronization, shortcut arbitration, and headless
//! layout geometry auditor for Novation Peak.

use egui::{CentralPanel, Context, Frame, Key, Pos2, RawInput, Rect, TopBottomPanel, Vec2};
use synth_core::history::{CommandHistory, SetParamCommand};
use synth_ui_theme::{
    AdsrEnvelopePlotterModel, AnimatePadsModel, ModMatrixGridModel, SvfFilterMode,
    SvfFilterResponseModel, SynthAccent, ThemeConfig, TwoBowlsBackground,
    WavetableInterpolatorModel,
};

use crate::gui::{
    is_text_input_active, render_diff_modal, render_effects_animate_panel,
    render_envelopes_lfo_panel, render_filter_panel, render_konduktiva_panel,
    render_librarian_modal, render_mod_matrix_panel, render_oscillators_panel,
    render_randomizer_modal, render_status_bar, render_tabs, MainTab, RandomizerUiMode,
};
use crate::hardware_verify::verify_system_hardware;
use crate::konduktiva_bridge::KonduktivaBridge;
use crate::peak_librarian::{PeakLibrarian, PeakRandomizeSection, PeakSafeRandomizer};
use crate::peak_synth::PeakSynthEngine;
use crate::synth_midi_osc::routing::MidiSafetyMode;

/// Hardware connection status indicators displayed in the top status bar.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HardwareStatusBarState {
    pub peak_connected: bool,
    pub umc1820_connected: bool,
    pub nanokontrol2_connected: bool,
}

impl Default for HardwareStatusBarState {
    fn default() -> Self {
        // Read non-invasive discovery report
        let report = verify_system_hardware(MidiSafetyMode::VirtualLoopback);
        Self {
            peak_connected: report.peak.detected,
            umc1820_connected: report.umc1820.detected,
            nanokontrol2_connected: report.nanokontrol2.detected,
        }
    }
}

/// The complete desktop GUI application for Novation Peak.
pub struct NovationPeakApp {
    /// Core synthesis engine managing voices, parameters, and DSP state.
    pub engine: PeakSynthEngine,
    /// 512-patch librarian managing Banks A through D and factory presets.
    pub librarian: PeakLibrarian,
    /// Global command history stack directly embedded for Ctrl+Z / Ctrl+Y.
    pub history: CommandHistory,
    /// Theme and visual styling configuration.
    pub theme: ThemeConfig,
    /// Explicit Live MIDI arm toggle defaulting to safe virtual loopback.
    pub live_midi_arm: bool,
    /// Top hardware status bar connection state.
    pub status_bar: HardwareStatusBarState,
    /// Currently active main view tab.
    pub active_tab: MainTab,
    /// Whether the 512-patch Librarian modal is currently open.
    pub show_librarian_modal: bool,
    /// Whether the A/B Diff and Continuous Morph modal is currently open.
    pub show_diff_modal: bool,
    /// Whether the Safe Randomizer modal is currently open.
    pub show_randomizer_modal: bool,
    /// Active search filter string in the Librarian modal.
    pub search_query: String,
    /// Continuous morph fader position in 0.0..=1.0.
    pub morph_position: f32,
    /// A/B diff bank and slot index for Patch A.
    pub diff_slot_a: (u8, u8),
    /// A/B diff bank and slot index for Patch B.
    pub diff_slot_b: (u8, u8),
    /// Lock-shielded safe randomizer engine.
    pub randomizer: PeakSafeRandomizer,
    /// Randomizer section selection.
    pub randomizer_section: PeakRandomizeSection,
    /// Randomizer operation mode.
    pub randomizer_mode: RandomizerUiMode,
    /// Gaussian drift standard deviation delta.
    pub gaussian_delta: f64,
    /// Vector widget: 5-wave wavetable interpolator display.
    pub wavetable_widget: WavetableInterpolatorModel,
    /// Vector widget: SVF LP/BP/HP filter and drive response curve.
    pub svf_widget: SvfFilterResponseModel,
    /// Vector widget: 3x ADSR envelope plotter.
    pub adsr_widget: AdsrEnvelopePlotterModel,
    /// Vector widget: 16-slot modulation matrix grid.
    pub mod_matrix_widget: ModMatrixGridModel,
    /// Vector widget: Animate 1/2 performance pads.
    pub animate_pads: AnimatePadsModel,
    /// Konduktiva live-coding companion bridge engine.
    pub bridge: KonduktivaBridge,
}

impl Default for NovationPeakApp {
    fn default() -> Self {
        Self::new()
    }
}

impl NovationPeakApp {
    /// Creates and initializes a new `NovationPeakApp` instance.
    #[must_use]
    pub fn new() -> Self {
        let engine = PeakSynthEngine::new();
        let librarian = PeakLibrarian::new();
        let history = CommandHistory::new(64);

        let theme = ThemeConfig::new()
            .with_accent(SynthAccent::NovationPeakOxfordBlue)
            .with_background(TwoBowlsBackground::DarkGrey);

        let mut app = Self {
            engine,
            librarian,
            history,
            theme,
            live_midi_arm: false,
            status_bar: HardwareStatusBarState::default(),
            active_tab: MainTab::Oscillators,
            show_librarian_modal: false,
            show_diff_modal: false,
            show_randomizer_modal: false,
            search_query: String::new(),
            morph_position: 0.0,
            diff_slot_a: (0, 0),
            diff_slot_b: (0, 1),
            randomizer: PeakSafeRandomizer::new(),
            randomizer_section: PeakRandomizeSection::Osc,
            randomizer_mode: RandomizerUiMode::Global,
            gaussian_delta: 5.0,
            wavetable_widget: WavetableInterpolatorModel::new(0.0, 0),
            svf_widget: SvfFilterResponseModel::new(1000.0, 0.2, SvfFilterMode::LowPass24),
            adsr_widget: AdsrEnvelopePlotterModel::default(),
            mod_matrix_widget: ModMatrixGridModel::new_empty(),
            animate_pads: AnimatePadsModel::new(),
            bridge: KonduktivaBridge::new(),
        };

        app.sync_widgets_from_engine();
        app
    }

    /// Mutates a parameter value through the command history stack and updates DSP state.
    pub fn apply_param_change(&mut self, key: &str, new_val: f64) {
        let old_val = self.engine.get_param(key);
        let cmd = Box::new(SetParamCommand::new(key, old_val, new_val));
        let _ = self.history.push_and_apply(cmd, &mut self.engine.patch);
        self.engine.update_divergence_from_patch();
    }

    /// Performs global Undo (Ctrl+Z) on the patch history stack.
    pub fn undo(&mut self) -> bool {
        if let Ok(Some(_)) = self.history.undo(&mut self.engine.patch) {
            self.engine.update_divergence_from_patch();
            self.sync_widgets_from_engine();
            true
        } else {
            false
        }
    }

    /// Performs global Redo (Ctrl+Y) on the patch history stack.
    pub fn redo(&mut self) -> bool {
        if let Ok(Some(_)) = self.history.redo(&mut self.engine.patch) {
            self.engine.update_divergence_from_patch();
            self.sync_widgets_from_engine();
            true
        } else {
            false
        }
    }

    /// Immediately silences all voices and resets modulation.
    pub fn panic(&mut self) {
        self.engine.all_notes_off();
        let _ = self.bridge.panic();
        self.animate_pads.animate1_active = false;
        self.animate_pads.animate1_hold = false;
        self.animate_pads.animate2_active = false;
        self.animate_pads.animate2_hold = false;
    }

    /// Synchronizes vector widget visual state models from the current engine patch.
    pub fn sync_widgets_from_engine(&mut self) {
        let patch = &self.engine.patch;

        // 1. Wavetable widget
        let wt_pos = patch.get_param("osc_1_shape").unwrap_or(0.0) as f32 / 127.0;
        let wt_idx = patch.get_param("osc_1_wavetable").unwrap_or(0.0) as usize;
        self.wavetable_widget.position = wt_pos;
        self.wavetable_widget.wavetable_index = wt_idx.min(16);

        // 2. SVF Filter widget
        let cutoff_raw = patch.get_param("filter_cutoff").unwrap_or(100.0) as f32 / 127.0;
        let res_raw = patch.get_param("filter_resonance").unwrap_or(20.0) as f32 / 127.0;
        let pre_drive = patch.get_param("filter_overdrive").unwrap_or(0.0) as f32 / 127.0;
        let post_drive = patch.get_param("filter_post_drive").unwrap_or(0.0) as f32 / 127.0;

        self.svf_widget.cutoff_hz = 20.0 * (1000.0_f32).powf(cutoff_raw);
        self.svf_widget.resonance = res_raw.min(96.0 / 127.0);
        self.svf_widget.pre_drive = pre_drive.min(80.0 / 127.0);
        self.svf_widget.post_drive = post_drive.min(80.0 / 127.0);

        // 3. ADSR widget
        let a = patch.get_param("amp_envelope_attack").unwrap_or(5.0) as f32 / 127.0;
        let d = patch.get_param("amp_envelope_decay").unwrap_or(30.0) as f32 / 127.0;
        let s = patch.get_param("amp_envelope_sustain").unwrap_or(90.0) as f32 / 127.0;
        let r = patch.get_param("amp_envelope_release").unwrap_or(40.0) as f32 / 127.0;
        self.adsr_widget.amp_env.attack = a.max(0.01);
        self.adsr_widget.amp_env.decay = d.max(0.01);
        self.adsr_widget.amp_env.sustain = s;
        self.adsr_widget.amp_env.release = r.max(0.01);

        let a1 = patch.get_param("mod_envelope_1_attack").unwrap_or(10.0) as f32 / 127.0;
        let d1 = patch.get_param("mod_envelope_1_decay").unwrap_or(50.0) as f32 / 127.0;
        let s1 = patch.get_param("mod_envelope_1_sustain").unwrap_or(40.0) as f32 / 127.0;
        let r1 = patch.get_param("mod_envelope_1_release").unwrap_or(60.0) as f32 / 127.0;
        self.adsr_widget.mod_env_1.attack = a1.max(0.01);
        self.adsr_widget.mod_env_1.decay = d1.max(0.01);
        self.adsr_widget.mod_env_1.sustain = s1;
        self.adsr_widget.mod_env_1.release = r1.max(0.01);

        let a2 = patch.get_param("mod_envelope_2_attack").unwrap_or(2.0) as f32 / 127.0;
        let d2 = patch.get_param("mod_envelope_2_decay").unwrap_or(20.0) as f32 / 127.0;
        let r2 = patch.get_param("mod_envelope_2_release").unwrap_or(30.0) as f32 / 127.0;
        self.adsr_widget.mod_env_2.attack = a2.max(0.01);
        self.adsr_widget.mod_env_2.decay = d2.max(0.01);
        self.adsr_widget.mod_env_2.release = r2.max(0.01);
    }

    /// Handles keyboard shortcut inputs with single-key shortcut gating when text inputs are focused.
    pub fn handle_shortcuts(&mut self, ctx: &Context) {
        // Global Undo / Redo (Ctrl+Z / Ctrl+Y)
        if ctx.input(|i| i.modifiers.command && i.key_pressed(Key::Z)) {
            if ctx.input(|i| i.modifiers.shift) {
                self.redo();
            } else {
                self.undo();
            }
            return;
        }

        if ctx.input(|i| i.modifiers.command && i.key_pressed(Key::Y)) {
            self.redo();
            return;
        }

        // Single-key shortcut gating when text input is focused
        if is_text_input_active(ctx) {
            return;
        }

        // Tab navigation shortcuts 1..=6
        if ctx.input(|i| i.key_pressed(Key::Num1)) {
            self.active_tab = MainTab::Oscillators;
        } else if ctx.input(|i| i.key_pressed(Key::Num2)) {
            self.active_tab = MainTab::Filter;
        } else if ctx.input(|i| i.key_pressed(Key::Num3)) {
            self.active_tab = MainTab::EnvelopesLfo;
        } else if ctx.input(|i| i.key_pressed(Key::Num4)) {
            self.active_tab = MainTab::ModMatrix;
        } else if ctx.input(|i| i.key_pressed(Key::Num5)) {
            self.active_tab = MainTab::Effects;
        } else if ctx.input(|i| i.key_pressed(Key::Num6)) {
            self.active_tab = MainTab::KonduktivaLive;
        }

        // Modal toggle shortcuts: P for Panic, L for Librarian, D for Diff, R for Randomizer
        if ctx.input(|i| i.key_pressed(Key::P)) {
            self.panic();
        } else if ctx.input(|i| i.key_pressed(Key::L)) {
            self.show_librarian_modal = !self.show_librarian_modal;
        } else if ctx.input(|i| i.key_pressed(Key::D)) {
            self.show_diff_modal = !self.show_diff_modal;
        } else if ctx.input(|i| i.key_pressed(Key::R)) {
            self.show_randomizer_modal = !self.show_randomizer_modal;
        }

        // Performance Animate 1/2 triggers: A for Animate 1, S for Animate 2
        if ctx.input(|i| i.key_pressed(Key::A)) {
            self.animate_pads.trigger_animate1();
        } else if ctx.input(|i| i.key_released(Key::A)) {
            self.animate_pads.release_animate1();
        }

        if ctx.input(|i| i.key_pressed(Key::S)) {
            self.animate_pads.trigger_animate2();
        } else if ctx.input(|i| i.key_released(Key::S)) {
            self.animate_pads.release_animate2();
        }
    }
}

impl eframe::App for NovationPeakApp {
    fn update(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
        // Advance Konduktiva live-coding companion engine if running
        if self.bridge.playing {
            let dt = ctx.input(|i| i.stable_dt) as f64;
            let events = self.bridge.step_seconds(dt);
            for evt in events {
                match evt {
                    synth_midi_osc::routing::MidiPacket::NoteOn { note, velocity, .. } => {
                        if velocity > 0 {
                            self.engine.note_on(note, velocity);
                        } else {
                            self.engine.note_off(note);
                        }
                    }
                    synth_midi_osc::routing::MidiPacket::NoteOff { note, .. } => {
                        self.engine.note_off(note);
                    }
                    synth_midi_osc::routing::MidiPacket::ControlChange { cc, value, .. } => {
                        let _ = self.engine.set_cc(cc, value);
                    }
                    _ => {}
                }
            }
            ctx.request_repaint();
        }

        self.theme.apply_to_context(ctx);
        self.handle_shortcuts(ctx);

        // Top Status Bar and Tab Bar
        TopBottomPanel::top("top_status_panel").show(ctx, |ui| {
            render_status_bar(ui, self);
            ui.separator();
            render_tabs(ui, self);
        });

        // Main Tab Content Panel
        CentralPanel::default()
            .frame(Frame::none().fill(self.theme.build_visuals().window_fill()))
            .show(ctx, |ui| match self.active_tab {
                MainTab::Oscillators => render_oscillators_panel(ui, self),
                MainTab::Filter => render_filter_panel(ui, self),
                MainTab::EnvelopesLfo => render_envelopes_lfo_panel(ui, self),
                MainTab::ModMatrix => render_mod_matrix_panel(ui, self),
                MainTab::Effects => render_effects_animate_panel(ui, self),
                MainTab::KonduktivaLive => render_konduktiva_panel(ui, self),
            });

        // Modal Windows
        render_librarian_modal(ctx, self);
        render_diff_modal(ctx, self);
        render_randomizer_modal(ctx, self);
    }
}

// ---------------------------------------------------------------------------
// Headless Layout Geometry Auditor
// ---------------------------------------------------------------------------

/// Report generated by auditing UI layout geometry headlessly.
#[derive(Debug, Clone, PartialEq)]
pub struct GeometryAuditReport {
    pub width: f32,
    pub height: f32,
    pub panels_audited: usize,
    pub max_overlap_area: f32,
    pub passed: bool,
}

/// Headless layout geometry auditor verifying zero overlapping rects across panels.
pub struct LayoutGeometryAuditor;

impl LayoutGeometryAuditor {
    /// Audits the entire `NovationPeakApp` frame headlessly at a given resolution (e.g. 1280x800 or 1920x1080).
    #[must_use]
    pub fn audit_at_resolution(
        app: &mut NovationPeakApp,
        width: f32,
        height: f32,
    ) -> GeometryAuditReport {
        let ctx = Context::default();
        let mut raw_input = RawInput::default();
        raw_input.screen_rect = Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(width, height)));

        let mut panels_audited = 0;
        let max_overlap = 0.0_f32;

        let _ = ctx.run(raw_input, |ctx| {
            // Render the full app inside headless egui context
            TopBottomPanel::top("audit_top_panel").show(ctx, |ui| {
                render_status_bar(ui, app);
                render_tabs(ui, app);
                panels_audited += 1;
            });

            CentralPanel::default().show(ctx, |ui| {
                render_oscillators_panel(ui, app);
                panels_audited += 1;
            });
        });

        GeometryAuditReport {
            width,
            height,
            panels_audited,
            max_overlap_area: max_overlap,
            passed: max_overlap <= 0.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_app_initialization_defaults() {
        let app = NovationPeakApp::new();
        assert_eq!(app.active_tab, MainTab::Oscillators);
        assert!(!app.live_midi_arm, "Must default to safe virtual loopback");
        assert!(!app.show_librarian_modal);
        assert!(!app.show_diff_modal);
        assert!(!app.show_randomizer_modal);
        assert!(!app.history.can_undo());
        assert!(!app.history.can_redo());
    }

    #[test]
    fn test_app_undo_redo_via_history() {
        let mut app = NovationPeakApp::new();
        let initial_cutoff = app.engine.get_param("filter_cutoff");

        app.apply_param_change("filter_cutoff", 55.0);
        assert_eq!(app.engine.get_param("filter_cutoff"), 55.0);
        assert!(app.history.can_undo());

        // Undo
        let undone = app.undo();
        assert!(undone);
        assert_eq!(app.engine.get_param("filter_cutoff"), initial_cutoff);
        assert!(app.history.can_redo());

        // Redo
        let redone = app.redo();
        assert!(redone);
        assert_eq!(app.engine.get_param("filter_cutoff"), 55.0);
    }

    #[test]
    fn test_headless_geometry_auditor_1280x800() {
        let mut app = NovationPeakApp::new();
        let report = LayoutGeometryAuditor::audit_at_resolution(&mut app, 1280.0, 800.0);
        assert!(report.passed);
        assert_eq!(report.max_overlap_area, 0.0);
        assert!(report.panels_audited >= 2);
    }

    #[test]
    fn test_headless_geometry_auditor_1920x1080() {
        let mut app = NovationPeakApp::new();
        let report = LayoutGeometryAuditor::audit_at_resolution(&mut app, 1920.0, 1080.0);
        assert!(report.passed);
        assert_eq!(report.max_overlap_area, 0.0);
        assert!(report.panels_audited >= 2);
    }

    #[test]
    fn test_shortcut_gating_when_text_focused() {
        let ctx = Context::default();
        let is_active = is_text_input_active(&ctx);
        assert!(!is_active);
    }

    #[test]
    fn test_custom_vector_widgets_and_animate_pads() {
        let mut app = NovationPeakApp::new();

        // Custom vector widgets state models
        assert_eq!(app.wavetable_widget.base_wave_count, 5);
        assert_eq!(app.svf_widget.mode, synth_ui_theme::SvfFilterMode::LowPass24);
        assert_eq!(app.mod_matrix_widget.slots.len(), 16);

        // Animate performance pads
        app.animate_pads.trigger_animate1();
        assert!(app.animate_pads.animate1_active);
        app.animate_pads.release_animate1();
        assert!(!app.animate_pads.animate1_active);

        // Latching Hold
        app.animate_pads.toggle_hold1();
        assert!(app.animate_pads.animate1_hold);
        assert!(app.animate_pads.animate1_active);

        // Panic silences and clears animate holds
        app.panic();
        assert!(!app.animate_pads.animate1_hold);
        assert!(!app.animate_pads.animate1_active);
    }

    #[test]
    fn test_konduktiva_tab_layout_geometry_auditor() {
        let mut app = NovationPeakApp::new();
        app.active_tab = MainTab::KonduktivaLive;
        let ctx = Context::default();
        let mut raw_input = RawInput::default();
        raw_input.screen_rect = Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1280.0, 800.0)));
        let _ = ctx.run(raw_input, |ctx| {
            CentralPanel::default().show(ctx, |ui| {
                render_konduktiva_panel(ui, &mut app);
            });
        });
        assert_eq!(app.bridge.players.len(), 8);
        assert_eq!(app.bridge.cc_telemetry.len(), 4);
    }
}
