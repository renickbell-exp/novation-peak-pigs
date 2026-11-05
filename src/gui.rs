// SPDX-License-Identifier: GPL-3.0-or-later
//! `novation_peak_manager::gui`: Desktop user interface views, layout panels,
//! modals, and status bar for the Novation Peak synthesizer manager.

use egui::{
    Align, Button, Color32, ComboBox, Layout, ScrollArea, Slider, SliderClamping, TextEdit, Ui,
    Vec2, Window,
};
use synth_core::diff::PatchDiff;
use synth_core::history::ReplacePatchCommand;
use synth_ui_theme::{
    adsr_envelope_plotter_widget, animate_pads_widget, mod_matrix_grid_widget,
    svf_filter_response_widget, wavetable_interpolator_widget, SynthAccent, TwoBowlsBackground,
};

use crate::app::NovationPeakApp;
use crate::peak_librarian::{morph_patches, PeakRandomizeSection};
use crate::peak_synth::WAVETABLE_NAMES;

/// Main navigation tabs for Novation Peak synthesizer control.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MainTab {
    #[default]
    Oscillators,
    Filter,
    EnvelopesLfo,
    ModMatrix,
    Effects,
}

impl MainTab {
    #[must_use]
    pub const fn display_name(&self) -> &'static str {
        match self {
            Self::Oscillators => "Oscillators & Wavetable",
            Self::Filter => "SVF Filter & Drive",
            Self::EnvelopesLfo => "Envelopes & LFOs",
            Self::ModMatrix => "16-Slot Mod Matrix",
            Self::Effects => "FX & Animate Bar",
        }
    }
}

/// Randomizer operation scope in GUI.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RandomizerUiMode {
    #[default]
    Global,
    Section,
    GaussianDrift,
}

/// Returns whether any text edit widget currently retains focus or is requesting keyboard input.
///
/// When active, single-key shortcuts (such as tab switching or performance triggers)
/// must be gated so typing does not trigger unintended global actions.
#[must_use]
pub fn is_text_input_active(ctx: &egui::Context) -> bool {
    ctx.wants_keyboard_input() || ctx.memory(|m| m.focused().is_some())
}

/// Renders the top hardware status and navigation bar.
pub fn render_status_bar(ui: &mut Ui, app: &mut NovationPeakApp) {
    let accent_color = app.theme.resolved_accent_color();

    ui.horizontal(|ui| {
        ui.heading("NOVATION PEAK");
        ui.label(
            egui::RichText::new("8-Voice Polyphonic Hybrid")
                .color(accent_color)
                .strong(),
        );

        ui.separator();

        // Hardware device badges
        let peak_connected = app.status_bar.peak_connected;
        let umc_connected = app.status_bar.umc1820_connected;
        let nano_connected = app.status_bar.nanokontrol2_connected;

        let render_badge = |ui: &mut Ui, name: &str, connected: bool| {
            let col = if connected {
                Color32::from_rgb(40, 180, 80)
            } else {
                Color32::from_rgb(120, 120, 120)
            };
            ui.colored_label(col, format!("[{}]", name));
        };

        render_badge(ui, "Peak hw:3,0,0", peak_connected);
        render_badge(ui, "UMC1820 hw:0,0,0", umc_connected);
        render_badge(ui, "nanoKONTROL2 hw:2,0,0", nano_connected);

        ui.separator();

        // Live MIDI Arm toggle (default safe virtual loopback)
        let arm_text = if app.live_midi_arm {
            egui::RichText::new("LIVE MIDI ARMED [DANGER]")
                .color(Color32::from_rgb(255, 60, 60))
                .strong()
        } else {
            egui::RichText::new("Virtual Loopback [SAFE]")
                .color(Color32::from_rgb(100, 220, 120))
        };
        if ui.button(arm_text).clicked() {
            app.live_midi_arm = !app.live_midi_arm;
        }

        // Panic button
        if ui
            .add(
                Button::new(
                    egui::RichText::new("PANIC")
                        .color(Color32::from_rgb(255, 80, 80))
                        .strong(),
                )
                .fill(Color32::from_rgb(60, 20, 20)),
            )
            .clicked()
        {
            app.panic();
        }

        ui.separator();

        // Global Undo/Redo buttons
        let can_undo = app.history.can_undo();
        let can_redo = app.history.can_redo();

        if ui
            .add_enabled(can_undo, Button::new("Undo (Ctrl+Z)"))
            .clicked()
        {
            app.undo();
        }

        if ui
            .add_enabled(can_redo, Button::new("Redo (Ctrl+Y)"))
            .clicked()
        {
            app.redo();
        }

        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            // Modals toggles
            if ui
                .selectable_label(app.show_randomizer_modal, "Randomizer")
                .clicked()
            {
                app.show_randomizer_modal = !app.show_randomizer_modal;
            }

            if ui.selectable_label(app.show_diff_modal, "A/B Diff").clicked() {
                app.show_diff_modal = !app.show_diff_modal;
            }

            if ui
                .selectable_label(app.show_librarian_modal, "Librarian")
                .clicked()
            {
                app.show_librarian_modal = !app.show_librarian_modal;
            }

            ui.separator();

            // Accent selector
            ComboBox::from_id_salt("accent_selector")
                .selected_text(app.theme.accent.display_name())
                .show_ui(ui, |ui| {
                    for accent in SynthAccent::all() {
                        if ui
                            .selectable_value(
                                &mut app.theme.accent,
                                *accent,
                                accent.display_name(),
                            )
                            .clicked()
                        {
                            app.theme.accent = *accent;
                        }
                    }
                });

            // Background selector
            let current_bg = app
                .theme
                .two_bowls_bg
                .unwrap_or(TwoBowlsBackground::DarkGrey);
            ComboBox::from_id_salt("bg_selector")
                .selected_text(current_bg.display_name())
                .show_ui(ui, |ui| {
                    for bg in TwoBowlsBackground::all() {
                        if ui
                            .selectable_label(current_bg == *bg, bg.display_name())
                            .clicked()
                        {
                            app.theme.two_bowls_bg = Some(*bg);
                        }
                    }
                });
        });
    });
}

/// Renders the main category navigation tab bar.
pub fn render_tabs(ui: &mut Ui, app: &mut NovationPeakApp) {
    ui.horizontal(|ui| {
        let tabs = [
            MainTab::Oscillators,
            MainTab::Filter,
            MainTab::EnvelopesLfo,
            MainTab::ModMatrix,
            MainTab::Effects,
        ];

        for (idx, tab) in tabs.iter().enumerate() {
            let label = format!("{}. {}", idx + 1, tab.display_name());
            let is_selected = app.active_tab == *tab;
            if ui.selectable_label(is_selected, label).clicked() {
                app.active_tab = *tab;
            }
        }
    });
}

/// Renders the Oscillators & Wavetable panel.
pub fn render_oscillators_panel(ui: &mut Ui, app: &mut NovationPeakApp) {
    let accent_color = app.theme.resolved_accent_color();

    ScrollArea::vertical().show(ui, |ui| {
        ui.columns(2, |columns| {
            // Left Column: Vector Wavetable Interpolator Display & Controls
            columns[0].vertical(|ui| {
                ui.group(|ui| {
                    ui.heading("Wavetable Interpolator (5-Wave Display)");
                    let display_size = Vec2::new(ui.available_width().max(200.0), 160.0);
                    wavetable_interpolator_widget(
                        ui,
                        &app.wavetable_widget,
                        display_size,
                        accent_color,
                    );

                    ui.add_space(8.0);
                    let mut pos = app.wavetable_widget.position;
                    if ui
                        .add(
                            Slider::new(&mut pos, 0.0..=1.0)
                                .text("WT Position")
                                .clamping(SliderClamping::Always),
                        )
                        .changed()
                    {
                        app.wavetable_widget.position = pos;
                        let cc_val = (pos * 127.0).round() as f64;
                        app.apply_param_change("osc_1_shape", cc_val);
                    }

                    let mut wt_idx = app.wavetable_widget.wavetable_index;
                    ComboBox::from_id_salt("wt_selector")
                        .selected_text(WAVETABLE_NAMES.get(wt_idx).copied().unwrap_or("Wavetable"))
                        .show_ui(ui, |ui| {
                            for (i, name) in WAVETABLE_NAMES.iter().enumerate() {
                                if ui.selectable_value(&mut wt_idx, i, *name).clicked() {
                                    app.wavetable_widget.wavetable_index = wt_idx;
                                    app.apply_param_change("osc_1_wavetable", wt_idx as f64);
                                }
                            }
                        });
                });

                ui.group(|ui| {
                    ui.heading("Mixer & Balance");
                    let mut osc1_mix = app.engine.get_param("osc_1_mix");
                    let mut osc2_mix = app.engine.get_param("osc_2_mix");
                    let mut osc3_mix = app.engine.get_param("osc_3_mix");
                    let mut ring_mix = app.engine.get_param("ring_mod_mix");
                    let mut noise_mix = app.engine.get_param("noise_mix");

                    if ui
                        .add(Slider::new(&mut osc1_mix, 0.0..=127.0).text("Osc 1 Level"))
                        .changed()
                    {
                        app.apply_param_change("osc_1_mix", osc1_mix);
                    }
                    if ui
                        .add(Slider::new(&mut osc2_mix, 0.0..=127.0).text("Osc 2 Level"))
                        .changed()
                    {
                        app.apply_param_change("osc_2_mix", osc2_mix);
                    }
                    if ui
                        .add(Slider::new(&mut osc3_mix, 0.0..=127.0).text("Osc 3 Level"))
                        .changed()
                    {
                        app.apply_param_change("osc_3_mix", osc3_mix);
                    }
                    if ui
                        .add(Slider::new(&mut ring_mix, 0.0..=127.0).text("Ring Mod Level"))
                        .changed()
                    {
                        app.apply_param_change("ring_mod_mix", ring_mix);
                    }
                    if ui
                        .add(Slider::new(&mut noise_mix, 0.0..=127.0).text("Noise Level"))
                        .changed()
                    {
                        app.apply_param_change("noise_mix", noise_mix);
                    }
                });
            });

            // Right Column: Oscillators 1, 2, and 3 Parameters
            columns[1].vertical(|ui| {
                // Oscillator 1
                ui.group(|ui| {
                    ui.heading("Oscillator 1 (FPGA NCO)");
                    let mut coarse = app.engine.get_param("osc_1_coarse");
                    let mut fine = app.engine.get_param("osc_1_fine");
                    let mut vsync = app.engine.get_param("osc_1_vsync");

                    if ui
                        .add(Slider::new(&mut coarse, 0.0..=127.0).text("Coarse Tune"))
                        .changed()
                    {
                        app.apply_param_change("osc_1_coarse", coarse);
                    }
                    if ui
                        .add(Slider::new(&mut fine, 0.0..=127.0).text("Fine Tune"))
                        .changed()
                    {
                        app.apply_param_change("osc_1_fine", fine);
                    }
                    if ui
                        .add(Slider::new(&mut vsync, 0.0..=127.0).text("Virtual Sync"))
                        .changed()
                    {
                        app.apply_param_change("osc_1_vsync", vsync);
                    }
                });

                // Oscillator 2
                ui.group(|ui| {
                    ui.heading("Oscillator 2 (FPGA NCO)");
                    let mut coarse2 = app.engine.get_param("osc_2_coarse");
                    let mut fine2 = app.engine.get_param("osc_2_fine");
                    let mut shape2 = app.engine.get_param("osc_2_shape");
                    let mut vsync2 = app.engine.get_param("osc_2_vsync");

                    if ui
                        .add(Slider::new(&mut coarse2, 0.0..=127.0).text("Coarse Tune"))
                        .changed()
                    {
                        app.apply_param_change("osc_2_coarse", coarse2);
                    }
                    if ui
                        .add(Slider::new(&mut fine2, 0.0..=127.0).text("Fine Tune"))
                        .changed()
                    {
                        app.apply_param_change("osc_2_fine", fine2);
                    }
                    if ui
                        .add(Slider::new(&mut shape2, 0.0..=127.0).text("Shape / WT Pos"))
                        .changed()
                    {
                        app.apply_param_change("osc_2_shape", shape2);
                    }
                    if ui
                        .add(Slider::new(&mut vsync2, 0.0..=127.0).text("Virtual Sync"))
                        .changed()
                    {
                        app.apply_param_change("osc_2_vsync", vsync2);
                    }
                });

                // Oscillator 3
                ui.group(|ui| {
                    ui.heading("Oscillator 3 (FPGA NCO)");
                    let mut coarse3 = app.engine.get_param("osc_3_coarse");
                    let mut fine3 = app.engine.get_param("osc_3_fine");
                    let mut shape3 = app.engine.get_param("osc_3_shape");
                    let mut vsync3 = app.engine.get_param("osc_3_vsync");

                    if ui
                        .add(Slider::new(&mut coarse3, 0.0..=127.0).text("Coarse Tune"))
                        .changed()
                    {
                        app.apply_param_change("osc_3_coarse", coarse3);
                    }
                    if ui
                        .add(Slider::new(&mut fine3, 0.0..=127.0).text("Fine Tune"))
                        .changed()
                    {
                        app.apply_param_change("osc_3_fine", fine3);
                    }
                    if ui
                        .add(Slider::new(&mut shape3, 0.0..=127.0).text("Shape / WT Pos"))
                        .changed()
                    {
                        app.apply_param_change("osc_3_shape", shape3);
                    }
                    if ui
                        .add(Slider::new(&mut vsync3, 0.0..=127.0).text("Virtual Sync"))
                        .changed()
                    {
                        app.apply_param_change("osc_3_vsync", vsync3);
                    }
                });
            });
        });
    });
}

/// Renders the SVF Filter & Drive response curve panel.
pub fn render_filter_panel(ui: &mut Ui, app: &mut NovationPeakApp) {
    let accent_color = app.theme.resolved_accent_color();

    ScrollArea::vertical().show(ui, |ui| {
        ui.group(|ui| {
            ui.heading("State-Variable Filter Response (SVF Bode Plot)");
            let display_size = Vec2::new(ui.available_width().max(300.0), 200.0);
            svf_filter_response_widget(ui, &app.svf_widget, display_size, accent_color);
        });

        ui.add_space(8.0);

        ui.columns(2, |columns| {
            // Cutoff & Resonance controls
            columns[0].group(|ui| {
                ui.heading("Cutoff & Resonance");
                let mut cutoff = app.engine.get_param("filter_cutoff");
                let mut res = app.engine.get_param("filter_resonance");

                if ui
                    .add(Slider::new(&mut cutoff, 0.0..=127.0).text("Cutoff Frequency"))
                    .changed()
                {
                    app.apply_param_change("filter_cutoff", cutoff);
                    // Approximate Hz calculation: exponential from 20Hz to 20kHz
                    let norm = cutoff as f32 / 127.0;
                    app.svf_widget.cutoff_hz = 20.0 * (1000.0_f32).powf(norm);
                }

                if ui
                    .add(Slider::new(&mut res, 0.0..=127.0).text("Resonance (Safe Cap 96)"))
                    .changed()
                {
                    // Automatic safe cap
                    let clamped_res = res.min(96.0);
                    app.apply_param_change("filter_resonance", clamped_res);
                    app.svf_widget.resonance = clamped_res as f32 / 127.0;
                }

                ui.separator();
                ui.label("Filter Mode:");
                ui.horizontal(|ui| {
                    use synth_ui_theme::SvfFilterMode;
                    if ui
                        .selectable_label(
                            app.svf_widget.mode == SvfFilterMode::LowPass12,
                            "12dB LP",
                        )
                        .clicked()
                    {
                        app.svf_widget.mode = SvfFilterMode::LowPass12;
                    }
                    if ui
                        .selectable_label(
                            app.svf_widget.mode == SvfFilterMode::LowPass24,
                            "24dB LP",
                        )
                        .clicked()
                    {
                        app.svf_widget.mode = SvfFilterMode::LowPass24;
                    }
                    if ui
                        .selectable_label(app.svf_widget.mode == SvfFilterMode::BandPass, "BP")
                        .clicked()
                    {
                        app.svf_widget.mode = SvfFilterMode::BandPass;
                    }
                    if ui
                        .selectable_label(app.svf_widget.mode == SvfFilterMode::HighPass, "HP")
                        .clicked()
                    {
                        app.svf_widget.mode = SvfFilterMode::HighPass;
                    }
                });
            });

            // Analogue Pre-Drive and Post-Drive Saturation controls
            columns[1].group(|ui| {
                ui.heading("Analogue Overdrive & Modulation");
                let mut pre_drive = app.engine.get_param("filter_overdrive");
                let mut post_drive = app.engine.get_param("filter_post_drive");
                let mut tracking = app.engine.get_param("filter_track");
                let mut mod1_depth = app.engine.get_param("mod_1_to_filter");

                if ui
                    .add(Slider::new(&mut pre_drive, 0.0..=127.0).text("Pre-Filter Overdrive"))
                    .changed()
                {
                    let safe_pre = pre_drive.min(80.0);
                    app.apply_param_change("filter_overdrive", safe_pre);
                    app.svf_widget.pre_drive = safe_pre as f32 / 127.0;
                }

                if ui
                    .add(Slider::new(&mut post_drive, 0.0..=127.0).text("Post-Filter Drive"))
                    .changed()
                {
                    let safe_post = post_drive.min(80.0);
                    app.apply_param_change("filter_post_drive", safe_post);
                    app.svf_widget.post_drive = safe_post as f32 / 127.0;
                }

                if ui
                    .add(Slider::new(&mut tracking, 0.0..=127.0).text("Keyboard Tracking"))
                    .changed()
                {
                    app.apply_param_change("filter_track", tracking);
                }

                if ui
                    .add(Slider::new(&mut mod1_depth, 0.0..=127.0).text("Mod Env 1 Depth"))
                    .changed()
                {
                    app.apply_param_change("mod_1_to_filter", mod1_depth);
                }
            });
        });
    });
}

/// Renders the Envelopes & LFOs panel with 3x ADSR vector plotter.
pub fn render_envelopes_lfo_panel(ui: &mut Ui, app: &mut NovationPeakApp) {
    let accent_color = app.theme.resolved_accent_color();
    let readout_color = app.theme.resolved_readout_color();
    let secondary_color = app.theme.accent.secondary_color();

    ScrollArea::vertical().show(ui, |ui| {
        ui.group(|ui| {
            ui.heading("3x ADSR Envelope Plotter (Click to Select Envelope)");
            let display_size = Vec2::new(ui.available_width().max(300.0), 180.0);
            let colors = [accent_color, secondary_color, readout_color];
            adsr_envelope_plotter_widget(ui, &mut app.adsr_widget, display_size, colors);
        });

        ui.add_space(8.0);

        ui.columns(3, |columns| {
            // Amp Envelope
            columns[0].group(|ui| {
                ui.heading("Amp Envelope");
                let mut a = app.engine.get_param("amp_envelope_attack");
                let mut d = app.engine.get_param("amp_envelope_decay");
                let mut s = app.engine.get_param("amp_envelope_sustain");
                let mut r = app.engine.get_param("amp_envelope_release");

                if ui.add(Slider::new(&mut a, 0.0..=127.0).text("Attack")).changed() {
                    app.apply_param_change("amp_envelope_attack", a);
                    app.adsr_widget.amp_env.attack = (a as f32 / 127.0).max(0.01);
                }
                if ui.add(Slider::new(&mut d, 0.0..=127.0).text("Decay")).changed() {
                    app.apply_param_change("amp_envelope_decay", d);
                    app.adsr_widget.amp_env.decay = (d as f32 / 127.0).max(0.01);
                }
                if ui.add(Slider::new(&mut s, 0.0..=127.0).text("Sustain")).changed() {
                    app.apply_param_change("amp_envelope_sustain", s);
                    app.adsr_widget.amp_env.sustain = s as f32 / 127.0;
                }
                if ui.add(Slider::new(&mut r, 0.0..=127.0).text("Release")).changed() {
                    app.apply_param_change("amp_envelope_release", r);
                    app.adsr_widget.amp_env.release = (r as f32 / 127.0).max(0.01);
                }
            });

            // Mod Envelope 1
            columns[1].group(|ui| {
                ui.heading("Mod Envelope 1");
                let mut a1 = app.engine.get_param("mod_envelope_1_attack");
                let mut d1 = app.engine.get_param("mod_envelope_1_decay");
                let mut s1 = app.engine.get_param("mod_envelope_1_sustain");
                let mut r1 = app.engine.get_param("mod_envelope_1_release");

                if ui.add(Slider::new(&mut a1, 0.0..=127.0).text("Attack")).changed() {
                    app.apply_param_change("mod_envelope_1_attack", a1);
                    app.adsr_widget.mod_env_1.attack = (a1 as f32 / 127.0).max(0.01);
                }
                if ui.add(Slider::new(&mut d1, 0.0..=127.0).text("Decay")).changed() {
                    app.apply_param_change("mod_envelope_1_decay", d1);
                    app.adsr_widget.mod_env_1.decay = (d1 as f32 / 127.0).max(0.01);
                }
                if ui.add(Slider::new(&mut s1, 0.0..=127.0).text("Sustain")).changed() {
                    app.apply_param_change("mod_envelope_1_sustain", s1);
                    app.adsr_widget.mod_env_1.sustain = s1 as f32 / 127.0;
                }
                if ui.add(Slider::new(&mut r1, 0.0..=127.0).text("Release")).changed() {
                    app.apply_param_change("mod_envelope_1_release", r1);
                    app.adsr_widget.mod_env_1.release = (r1 as f32 / 127.0).max(0.01);
                }
            });

            // Mod Envelope 2
            columns[2].group(|ui| {
                ui.heading("Mod Envelope 2");
                let mut a2 = app.engine.get_param("mod_envelope_2_attack");
                let mut d2 = app.engine.get_param("mod_envelope_2_decay");
                let mut s2 = 0.0; // Mod env 2 sustain
                let mut r2 = app.engine.get_param("mod_envelope_2_release");

                if ui.add(Slider::new(&mut a2, 0.0..=127.0).text("Attack")).changed() {
                    app.apply_param_change("mod_envelope_2_attack", a2);
                    app.adsr_widget.mod_env_2.attack = (a2 as f32 / 127.0).max(0.01);
                }
                if ui.add(Slider::new(&mut d2, 0.0..=127.0).text("Decay")).changed() {
                    app.apply_param_change("mod_envelope_2_decay", d2);
                    app.adsr_widget.mod_env_2.decay = (d2 as f32 / 127.0).max(0.01);
                }
                if ui.add(Slider::new(&mut s2, 0.0..=127.0).text("Sustain")).changed() {
                    app.adsr_widget.mod_env_2.sustain = s2 as f32 / 127.0;
                }
                if ui.add(Slider::new(&mut r2, 0.0..=127.0).text("Release")).changed() {
                    app.apply_param_change("mod_envelope_2_release", r2);
                    app.adsr_widget.mod_env_2.release = (r2 as f32 / 127.0).max(0.01);
                }
            });
        });

        ui.add_space(8.0);

        // LFO 1 & LFO 2 sections
        ui.columns(2, |columns| {
            columns[0].group(|ui| {
                ui.heading("LFO 1 (Tempo Synced)");
                let mut rate1 = app.engine.get_param("lfo_1_rate");
                let mut sync1 = app.engine.get_param("lfo_1_sync_rate");
                let mut fade1 = app.engine.get_param("lfo_1_fade");

                if ui.add(Slider::new(&mut rate1, 0.0..=127.0).text("Rate")).changed() {
                    app.apply_param_change("lfo_1_rate", rate1);
                }
                if ui.add(Slider::new(&mut sync1, 0.0..=127.0).text("Sync Rate")).changed() {
                    app.apply_param_change("lfo_1_sync_rate", sync1);
                }
                if ui.add(Slider::new(&mut fade1, 0.0..=127.0).text("Fade Time")).changed() {
                    app.apply_param_change("lfo_1_fade", fade1);
                }
            });

            columns[1].group(|ui| {
                ui.heading("LFO 2 (Polyphonic / Global)");
                let mut rate2 = app.engine.get_param("lfo_2_rate");
                let mut sync2 = app.engine.get_param("lfo_2_sync_rate");
                let mut fade2 = app.engine.get_param("lfo_2_fade");

                if ui.add(Slider::new(&mut rate2, 0.0..=127.0).text("Rate")).changed() {
                    app.apply_param_change("lfo_2_rate", rate2);
                }
                if ui.add(Slider::new(&mut sync2, 0.0..=127.0).text("Sync Rate")).changed() {
                    app.apply_param_change("lfo_2_sync_rate", sync2);
                }
                if ui.add(Slider::new(&mut fade2, 0.0..=127.0).text("Fade Time")).changed() {
                    app.apply_param_change("lfo_2_fade", fade2);
                }
            });
        });
    });
}

/// Renders the 16-Slot Modulation Matrix panel.
pub fn render_mod_matrix_panel(ui: &mut Ui, app: &mut NovationPeakApp) {
    let accent_color = app.theme.resolved_accent_color();

    ScrollArea::vertical().show(ui, |ui| {
        ui.heading("16-Slot Modulation Matrix");

        let grid_size = Vec2::new(ui.available_width().max(400.0), 380.0);
        mod_matrix_grid_widget(ui, &mut app.mod_matrix_widget, grid_size, accent_color);

        ui.add_space(8.0);

        // Slot Detail Editor
        if let Some(sel) = app.mod_matrix_widget.selected_slot {
            ui.group(|ui| {
                ui.heading(format!("Edit Matrix Slot #{:02}", sel + 1));
                if let Some(slot) = app.mod_matrix_widget.slot_mut(sel) {
                    ui.horizontal(|ui| {
                        ui.label("Source A:");
                        ui.text_edit_singleline(&mut slot.source_a);

                        ui.label("Source B:");
                        ui.text_edit_singleline(&mut slot.source_b);

                        ui.label("Destination:");
                        ui.text_edit_singleline(&mut slot.destination);

                        ui.label("Depth:");
                        ui.add(Slider::new(&mut slot.depth, -64.0..=63.0));
                    });
                    slot.active = slot.destination != "Off" && slot.depth.abs() > 0.001;
                }
            });
        }
    });
}

/// Renders the FX and Animate performance bar panel.
pub fn render_effects_animate_panel(ui: &mut Ui, app: &mut NovationPeakApp) {
    let accent_color = app.theme.resolved_accent_color();

    ScrollArea::vertical().show(ui, |ui| {
        ui.columns(2, |columns| {
            // Left Column: Analogue Distortion, Chorus, Delay, Reverb
            columns[0].vertical(|ui| {
                ui.group(|ui| {
                    ui.heading("Analogue Distortion");
                    let mut dist = app.engine.get_param("distortion_level");
                    if ui
                        .add(Slider::new(&mut dist, 0.0..=127.0).text("Level (Safe Cap 90)"))
                        .changed()
                    {
                        let safe_dist = dist.min(90.0);
                        app.apply_param_change("distortion_level", safe_dist);
                    }
                });

                ui.group(|ui| {
                    ui.heading("Chorus & Ensemble");
                    let mut chorus = app.engine.get_param("chorus_level");
                    if ui
                        .add(Slider::new(&mut chorus, 0.0..=127.0).text("Chorus Send"))
                        .changed()
                    {
                        app.apply_param_change("chorus_level", chorus);
                    }
                });

                ui.group(|ui| {
                    ui.heading("Stereo Delay");
                    let mut delay_send = app.engine.get_param("delay_send_level");
                    let mut delay_time = app.engine.get_param("delay_time");
                    let mut delay_fb = app.engine.get_param("delay_feedback");

                    if ui
                        .add(Slider::new(&mut delay_send, 0.0..=127.0).text("Send Level"))
                        .changed()
                    {
                        app.apply_param_change("delay_send_level", delay_send);
                    }
                    if ui
                        .add(Slider::new(&mut delay_time, 0.0..=127.0).text("Delay Time"))
                        .changed()
                    {
                        app.apply_param_change("delay_time", delay_time);
                    }
                    if ui
                        .add(Slider::new(&mut delay_fb, 0.0..=127.0).text("Feedback (Safe Cap 85)"))
                        .changed()
                    {
                        let safe_fb = delay_fb.min(85.0);
                        app.apply_param_change("delay_feedback", safe_fb);
                    }
                });

                ui.group(|ui| {
                    ui.heading("Algorithmic Reverb");
                    let mut rev_send = app.engine.get_param("reverb_send_level");
                    let mut rev_time = app.engine.get_param("reverb_decay_time");

                    if ui
                        .add(Slider::new(&mut rev_send, 0.0..=127.0).text("Send Level"))
                        .changed()
                    {
                        app.apply_param_change("reverb_send_level", rev_send);
                    }
                    if ui
                        .add(Slider::new(&mut rev_time, 0.0..=127.0).text("Decay Time"))
                        .changed()
                    {
                        app.apply_param_change("reverb_decay_time", rev_time);
                    }
                });
            });

            // Right Column: Tactile Animate 1 and 2 Performance Pads
            columns[1].vertical(|ui| {
                ui.group(|ui| {
                    ui.heading("Performance Animate Pads (Animate 1 & 2)");
                    ui.label("Tactile performance buttons with momentary trigger and latching HOLD.");
                    ui.add_space(8.0);

                    let pads_size = Vec2::new(ui.available_width().max(240.0), 180.0);
                    animate_pads_widget(ui, &mut app.animate_pads, pads_size, accent_color);
                });
            });
        });
    });
}

/// Renders the 512-patch Librarian modal window.
pub fn render_librarian_modal(ctx: &egui::Context, app: &mut NovationPeakApp) {
    if !app.show_librarian_modal {
        return;
    }

    let mut open = app.show_librarian_modal;
    Window::new("Patch Librarian (512-Patch Manager)")
        .open(&mut open)
        .resizable(true)
        .default_size(Vec2::new(600.0, 500.0))
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label("Search:");
                ui.add(TextEdit::singleline(&mut app.search_query).hint_text("Filter patches..."));
            });

            ui.separator();

            let matches: Vec<(String, u32, String, String, synth_core::SynthPatch)> = app
                .librarian
                .search(&app.search_query)
                .into_iter()
                .map(|p| {
                    (
                        p.meta.bank.clone(),
                        p.meta.slot,
                        p.meta.name.clone(),
                        p.meta.category.clone(),
                        p.clone(),
                    )
                })
                .collect();
            ui.label(format!("Showing {} matches:", matches.len()));

            ScrollArea::vertical().max_height(350.0).show(ui, |ui| {
                for (bank, slot, name, category, patch) in matches {
                    let label = format!("[{}:{}] {} ({})", bank, slot, name, category);

                    ui.horizontal(|ui| {
                        ui.label(label);
                        if ui.button("Load").clicked() {
                            let old_patch = app.engine.patch.clone();
                            let cmd = Box::new(ReplacePatchCommand::new(
                                old_patch,
                                patch,
                                "Load Patch",
                            ));
                            let _ = app.history.push_and_apply(cmd, &mut app.engine.patch);
                            app.sync_widgets_from_engine();
                        }
                    });
                }
            });

            ui.separator();
            ui.horizontal(|ui| {
                if ui.button("Close").clicked() {
                    app.show_librarian_modal = false;
                }
            });
        });

    app.show_librarian_modal = open;
}

/// Renders the A/B Patch Diff & Continuous Morph modal window.
pub fn render_diff_modal(ctx: &egui::Context, app: &mut NovationPeakApp) {
    if !app.show_diff_modal {
        return;
    }

    let mut open = app.show_diff_modal;
    Window::new("A/B Patch Diff & Continuous Morph")
        .open(&mut open)
        .resizable(true)
        .default_size(Vec2::new(550.0, 480.0))
        .show(ctx, |ui| {
            let patch_a = app
                .librarian
                .get_patch(app.diff_slot_a.0, app.diff_slot_a.1)
                .cloned()
                .unwrap_or_else(|| app.engine.patch.clone());
            let patch_b = app
                .librarian
                .get_patch(app.diff_slot_b.0, app.diff_slot_b.1)
                .cloned()
                .unwrap_or_else(|| app.engine.patch.clone());

            ui.horizontal(|ui| {
                ui.label(format!("Patch A: {}", patch_a.meta.name));
                ui.separator();
                ui.label(format!("Patch B: {}", patch_b.meta.name));
            });

            ui.add_space(8.0);
            ui.heading("Continuous Morph Fader (0.0 to 1.0)");
            let mut morph_fader = app.morph_position;
            if ui
                .add(
                    Slider::new(&mut morph_fader, 0.0..=1.0)
                        .text("Morph Position")
                        .clamping(SliderClamping::Always),
                )
                .changed()
            {
                app.morph_position = morph_fader;
                let morphed = morph_patches(&patch_a, &patch_b, morph_fader as f64);
                let old_patch = app.engine.patch.clone();
                let cmd = Box::new(ReplacePatchCommand::new(
                    old_patch,
                    morphed,
                    "Morph Patches",
                ));
                let _ = app.history.push_and_apply(cmd, &mut app.engine.patch);
                app.sync_widgets_from_engine();
            }

            ui.separator();

            let diff = PatchDiff::compare(&patch_a, &patch_b, 0.001);
            ui.heading(format!("Parameter Differences ({})", diff.modified.len()));

            ScrollArea::vertical().max_height(250.0).show(ui, |ui| {
                for item in &diff.modified {
                    ui.label(format!(
                        "{}: {:.1} -> {:.1}",
                        item.key,
                        item.val_a.unwrap_or(0.0),
                        item.val_b.unwrap_or(0.0)
                    ));
                }
            });

            ui.separator();
            if ui.button("Close").clicked() {
                app.show_diff_modal = false;
            }
        });

    app.show_diff_modal = open;
}

/// Renders the Lock-Shielded Safe Randomizer modal window.
pub fn render_randomizer_modal(ctx: &egui::Context, app: &mut NovationPeakApp) {
    if !app.show_randomizer_modal {
        return;
    }

    let mut open = app.show_randomizer_modal;
    Window::new("Lock-Shielded Safe Randomizer")
        .open(&mut open)
        .resizable(true)
        .default_size(Vec2::new(480.0, 420.0))
        .show(ctx, |ui| {
            ui.heading("Randomization Mode");
            ui.horizontal(|ui| {
                ui.selectable_value(&mut app.randomizer_mode, RandomizerUiMode::Global, "Global");
                ui.selectable_value(&mut app.randomizer_mode, RandomizerUiMode::Section, "Section-Scoped");
                ui.selectable_value(&mut app.randomizer_mode, RandomizerUiMode::GaussianDrift, "Gaussian Drift");
            });

            if app.randomizer_mode == RandomizerUiMode::Section {
                ui.separator();
                ui.label("Select Module Section:");
                ui.horizontal(|ui| {
                    ui.selectable_value(&mut app.randomizer_section, PeakRandomizeSection::Osc, "Osc");
                    ui.selectable_value(&mut app.randomizer_section, PeakRandomizeSection::Filter, "Filter");
                    ui.selectable_value(&mut app.randomizer_section, PeakRandomizeSection::Env, "Env");
                    ui.selectable_value(&mut app.randomizer_section, PeakRandomizeSection::Lfo, "LFO");
                    ui.selectable_value(&mut app.randomizer_section, PeakRandomizeSection::Fx, "FX");
                });
            } else if app.randomizer_mode == RandomizerUiMode::GaussianDrift {
                ui.separator();
                ui.add(Slider::new(&mut app.gaussian_delta, 1.0..=20.0).text("Drift Delta"));
            }

            ui.separator();
            ui.heading("Safety Ceilings (Enforced Automatically)");
            ui.label("- Filter Resonance (CC 36): max 96 (prevents harsh runaway)");
            ui.label("- Filter Pre-Drive (CC 79): max 80 (prevents hard clipping)");
            ui.label("- Filter Post-Drive (CC 80): max 80 (protects speakers)");
            ui.label("- Distortion Level (CC 104): max 90");
            ui.label("- Delay Feedback (CC 110): max 85 (prevents infinite loop)");

            ui.add_space(8.0);
            if ui
                .add(
                    Button::new(
                        egui::RichText::new("GENERATE RANDOM PATCH")
                            .color(Color32::WHITE)
                            .strong(),
                    )
                    .fill(Color32::from_rgb(20, 100, 180)),
                )
                .clicked()
            {
                let randomized = match app.randomizer_mode {
                    RandomizerUiMode::Global => {
                        app.randomizer.randomize_global(&app.engine.patch, 100.0, 100.0, None)
                    }
                    RandomizerUiMode::Section => app.randomizer.randomize_section(
                        &app.engine.patch,
                        app.randomizer_section,
                        100.0,
                        100.0,
                        None,
                    ),
                    RandomizerUiMode::GaussianDrift => app
                        .randomizer
                        .randomize_gaussian_drift(&app.engine.patch, app.gaussian_delta, 100.0, None),
                };

                let old_patch = app.engine.patch.clone();
                let cmd = Box::new(ReplacePatchCommand::new(
                    old_patch,
                    randomized,
                    "Randomize Patch",
                ));
                let _ = app.history.push_and_apply(cmd, &mut app.engine.patch);
                app.sync_widgets_from_engine();
            }

            ui.separator();
            if ui.button("Close").clicked() {
                app.show_randomizer_modal = false;
            }
        });

    app.show_randomizer_modal = open;
}