// SPDX-License-Identifier: GPL-3.0-or-later
//! Reusable headless-testable vector widget state models and painter helpers:
//! - [`RotaryKnobModel`]: Rotary dial with target arc and physical soft-takeover ghost ring.
//! - [`StepLedButtonModel`]: Step sequencer button and LED indicator for 303/controller hardware.
//! - [`EnvelopeCurveModel`]: ADSR curve visualization and polygon area generator.
//! - [`KeyOverlayBadgeModel`]: 25-key keyboard layout and AFX overlay badge manager.

use egui::{
    Align2, Color32, FontId, Painter, Pos2, Rect, Response, Rounding, Sense, Shape, Stroke, Ui,
    Vec2,
};
use serde::{Deserialize, Serialize};

/// Default sweep arc angle for rotary knobs in degrees (270 degrees total travel).
pub const DEFAULT_KNOB_SWEEP_DEGREES: f32 = 270.0;

/// Outcome of updating the physical controller position on a soft-takeover control.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SoftTakeoverOutcome {
    /// Physical control has caught the parameter: parameter updated to new value.
    Caught {
        /// The new normalized parameter value after catch.
        new_value: f32,
    },
    /// Physical control has not caught the parameter yet: distance remaining to target.
    PendingCatch {
        /// Absolute distance between physical position and target parameter value.
        distance: f32,
    },
    /// Physical control value was disconnected or cleared.
    Disconnected,
}

/// Headless vector rotary knob model with target arc and physical soft-takeover ghost ring.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RotaryKnobModel {
    /// Unique identifier for this knob.
    pub id: String,
    /// Human-readable parameter label (for example "Cutoff" or "Resonance").
    pub label: String,
    /// Engineering units (for example "Hz", "%", "ms", or "dB").
    pub unit: String,
    /// Current normalized logical parameter value in 0.0..=1.0.
    pub value: f32,
    /// Current physical hardware controller position in 0.0..=1.0, if connected.
    pub physical_value: Option<f32>,
    /// Whether the physical control has caught up with the patch target value.
    pub is_caught: bool,
    /// Pickup tolerance delta for soft-takeover catch (default 0.03).
    pub catch_threshold: f32,
    /// Whether the parameter is bipolar centered at 0.5 (for example pan, pitch, mod depth).
    pub bipolar: bool,
    /// Total travel sweep angle in degrees (default 270.0).
    pub sweep_degrees: f32,
}

impl Default for RotaryKnobModel {
    fn default() -> Self {
        Self {
            id: String::new(),
            label: String::new(),
            unit: String::new(),
            value: 0.5,
            physical_value: None,
            is_caught: true,
            catch_threshold: 0.03,
            bipolar: false,
            sweep_degrees: DEFAULT_KNOB_SWEEP_DEGREES,
        }
    }
}

impl RotaryKnobModel {
    /// Creates a new rotary knob model with default parameters.
    #[must_use]
    pub fn new(id: impl Into<String>, label: impl Into<String>, value: f32) -> Self {
        let clamped = value.clamp(0.0, 1.0);
        Self {
            id: id.into(),
            label: label.into(),
            unit: String::new(),
            value: clamped,
            physical_value: None,
            is_caught: true,
            catch_threshold: 0.03,
            bipolar: false,
            sweep_degrees: DEFAULT_KNOB_SWEEP_DEGREES,
        }
    }

    /// Sets the engineering unit string.
    #[must_use]
    pub fn with_unit(mut self, unit: impl Into<String>) -> Self {
        self.unit = unit.into();
        self
    }

    /// Configures bipolar mode centered at 0.5.
    #[must_use]
    pub fn with_bipolar(mut self, bipolar: bool) -> Self {
        self.bipolar = bipolar;
        self
    }

    /// Sets the initial physical hardware controller position and soft-takeover status.
    #[must_use]
    pub fn with_physical_value(mut self, physical: f32) -> Self {
        let phys_clamped = physical.clamp(0.0, 1.0);
        self.physical_value = Some(phys_clamped);
        let delta = (phys_clamped - self.value).abs();
        self.is_caught = delta <= self.catch_threshold;
        self
    }

    /// Sets the pickup catch tolerance threshold.
    #[must_use]
    pub fn with_catch_threshold(mut self, threshold: f32) -> Self {
        self.catch_threshold = threshold.max(0.001);
        self
    }

    /// Updates the logical patch target value (for example from preset loading or automation).
    pub fn set_target_value(&mut self, new_target: f32) {
        self.value = new_target.clamp(0.0, 1.0);
        if let Some(phys) = self.physical_value {
            let delta = (phys - self.value).abs();
            if delta > self.catch_threshold {
                self.is_caught = false;
            }
        }
    }

    /// Updates the physical hardware controller position and executes soft-takeover arbitration.
    pub fn update_physical_value(&mut self, new_phys: f32) -> SoftTakeoverOutcome {
        let clamped = new_phys.clamp(0.0, 1.0);
        let prev = self.physical_value;
        self.physical_value = Some(clamped);

        if self.is_caught {
            self.value = clamped;
            return SoftTakeoverOutcome::Caught { new_value: clamped };
        }

        let delta = (clamped - self.value).abs();
        let within_threshold = delta <= self.catch_threshold;
        let crossed = match prev {
            Some(p) => (p - self.value) * (clamped - self.value) <= 0.0,
            None => false,
        };

        if within_threshold || crossed {
            self.is_caught = true;
            self.value = clamped;
            SoftTakeoverOutcome::Caught { new_value: clamped }
        } else {
            SoftTakeoverOutcome::PendingCatch { distance: delta }
        }
    }

    /// Computes the pointer angle in radians for any normalized position in 0.0..=1.0.
    #[must_use]
    pub fn angle_for_norm(&self, norm: f32) -> f32 {
        let sweep_rad = self.sweep_degrees.to_radians();
        (norm.clamp(0.0, 1.0) - 0.5) * sweep_rad
    }

    /// Returns the pointer angle in radians for the current logical target value.
    #[must_use]
    pub fn current_angle(&self) -> f32 {
        self.angle_for_norm(self.value)
    }

    /// Returns the pointer angle in radians for the physical hardware controller, if connected.
    #[must_use]
    pub fn physical_angle(&self) -> Option<f32> {
        self.physical_value.map(|phys| self.angle_for_norm(phys))
    }

    /// Computes a point on the dial given center coordinates, radius, and angle in radians.
    #[must_use]
    pub fn dial_point(center: Pos2, radius: f32, angle_rad: f32) -> Pos2 {
        Pos2::new(
            center.x + radius * angle_rad.sin(),
            center.y - radius * angle_rad.cos(),
        )
    }

    /// Returns the active arc start and end normalized positions.
    #[must_use]
    pub fn arc_norm_range(&self) -> (f32, f32) {
        if self.bipolar {
            (0.5, self.value)
        } else {
            (0.0, self.value)
        }
    }

    /// Generates polyline points for the complete background track.
    #[must_use]
    pub fn track_points(&self, center: Pos2, radius: f32, steps: usize) -> Vec<Pos2> {
        let num_steps = steps.max(8);
        (0..=num_steps)
            .map(|i| {
                let norm = i as f32 / num_steps as f32;
                let angle = self.angle_for_norm(norm);
                Self::dial_point(center, radius, angle)
            })
            .collect()
    }

    /// Generates polyline points for the active target parameter value arc.
    #[must_use]
    pub fn arc_points(&self, center: Pos2, radius: f32, steps: usize) -> Vec<Pos2> {
        let (start_norm, end_norm) = self.arc_norm_range();
        let total_steps = steps.max(8);
        let delta = (end_norm - start_norm).abs();
        let arc_steps = ((delta * total_steps as f32).ceil() as usize).max(2);

        (0..=arc_steps)
            .map(|i| {
                let frac = i as f32 / arc_steps as f32;
                let norm = start_norm + frac * (end_norm - start_norm);
                let angle = self.angle_for_norm(norm);
                Self::dial_point(center, radius, angle)
            })
            .collect()
    }

    /// Generates pointer line endpoints for the logical target value.
    #[must_use]
    pub fn pointer_endpoints(
        &self,
        center: Pos2,
        inner_radius: f32,
        outer_radius: f32,
    ) -> (Pos2, Pos2) {
        let angle = self.current_angle();
        (
            Self::dial_point(center, inner_radius, angle),
            Self::dial_point(center, outer_radius, angle),
        )
    }

    /// Generates pointer line endpoints for the physical soft-takeover ghost ring, if pending.
    #[must_use]
    pub fn ghost_pointer_endpoints(
        &self,
        center: Pos2,
        inner_radius: f32,
        outer_radius: f32,
    ) -> Option<(Pos2, Pos2)> {
        if self.is_caught {
            return None;
        }
        self.physical_value.map(|phys| {
            let angle = self.angle_for_norm(phys);
            (
                Self::dial_point(center, inner_radius, angle),
                Self::dial_point(center, outer_radius, angle),
            )
        })
    }

    /// Generates ghost ring arc points showing physical controller travel, if pending.
    #[must_use]
    pub fn ghost_ring_points(&self, center: Pos2, radius: f32, steps: usize) -> Option<Vec<Pos2>> {
        if self.is_caught {
            return None;
        }
        self.physical_value.map(|phys| {
            let total_steps = steps.max(8);
            let arc_steps = ((phys * total_steps as f32).ceil() as usize).max(2);
            (0..=arc_steps)
                .map(|i| {
                    let frac = i as f32 / arc_steps as f32;
                    let norm = frac * phys;
                    let angle = self.angle_for_norm(norm);
                    Self::dial_point(center, radius, angle)
                })
                .collect()
        })
    }

    /// Paints the complete vector rotary knob onto an egui Painter with crisp borders.
    pub fn paint(
        &self,
        painter: &Painter,
        rect: Rect,
        accent_color: Color32,
        ghost_color: Color32,
        bg_fill: Color32,
    ) {
        let size = rect.width().min(rect.height());
        let radius = size * 0.40;
        let center = Pos2::new(rect.center().x, rect.top() + radius + 4.0);

        // Knob body
        let body_radius = radius * 0.70;
        painter.circle_filled(center, body_radius, bg_fill);
        painter.circle_stroke(
            center,
            body_radius,
            Stroke::new(1.0, Color32::from_gray(100)),
        );

        // Background unlit track
        let track_radius = radius * 0.88;
        let track_width = (radius * 0.12).max(1.5);
        let track_pts = self.track_points(center, track_radius, 32);
        if track_pts.len() >= 2 {
            painter.add(Shape::line(
                track_pts,
                Stroke::new(track_width, Color32::from_gray(65)),
            ));
        }

        // Active parameter value arc
        let arc_pts = self.arc_points(center, track_radius, 32);
        if arc_pts.len() >= 2 {
            painter.add(Shape::line(
                arc_pts,
                Stroke::new(track_width, accent_color),
            ));
        }

        // Soft-takeover ghost ring and ghost pointer
        if !self.is_caught {
            let ghost_radius = radius * 0.98;
            if let Some(ghost_pts) = self.ghost_ring_points(center, ghost_radius, 32) {
                if ghost_pts.len() >= 2 {
                    painter.add(Shape::line(
                        ghost_pts,
                        Stroke::new(1.5, ghost_color),
                    ));
                }
            }
            if let Some((gp1, gp2)) =
                self.ghost_pointer_endpoints(center, body_radius * 0.35, body_radius * 0.90)
            {
                painter.line_segment([gp1, gp2], Stroke::new(2.0, ghost_color));
            }
        }

        // Logical parameter value pointer needle
        let (p1, p2) =
            self.pointer_endpoints(center, body_radius * 0.20, body_radius * 0.85);
        painter.line_segment([p1, p2], Stroke::new(2.0, Color32::WHITE));

        // Label and value text
        if !self.label.is_empty() {
            let label_pos = Pos2::new(rect.center().x, rect.bottom() - 14.0);
            painter.text(
                label_pos,
                Align2::CENTER_CENTER,
                &self.label,
                FontId::proportional(11.0),
                Color32::from_gray(210),
            );
        }
    }

    /// Renders the knob as an interactive egui widget with drag adjustments.
    pub fn show(&mut self, ui: &mut Ui, accent_color: Color32) -> Response {
        let desired_size = Vec2::new(64.0, 80.0);
        let (rect, mut response) = ui.allocate_exact_size(desired_size, Sense::click_and_drag());

        if response.dragged() {
            let drag_delta = -response.drag_delta().y * 0.005;
            let new_val = (self.value + drag_delta).clamp(0.0, 1.0);
            if (new_val - self.value).abs() > 0.0001 {
                self.value = new_val;
                response.mark_changed();
            }
        }

        if ui.is_rect_visible(rect) {
            let ghost_color = Color32::from_rgb(255, 176, 0); // Amber warning ghost
            let bg_fill = if response.hovered() {
                Color32::from_gray(45)
            } else {
                Color32::from_gray(30)
            };
            self.paint(ui.painter(), rect, accent_color, ghost_color, bg_fill);
        }

        response
    }
}

/// Illumination state of a step LED indicator.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum StepLedState {
    /// LED is extinguished.
    #[default]
    Off,
    /// LED is steadily illuminated.
    On,
    /// LED is flashing or pulsing.
    Blinking,
    /// LED is illuminated at low brightness for background status.
    Dim,
}

/// Headless vector step LED button model for 303 bassline sequencers and hardware controllers.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StepLedButtonModel {
    /// Zero-based step sequence index (for example 0..15 for a 16-step bar).
    pub step_index: usize,
    /// Human-facing step number (1..16).
    pub step_number: u8,
    /// Musical pitch note name in pitch mode (for example "C", "D#", "G").
    pub note_name: String,
    /// Octave transposition relative to base (-1, 0, +1).
    pub octave: i8,
    /// Gate trigger active for this step.
    pub gate_on: bool,
    /// 303 Accent flag active for this step.
    pub accent: bool,
    /// 303 Slide / portamento flag active for this step.
    pub slide: bool,
    /// Rest flag active for this step.
    pub rest: bool,
    /// Sequencer playhead currently executing this step.
    pub is_current_step: bool,
    /// Step selected for inspection or parameter editing in the GUI.
    pub is_selected: bool,
    /// Current LED illumination state.
    pub led_state: StepLedState,
}

impl StepLedButtonModel {
    /// Creates a new step button model with default initial state.
    #[must_use]
    pub fn new(step_index: usize) -> Self {
        Self {
            step_index,
            step_number: (step_index % 16 + 1) as u8,
            note_name: "C".to_string(),
            octave: 0,
            gate_on: false,
            accent: false,
            slide: false,
            rest: false,
            is_current_step: false,
            is_selected: false,
            led_state: StepLedState::Off,
        }
    }

    /// Sets the pitch note name and octave transpose.
    #[must_use]
    pub fn with_note(mut self, note: impl Into<String>, octave: i8) -> Self {
        self.note_name = note.into();
        self.octave = octave.clamp(-2, 2);
        self
    }

    /// Toggles gate trigger state and updates the LED illumination.
    pub fn toggle_gate(&mut self) -> bool {
        self.gate_on = !self.gate_on;
        self.update_led_state();
        self.gate_on
    }

    /// Toggles accent flag.
    pub fn toggle_accent(&mut self) -> bool {
        self.accent = !self.accent;
        self.accent
    }

    /// Toggles slide / portamento flag.
    pub fn toggle_slide(&mut self) -> bool {
        self.slide = !self.slide;
        self.slide
    }

    /// Toggles rest flag.
    pub fn toggle_rest(&mut self) -> bool {
        self.rest = !self.rest;
        if self.rest {
            self.gate_on = false;
        }
        self.update_led_state();
        self.rest
    }

    /// Sets whether the sequencer playhead is currently positioned on this step.
    pub fn set_current_step(&mut self, is_current: bool) {
        self.is_current_step = is_current;
        self.update_led_state();
    }

    /// Updates internal LED illumination state based on gate and playhead.
    pub fn update_led_state(&mut self) {
        if self.is_current_step {
            self.led_state = StepLedState::Blinking;
        } else if self.gate_on {
            self.led_state = StepLedState::On;
        } else if self.rest {
            self.led_state = StepLedState::Off;
        } else {
            self.led_state = StepLedState::Dim;
        }
    }

    /// Computes effective LED color based on theme, accent, and gate state.
    #[must_use]
    pub fn effective_led_color(
        &self,
        active_color: Color32,
        accent_color: Color32,
        off_color: Color32,
    ) -> Color32 {
        if self.is_current_step {
            Color32::WHITE
        } else if self.accent && self.gate_on {
            accent_color
        } else if self.gate_on {
            active_color
        } else if self.led_state == StepLedState::Dim {
            Color32::from_rgba_premultiplied(
                active_color.r() / 4,
                active_color.g() / 4,
                active_color.b() / 4,
                128,
            )
        } else {
            off_color
        }
    }

    /// Computes layout rectangles for a sequence of step buttons with spacing.
    #[must_use]
    pub fn step_rects(total_rect: Rect, count: usize, spacing: f32) -> Vec<Rect> {
        if count == 0 {
            return Vec::new();
        }
        let total_spacing = spacing * (count - 1) as f32;
        let button_width = ((total_rect.width() - total_spacing) / count as f32).max(1.0);
        let mut rects = Vec::with_capacity(count);

        for i in 0..count {
            let left = total_rect.left() + i as f32 * (button_width + spacing);
            rects.push(Rect::from_min_size(
                Pos2::new(left, total_rect.top()),
                Vec2::new(button_width, total_rect.height()),
            ));
        }

        rects
    }

    /// Paints the step button with crisp zero-rounding borders and LED indicators.
    pub fn paint(
        &self,
        painter: &Painter,
        rect: Rect,
        accent_color: Color32,
        is_hovered: bool,
    ) {
        let button_bg = if self.is_selected {
            Color32::from_gray(55)
        } else if is_hovered {
            Color32::from_gray(40)
        } else {
            Color32::from_gray(25)
        };

        // Crisp zero-rounding button rectangle
        painter.rect_filled(rect, Rounding::ZERO, button_bg);
        let border_stroke = if self.is_selected {
            Stroke::new(1.5, accent_color)
        } else if self.is_current_step {
            Stroke::new(1.5, Color32::WHITE)
        } else {
            Stroke::new(1.0, Color32::from_gray(70))
        };
        painter.rect_stroke(rect, Rounding::ZERO, border_stroke);

        // Top circular LED indicator
        let led_center = Pos2::new(rect.center().x, rect.top() + 8.0);
        let led_radius = 4.0;
        let led_col = self.effective_led_color(
            Color32::from_rgb(255, 48, 48), // Classic 303 red LED
            Color32::from_rgb(255, 180, 0),  // Accent amber
            Color32::from_gray(45),          // Off LED
        );
        painter.circle_filled(led_center, led_radius, led_col);
        painter.circle_stroke(
            led_center,
            led_radius,
            Stroke::new(0.5, Color32::from_gray(90)),
        );

        // Step number label
        let num_str = format!("{}", self.step_number);
        painter.text(
            Pos2::new(rect.center().x, rect.center().y - 2.0),
            Align2::CENTER_CENTER,
            num_str,
            FontId::proportional(11.0),
            if self.gate_on {
                Color32::WHITE
            } else {
                Color32::from_gray(160)
            },
        );

        // Slide and accent badges
        if self.slide {
            painter.text(
                Pos2::new(rect.right() - 6.0, rect.bottom() - 8.0),
                Align2::CENTER_CENTER,
                "~",
                FontId::monospace(10.0),
                accent_color,
            );
        }
        if self.accent {
            painter.text(
                Pos2::new(rect.left() + 6.0, rect.bottom() - 8.0),
                Align2::CENTER_CENTER,
                "A",
                FontId::proportional(9.0),
                Color32::from_rgb(255, 180, 0),
            );
        }
    }

    /// Renders the step button as an interactive egui widget.
    pub fn show(&mut self, ui: &mut Ui, accent_color: Color32) -> Response {
        let desired_size = Vec2::new(32.0, 48.0);
        let (rect, response) = ui.allocate_exact_size(desired_size, Sense::click());

        if response.clicked() {
            self.toggle_gate();
        }

        if ui.is_rect_visible(rect) {
            self.paint(ui.painter(), rect, accent_color, response.hovered());
        }

        response
    }
}

/// Headless vector ADSR envelope curve model for visualization and polygon area fill.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EnvelopeCurveModel {
    /// Normalized attack duration in 0.0..=1.0.
    pub attack: f32,
    /// Normalized decay duration in 0.0..=1.0.
    pub decay: f32,
    /// Normalized sustain amplitude level in 0.0..=1.0.
    pub sustain: f32,
    /// Normalized release duration in 0.0..=1.0.
    pub release: f32,
    /// Curvature coefficient for attack slope (1.0 = linear, >1.0 exponential).
    pub attack_curvature: f32,
    /// Curvature coefficient for decay slope (1.0 = linear, >1.0 exponential).
    pub decay_curvature: f32,
    /// Curvature coefficient for release slope (1.0 = linear, >1.0 exponential).
    pub release_curvature: f32,
    /// Normalized gate hold duration between decay and release (default 0.20).
    pub hold_duration: f32,
}

impl Default for EnvelopeCurveModel {
    fn default() -> Self {
        Self {
            attack: 0.15,
            decay: 0.25,
            sustain: 0.60,
            release: 0.30,
            attack_curvature: 1.0,
            decay_curvature: 1.0,
            release_curvature: 1.0,
            hold_duration: 0.20,
        }
    }
}

impl EnvelopeCurveModel {
    /// Creates a new ADSR envelope model.
    #[must_use]
    pub fn new(attack: f32, decay: f32, sustain: f32, release: f32) -> Self {
        Self {
            attack: attack.clamp(0.0, 1.0),
            decay: decay.clamp(0.0, 1.0),
            sustain: sustain.clamp(0.0, 1.0),
            release: release.clamp(0.0, 1.0),
            attack_curvature: 1.0,
            decay_curvature: 1.0,
            release_curvature: 1.0,
            hold_duration: 0.20,
        }
    }

    /// Configures custom exponential or logarithmic curvatures for each stage.
    #[must_use]
    pub fn with_curvatures(
        mut self,
        attack_curv: f32,
        decay_curv: f32,
        release_curv: f32,
    ) -> Self {
        self.attack_curvature = attack_curv.max(0.1);
        self.decay_curvature = decay_curv.max(0.1);
        self.release_curvature = release_curv.max(0.1);
        self
    }

    /// Sets the gate hold duration.
    #[must_use]
    pub fn with_hold(mut self, hold: f32) -> Self {
        self.hold_duration = hold.clamp(0.05, 0.50);
        self
    }

    /// Computes proportional duration fractions for each stage summing to 1.0.
    #[must_use]
    pub fn stage_fractions(&self) -> (f32, f32, f32, f32) {
        let a = self.attack.max(0.02);
        let d = self.decay.max(0.02);
        let h = self.hold_duration.max(0.02);
        let r = self.release.max(0.02);
        let total = a + d + h + r;
        (a / total, d / total, h / total, r / total)
    }

    /// Evaluates envelope amplitude in 0.0..=1.0 at normalized time in 0.0..=1.0.
    #[must_use]
    pub fn evaluate_at(&self, t_norm: f32) -> f32 {
        let t = t_norm.clamp(0.0, 1.0);
        let (fa, fd, fh, fr) = self.stage_fractions();

        let t_attack_end = fa;
        let t_decay_end = fa + fd;
        let t_hold_end = fa + fd + fh;

        if t <= t_attack_end {
            let stage_t = (t / fa).clamp(0.0, 1.0);
            if (self.attack_curvature - 1.0).abs() < 0.001 {
                stage_t
            } else {
                stage_t.powf(1.0 / self.attack_curvature)
            }
        } else if t <= t_decay_end {
            let stage_t = ((t - t_attack_end) / fd).clamp(0.0, 1.0);
            let drop = if (self.decay_curvature - 1.0).abs() < 0.001 {
                stage_t
            } else {
                stage_t.powf(self.decay_curvature)
            };
            1.0 - (1.0 - self.sustain) * drop
        } else if t <= t_hold_end {
            self.sustain
        } else {
            let stage_t = ((t - t_hold_end) / fr).clamp(0.0, 1.0);
            let drop = if (self.release_curvature - 1.0).abs() < 0.001 {
                stage_t
            } else {
                stage_t.powf(self.release_curvature)
            };
            (self.sustain * (1.0 - drop)).max(0.0)
        }
    }

    /// Computes the five key landmark vertices for the envelope inside a bounding box:
    /// (1) Origin, (2) Attack peak, (3) Sustain start, (4) Sustain end, (5) Release finish.
    #[must_use]
    pub fn key_vertices(&self, rect: Rect) -> [Pos2; 5] {
        let (fa, fd, fh, _fr) = self.stage_fractions();
        let w = rect.width();
        let h = rect.height();

        let p0 = Pos2::new(rect.left(), rect.bottom());
        let p1 = Pos2::new(rect.left() + fa * w, rect.top());
        let p2 = Pos2::new(
            rect.left() + (fa + fd) * w,
            rect.bottom() - self.sustain * h,
        );
        let p3 = Pos2::new(
            rect.left() + (fa + fd + fh) * w,
            rect.bottom() - self.sustain * h,
        );
        let p4 = Pos2::new(rect.right(), rect.bottom());

        [p0, p1, p2, p3, p4]
    }

    /// Generates polyline vertex points across the full envelope curve.
    #[must_use]
    pub fn generate_polyline(&self, rect: Rect, samples: usize) -> Vec<Pos2> {
        let num_samples = samples.max(16);
        let w = rect.width();
        let h = rect.height();

        (0..=num_samples)
            .map(|i| {
                let t = i as f32 / num_samples as f32;
                let amp = self.evaluate_at(t);
                Pos2::new(rect.left() + t * w, rect.bottom() - amp * h)
            })
            .collect()
    }

    /// Generates closed polygon vertices for translucent filled area under the curve.
    #[must_use]
    pub fn generate_filled_polygon(&self, rect: Rect, samples: usize) -> Vec<Pos2> {
        let mut pts = self.generate_polyline(rect, samples);
        pts.push(Pos2::new(rect.right(), rect.bottom()));
        pts.push(Pos2::new(rect.left(), rect.bottom()));
        pts
    }

    /// Paints the complete envelope curve and filled background polygon.
    pub fn paint(
        &self,
        painter: &Painter,
        rect: Rect,
        stroke: Stroke,
        fill_color: Option<Color32>,
        handle_color: Option<Color32>,
    ) {
        // Zero-rounding crisp border frame
        painter.rect_stroke(
            rect,
            Rounding::ZERO,
            Stroke::new(1.0, Color32::from_gray(60)),
        );

        // Filled area under curve
        if let Some(fill) = fill_color {
            let poly_pts = self.generate_filled_polygon(rect, 48);
            if poly_pts.len() >= 3 {
                painter.add(Shape::convex_polygon(
                    poly_pts,
                    fill,
                    Stroke::NONE,
                ));
            }
        }

        // Polyline curve stroke
        let line_pts = self.generate_polyline(rect, 48);
        if line_pts.len() >= 2 {
            painter.add(Shape::line(line_pts, stroke));
        }

        // Circular handles at key landmark vertices
        if let Some(hcol) = handle_color {
            let landmarks = self.key_vertices(rect);
            for pt in landmarks {
                painter.circle_filled(pt, 3.0, hcol);
                painter.circle_stroke(
                    pt,
                    3.0,
                    Stroke::new(0.5, Color32::BLACK),
                );
            }
        }
    }

    /// Renders the envelope curve as an egui visualization widget.
    pub fn show(&self, ui: &mut Ui, stroke_color: Color32) -> Response {
        let desired_size = Vec2::new(160.0, 80.0);
        let (rect, response) = ui.allocate_exact_size(desired_size, Sense::hover());

        if ui.is_rect_visible(rect) {
            let fill_color = Color32::from_rgba_premultiplied(
                stroke_color.r() / 4,
                stroke_color.g() / 4,
                stroke_color.b() / 4,
                80,
            );
            self.paint(
                ui.painter(),
                rect,
                Stroke::new(2.0, stroke_color),
                Some(fill_color),
                Some(Color32::WHITE),
            );
        }

        response
    }
}

/// Headless vector keyboard model for the 25 keys on the Novation Bass Station II,
/// supporting AFX Mode per-key patch overlays and secondary faceplate function labels.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KeyOverlayBadgeModel {
    /// Zero-based key index (0..24 for 25 keys).
    pub key_index: u8,
    /// MIDI note number (36..60 for C2 to C4).
    pub midi_note: u8,
    /// Note name (for example "C", "C#", "D").
    pub note_name: String,
    /// Whether this is a black accidental piano key.
    pub is_black_key: bool,
    /// Optional AFX Mode patch slot ID assigned to this key.
    pub afx_patch_id: Option<u16>,
    /// Optional AFX Mode patch name assigned to this key.
    pub afx_patch_name: Option<String>,
    /// Primary secondary faceplate function label on Bass Station II.
    pub function_label: Option<String>,
    /// Sub-parameter secondary label on Bass Station II.
    pub sub_function_label: Option<String>,
    /// Whether this key is currently held down or triggered.
    pub is_pressed: bool,
    /// Category color override in hex for AFX Mode badges (for example "#00D2FF").
    pub badge_color_hex: Option<String>,
}

impl KeyOverlayBadgeModel {
    /// Creates a key overlay badge model for a given key index in 0..24.
    #[must_use]
    pub fn new(key_index: u8) -> Self {
        let idx = key_index.min(24);
        let is_black = Self::is_black_key_index(idx);
        let note = Self::note_name_for_index(idx);
        let (func, sub) = Self::bs2_function_for_key(idx);

        Self {
            key_index: idx,
            midi_note: 36 + idx,
            note_name: note.to_string(),
            is_black_key: is_black,
            afx_patch_id: None,
            afx_patch_name: None,
            function_label: Some(func.to_string()),
            sub_function_label: Some(sub.to_string()),
            is_pressed: false,
            badge_color_hex: None,
        }
    }

    /// Determines whether a 0..24 key index represents a black accidental key.
    #[must_use]
    pub const fn is_black_key_index(key_index: u8) -> bool {
        let note_in_octave = (key_index % 12) as usize;
        matches!(note_in_octave, 1 | 3 | 6 | 8 | 10)
    }

    /// Returns the note name string for a 0..24 key index.
    #[must_use]
    pub const fn note_name_for_index(key_index: u8) -> &'static str {
        const NOTE_NAMES: [&str; 12] = [
            "C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B",
        ];
        NOTE_NAMES[(key_index % 12) as usize]
    }

    /// Returns Novation Bass Station II printed faceplate functions for each key.
    #[must_use]
    pub const fn bs2_function_for_key(key_index: u8) -> (&'static str, &'static str) {
        match key_index {
            0 => ("OSC 1", "WAVE / OCT"),
            1 => ("OSC 1", "RANGE"),
            2 => ("OSC 2", "WAVE"),
            3 => ("OSC 2", "RANGE"),
            4 => ("SUB OSC", "WAVE"),
            5 => ("SUB OSC", "OCTAVE"),
            6 => ("FILTER", "TYPE"),
            7 => ("FILTER", "SLOPE"),
            8 => ("FILTER", "SHAPE"),
            9 => ("LFO 1", "WAVE"),
            10 => ("LFO 1", "SPEED"),
            11 => ("LFO 2", "WAVE"),
            12 => ("LFO 2", "SPEED"),
            13 => ("MOD ENV", "ATTACK"),
            14 => ("MOD ENV", "DECAY"),
            15 => ("MOD ENV", "SUSTAIN"),
            16 => ("MOD ENV", "RELEASE"),
            17 => ("AMP ENV", "ATTACK"),
            18 => ("AMP ENV", "DECAY"),
            19 => ("AMP ENV", "SUSTAIN"),
            20 => ("AMP ENV", "RELEASE"),
            21 => ("ARP", "MODE"),
            22 => ("ARP", "RHYTHM"),
            23 => ("ARP", "OCTAVE"),
            24 => ("AFX / PARA", "OVERLAY"),
            _ => ("UNKNOWN", ""),
        }
    }

    /// Creates the complete canonical set of 25 keys for the Bass Station II.
    #[must_use]
    pub fn bs2_25_keys() -> Vec<Self> {
        (0..25).map(Self::new).collect()
    }

    /// Assigns an AFX Mode patch overlay to this key.
    #[must_use]
    pub fn with_afx_patch(
        mut self,
        patch_id: u16,
        name: impl Into<String>,
        color_hex: impl Into<String>,
    ) -> Self {
        self.afx_patch_id = Some(patch_id);
        self.afx_patch_name = Some(name.into());
        self.badge_color_hex = Some(color_hex.into());
        self
    }

    /// Clears any assigned AFX Mode patch overlay.
    pub fn clear_afx_patch(&mut self) {
        self.afx_patch_id = None;
        self.afx_patch_name = None;
        self.badge_color_hex = None;
    }

    /// Headless calculation of piano key rectangles for all 25 keys:
    /// Returns a tuple of (15 white key rectangles, 10 black key rectangles).
    #[must_use]
    pub fn calculate_25_key_layout(total_rect: Rect) -> (Vec<Rect>, Vec<Rect>) {
        const WHITE_KEY_COUNT: usize = 15;
        let white_width = total_rect.width() / WHITE_KEY_COUNT as f32;
        let black_width = white_width * 0.60;
        let black_height = total_rect.height() * 0.62;

        let mut white_rects = Vec::with_capacity(WHITE_KEY_COUNT);
        for i in 0..WHITE_KEY_COUNT {
            let left = total_rect.left() + i as f32 * white_width;
            white_rects.push(Rect::from_min_size(
                Pos2::new(left, total_rect.top()),
                Vec2::new(white_width, total_rect.height()),
            ));
        }

        // Octave 1 black keys: C# (boundary 1), D# (boundary 2), F# (boundary 4), G# (boundary 5), A# (boundary 6)
        // Octave 2 black keys: C# (boundary 8), D# (boundary 9), F# (boundary 11), G# (boundary 12), A# (boundary 13)
        let black_boundaries = [1, 2, 4, 5, 6, 8, 9, 11, 12, 13];
        let mut black_rects = Vec::with_capacity(black_boundaries.len());
        for b in black_boundaries {
            let center_x = total_rect.left() + b as f32 * white_width;
            black_rects.push(Rect::from_min_size(
                Pos2::new(center_x - black_width * 0.5, total_rect.top()),
                Vec2::new(black_width, black_height),
            ));
        }

        (white_rects, black_rects)
    }

    /// Paints the key rectangle and top badge with crisp zero-rounding borders.
    pub fn paint_key_and_badge(
        &self,
        painter: &Painter,
        key_rect: Rect,
        badge_rect: Rect,
        accent_color: Color32,
    ) {
        // 1. Piano key surface
        let fill = if self.is_pressed {
            accent_color
        } else if self.is_black_key {
            Color32::from_gray(25)
        } else {
            Color32::from_gray(240)
        };

        painter.rect_filled(key_rect, Rounding::ZERO, fill);
        painter.rect_stroke(
            key_rect,
            Rounding::ZERO,
            Stroke::new(1.0, Color32::from_gray(80)),
        );

        // 2. AFX / function badge above the key
        let badge_fill = if let Some(ref hex) = self.badge_color_hex {
            crate::parse_hex_color(hex).unwrap_or(Color32::from_gray(50))
        } else if self.is_pressed {
            accent_color
        } else {
            Color32::from_gray(35)
        };

        painter.rect_filled(badge_rect, Rounding::ZERO, badge_fill);
        painter.rect_stroke(
            badge_rect,
            Rounding::ZERO,
            Stroke::new(1.0, Color32::from_gray(70)),
        );

        // Badge text: display AFX patch name if assigned, else BS2 faceplate label
        let display_text = if let Some(ref afx_name) = self.afx_patch_name {
            afx_name.as_str()
        } else if let Some(ref func) = self.function_label {
            func.as_str()
        } else {
            self.note_name.as_str()
        };

        painter.text(
            badge_rect.center(),
            Align2::CENTER_CENTER,
            display_text,
            FontId::proportional(9.0),
            Color32::WHITE,
        );
    }
}

// =============================================================================
// Percussion Vector Widgets (Nord Drum 2 & Konduktiva)
// =============================================================================

/// Headless vector drum pad model with dynamic velocity strike response, peak hold,
/// choke grouping, and Nord Drum 2 voice channel assignment.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DrumPadVelocityModel {
    /// Zero-based channel index (0..5 for Nord Drum 2 Voices 1..6).
    pub channel_index: usize,
    /// Channel display name (for example "Kick", "Snare", "Tom Lo", "Voice 1").
    pub label: String,
    /// MIDI trigger note number (default 60 + channel_index).
    pub midi_note: u8,
    /// Current velocity (0..127).
    pub velocity: u8,
    /// Peak hold velocity (0..127).
    pub peak_velocity: u8,
    /// Whether the pad is currently struck or active.
    pub is_triggered: bool,
    /// Whether this channel is currently focused for parameter editing (CC 70).
    pub is_focused: bool,
    /// Whether this channel is muted.
    pub is_muted: bool,
    /// Optional choke group assignment (for example 1 for open/closed hi-hat).
    pub choke_group: Option<u8>,
}

impl DrumPadVelocityModel {
    /// Creates a new drum pad velocity model for a voice channel index.
    #[must_use]
    pub fn new(channel_index: usize, label: impl Into<String>) -> Self {
        let ch = channel_index.min(5);
        Self {
            channel_index: ch,
            label: label.into(),
            midi_note: 60 + (ch as u8),
            velocity: 0,
            peak_velocity: 0,
            is_triggered: false,
            is_focused: false,
            is_muted: false,
            choke_group: None,
        }
    }

    /// Sets the MIDI trigger note number.
    #[must_use]
    pub fn with_midi_note(mut self, note: u8) -> Self {
        self.midi_note = note;
        self
    }

    /// Sets the current velocity.
    #[must_use]
    pub fn with_velocity(mut self, velocity: u8) -> Self {
        let v = velocity.min(127);
        self.velocity = v;
        self.peak_velocity = self.peak_velocity.max(v);
        self
    }

    /// Sets the choke group.
    #[must_use]
    pub fn with_choke_group(mut self, choke: u8) -> Self {
        self.choke_group = Some(choke);
        self
    }

    /// Sets editor focus state.
    #[must_use]
    pub fn with_focused(mut self, focused: bool) -> Self {
        self.is_focused = focused;
        self
    }

    /// Sets mute state.
    #[must_use]
    pub fn with_muted(mut self, muted: bool) -> Self {
        self.is_muted = muted;
        self
    }

    /// Triggers the pad with a strike velocity in 0..127.
    pub fn trigger(&mut self, velocity: u8) {
        let v = velocity.min(127);
        self.velocity = v;
        self.peak_velocity = self.peak_velocity.max(v);
        self.is_triggered = true;
    }

    /// Releases the active trigger state.
    pub fn release(&mut self) {
        self.is_triggered = false;
    }

    /// Decays the peak hold velocity by a step decrement.
    pub fn decay_peak(&mut self, step: u8) {
        self.peak_velocity = self.peak_velocity.saturating_sub(step);
    }

    /// Returns normalized velocity in 0.0..=1.0.
    #[must_use]
    pub fn normalized_velocity(&self) -> f32 {
        self.velocity as f32 / 127.0
    }

    /// Returns normalized peak hold velocity in 0.0..=1.0.
    #[must_use]
    pub fn normalized_peak(&self) -> f32 {
        self.peak_velocity as f32 / 127.0
    }

    /// Returns the 1-based voice number (1..6).
    #[must_use]
    pub fn voice_number(&self) -> usize {
        self.channel_index + 1
    }

    /// Computes layout rectangles for a grid of pads (for example 6 pads in 2 rows x 3 columns).
    #[must_use]
    pub fn compute_pad_rects(total_rect: Rect, count: usize, cols: usize, spacing: f32) -> Vec<Rect> {
        if count == 0 || cols == 0 {
            return Vec::new();
        }
        let rows = count.div_ceil(cols);
        let pad_width = ((total_rect.width() - spacing * (cols - 1) as f32) / cols as f32).max(1.0);
        let pad_height = ((total_rect.height() - spacing * (rows - 1) as f32) / rows as f32).max(1.0);

        let mut rects = Vec::with_capacity(count);
        for idx in 0..count {
            let col = idx % cols;
            let row = idx / cols;
            let min_x = total_rect.left() + col as f32 * (pad_width + spacing);
            let min_y = total_rect.top() + row as f32 * (pad_height + spacing);
            rects.push(Rect::from_min_size(
                Pos2::new(min_x, min_y),
                Vec2::new(pad_width, pad_height),
            ));
        }
        rects
    }

    /// Computes the bounding rect for the interior horizontal velocity meter.
    #[must_use]
    pub fn velocity_bar_rect(&self, pad_rect: Rect) -> Rect {
        let pad_h = pad_rect.height();
        let bar_h = (pad_h * 0.12).clamp(4.0, 10.0);
        let margin_x = 4.0;
        let margin_b = 4.0;
        Rect::from_min_size(
            Pos2::new(pad_rect.left() + margin_x, pad_rect.bottom() - bar_h - margin_b),
            Vec2::new((pad_rect.width() - margin_x * 2.0).max(1.0), bar_h),
        )
    }

    /// Computes the active filled portion of the velocity meter.
    #[must_use]
    pub fn velocity_fill_rect(&self, pad_rect: Rect) -> Rect {
        let bar = self.velocity_bar_rect(pad_rect);
        let fill_w = bar.width() * self.normalized_velocity();
        Rect::from_min_size(bar.min, Vec2::new(fill_w, bar.height()))
    }

    /// Paints the drum pad velocity widget.
    pub fn paint(
        &self,
        painter: &Painter,
        rect: Rect,
        accent_color: Color32,
        readout_color: Color32,
    ) {
        // Pad strike surface fill
        let bg_fill = if self.is_muted {
            Color32::from_gray(18)
        } else if self.is_triggered {
            Color32::from_rgba_premultiplied(
                (accent_color.r() as u16 * 140 / 255) as u8,
                (accent_color.g() as u16 * 140 / 255) as u8,
                (accent_color.b() as u16 * 140 / 255) as u8,
                220,
            )
        } else {
            Color32::from_gray(30)
        };

        let stroke_color = if self.is_focused {
            accent_color
        } else if self.is_triggered {
            Color32::WHITE
        } else {
            Color32::from_gray(65)
        };
        let stroke_width = if self.is_focused { 2.0 } else { 1.0 };

        painter.rect_filled(rect, Rounding::ZERO, bg_fill);
        painter.rect_stroke(rect, Rounding::ZERO, Stroke::new(stroke_width, stroke_color));

        // Header: Channel / voice number and optional choke badge
        let header_text = format!("CH {}", self.voice_number());
        painter.text(
            Pos2::new(rect.left() + 6.0, rect.top() + 6.0),
            Align2::LEFT_TOP,
            &header_text,
            FontId::proportional(11.0),
            if self.is_focused { readout_color } else { Color32::from_gray(180) },
        );

        if let Some(choke) = self.choke_group {
            let choke_text = format!("EX{}", choke);
            painter.text(
                Pos2::new(rect.right() - 6.0, rect.top() + 6.0),
                Align2::RIGHT_TOP,
                &choke_text,
                FontId::proportional(9.0),
                Color32::from_rgb(255, 120, 60),
            );
        }

        // Center: Instrument label (e.g. "KICK")
        let label_color = if self.is_muted {
            Color32::from_gray(80)
        } else if self.is_triggered {
            Color32::WHITE
        } else {
            Color32::from_gray(230)
        };
        painter.text(
            Pos2::new(rect.center().x, rect.center().y - 4.0),
            Align2::CENTER_CENTER,
            &self.label,
            FontId::proportional(14.0),
            label_color,
        );

        // Subtitle: MIDI Note number
        let note_text = format!("#{}", self.midi_note);
        painter.text(
            Pos2::new(rect.left() + 6.0, rect.center().y + 12.0),
            Align2::LEFT_TOP,
            &note_text,
            FontId::proportional(9.0),
            Color32::from_gray(140),
        );

        // Velocity readout in high-contrast amber LED readout color
        let vel_text = if self.velocity > 0 {
            format!("V:{:3}", self.velocity)
        } else {
            "IDLE".to_string()
        };
        painter.text(
            Pos2::new(rect.right() - 6.0, rect.center().y + 12.0),
            Align2::RIGHT_TOP,
            &vel_text,
            FontId::monospace(9.0),
            readout_color,
        );

        // Velocity Meter Bar at bottom
        let bar_slot = self.velocity_bar_rect(rect);
        painter.rect_filled(bar_slot, Rounding::ZERO, Color32::from_gray(18));
        painter.rect_stroke(bar_slot, Rounding::ZERO, Stroke::new(0.5, Color32::from_gray(50)));

        if self.velocity > 0 {
            let fill_rect = self.velocity_fill_rect(rect);
            painter.rect_filled(fill_rect, Rounding::ZERO, accent_color);
        }

        // Peak hold tick marker
        if self.peak_velocity > 0 {
            let peak_x = bar_slot.left() + bar_slot.width() * self.normalized_peak();
            painter.line_segment(
                [
                    Pos2::new(peak_x, bar_slot.top()),
                    Pos2::new(peak_x, bar_slot.bottom()),
                ],
                Stroke::new(1.5, readout_color),
            );
        }
    }

    /// Renders the interactive drum pad velocity widget.
    pub fn show(
        &mut self,
        ui: &mut Ui,
        accent_color: Color32,
        readout_color: Color32,
    ) -> Response {
        let desired_size = Vec2::new(96.0, 72.0);
        let (rect, response) = ui.allocate_exact_size(desired_size, Sense::click_and_drag());

        if response.clicked() {
            self.trigger(100);
        }

        if ui.is_rect_visible(rect) {
            self.paint(ui.painter(), rect, accent_color, readout_color);
        }

        response
    }
}

/// Standalone percussion vector widget for rendering a drum pad velocity strike indicator.
pub fn drum_pad_velocity_widget(
    ui: &mut Ui,
    model: &mut DrumPadVelocityModel,
    accent_color: Color32,
    readout_color: Color32,
) -> Response {
    model.show(ui, accent_color, readout_color)
}

/// Headless harmonic spectrum resonator model for Nord Drum 2 Tone synthesis models,
/// representing harmonic partial amplitudes, resonance Q, and spectral decay tilt.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResonantSpectrumBarModel {
    /// Number of harmonic partial bars displayed (typically 8 to 16).
    pub partial_count: usize,
    /// Base fundamental resonant frequency in Hz.
    pub fundamental_hz: f32,
    /// Normalized amplitude weights for each harmonic partial in 0.0..=1.0.
    pub harmonic_weights: Vec<f32>,
    /// Resonance Q / sharpness factor in 0.0..=1.0.
    pub resonance_q: f32,
    /// Bipolar spectral tilt in -1.0..=1.0 (negative = dark/low-pass, positive = bright/high-pass).
    pub spectral_tilt: f32,
    /// High-frequency decay damping coefficient in 0.0..=1.0.
    pub decay_damping: f32,
}

impl ResonantSpectrumBarModel {
    /// Creates a new resonant spectrum bar model with a specified number of partial bars.
    #[must_use]
    pub fn new(partial_count: usize) -> Self {
        let count = partial_count.clamp(1, 32);
        let mut model = Self {
            partial_count: count,
            fundamental_hz: 220.0,
            harmonic_weights: vec![0.0; count],
            resonance_q: 0.70,
            spectral_tilt: 0.0,
            decay_damping: 0.30,
        };
        model.recompute_harmonics();
        model
    }

    /// Creates a resonant spectrum model configured for standard Nord Drum 2 acoustic presets.
    #[must_use]
    pub fn from_preset(preset_name: &str) -> Self {
        let count = 8;
        let mut model = Self::new(count);
        match preset_name.trim().to_lowercase().as_str() {
            "bell" | "chime" => {
                model.fundamental_hz = 520.0;
                model.resonance_q = 0.90;
                model.spectral_tilt = 0.35;
                model.decay_damping = 0.15;
                model.harmonic_weights = vec![1.0, 0.72, 0.45, 0.85, 0.30, 0.60, 0.20, 0.40];
            }
            "membrane" | "drumhead" => {
                model.fundamental_hz = 140.0;
                model.resonance_q = 0.60;
                model.spectral_tilt = -0.40;
                model.decay_damping = 0.50;
                model.harmonic_weights = vec![1.0, 0.62, 0.38, 0.24, 0.15, 0.09, 0.05, 0.02];
            }
            "cymbal" | "metal" => {
                model.fundamental_hz = 880.0;
                model.resonance_q = 0.85;
                model.spectral_tilt = 0.50;
                model.decay_damping = 0.10;
                model.harmonic_weights = vec![0.40, 0.65, 0.90, 0.75, 1.0, 0.85, 0.70, 0.55];
            }
            "sub" | "kick" => {
                model.fundamental_hz = 55.0;
                model.resonance_q = 0.40;
                model.spectral_tilt = -0.80;
                model.decay_damping = 0.75;
                model.harmonic_weights = vec![1.0, 0.30, 0.08, 0.02, 0.0, 0.0, 0.0, 0.0];
            }
            _ => {
                model.recompute_harmonics();
            }
        }
        model
    }

    /// Sets explicit harmonic partial weights.
    #[must_use]
    pub fn with_harmonic_weights(mut self, weights: Vec<f32>) -> Self {
        self.partial_count = weights.len().clamp(1, 32);
        self.harmonic_weights = weights.into_iter().map(|w| w.clamp(0.0, 1.0)).collect();
        self
    }

    /// Sets the fundamental resonance frequency in Hz.
    #[must_use]
    pub fn with_fundamental(mut self, hz: f32) -> Self {
        self.fundamental_hz = hz.max(10.0);
        self
    }

    /// Sets the resonance Q factor in 0.0..=1.0.
    #[must_use]
    pub fn with_resonance_q(mut self, q: f32) -> Self {
        self.resonance_q = q.clamp(0.0, 1.0);
        self
    }

    /// Sets the spectral tilt in -1.0..=1.0.
    #[must_use]
    pub fn with_spectral_tilt(mut self, tilt: f32) -> Self {
        self.spectral_tilt = tilt.clamp(-1.0, 1.0);
        self
    }

    /// Sets the high-frequency decay damping in 0.0..=1.0.
    #[must_use]
    pub fn with_decay_damping(mut self, damping: f32) -> Self {
        self.decay_damping = damping.clamp(0.0, 1.0);
        self
    }

    /// Sets individual partial weight by zero-based index.
    pub fn set_partial_weight(&mut self, index: usize, weight: f32) {
        if index < self.harmonic_weights.len() {
            self.harmonic_weights[index] = weight.clamp(0.0, 1.0);
        }
    }

    /// Recomputes harmonic weights from fundamental, resonance Q, tilt, and damping.
    pub fn recompute_harmonics(&mut self) {
        self.harmonic_weights.resize(self.partial_count, 0.0);
        for i in 0..self.partial_count {
            let harmonic_idx = (i + 1) as f32;
            let natural = 1.0 / harmonic_idx.sqrt();
            let tilt_factor = 1.0 + self.spectral_tilt * ((i as f32) / self.partial_count as f32 - 0.5) * 2.0;
            let damp = (1.0 - self.decay_damping * 0.15).powf(harmonic_idx);
            let q_boost = if i == 0 {
                1.0 + self.resonance_q * 0.5
            } else if i == 1 {
                1.0 + self.resonance_q * 0.25
            } else {
                1.0
            };

            let weight = (natural * tilt_factor * damp * q_boost).clamp(0.02, 1.0);
            self.harmonic_weights[i] = weight;
        }
    }

    /// Computes layout bounding boxes for all partial bar slots across total_rect.
    #[must_use]
    pub fn bar_rects(&self, total_rect: Rect, spacing: f32) -> Vec<Rect> {
        let count = self.partial_count;
        if count == 0 {
            return Vec::new();
        }
        let bar_width = ((total_rect.width() - spacing * (count - 1) as f32) / count as f32).max(1.0);
        let mut rects = Vec::with_capacity(count);
        for i in 0..count {
            let left = total_rect.left() + i as f32 * (bar_width + spacing);
            rects.push(Rect::from_min_size(
                Pos2::new(left, total_rect.top()),
                Vec2::new(bar_width, total_rect.height()),
            ));
        }
        rects
    }

    /// Computes the active filled portion of each partial bar based on its weight.
    #[must_use]
    pub fn filled_bar_rects(&self, total_rect: Rect, spacing: f32) -> Vec<Rect> {
        let slots = self.bar_rects(total_rect, spacing);
        slots
            .into_iter()
            .enumerate()
            .map(|(i, slot)| {
                let weight = self.harmonic_weights.get(i).copied().unwrap_or(0.0).clamp(0.0, 1.0);
                let fill_height = (slot.height() * weight).max(1.0);
                Rect::from_min_size(
                    Pos2::new(slot.left(), slot.bottom() - fill_height),
                    Vec2::new(slot.width(), fill_height),
                )
            })
            .collect()
    }

    /// Generates polyline vertices connecting the top centers of the harmonic bars.
    #[must_use]
    pub fn envelope_polyline(&self, total_rect: Rect, spacing: f32) -> Vec<Pos2> {
        let filled = self.filled_bar_rects(total_rect, spacing);
        filled
            .into_iter()
            .map(|bar| Pos2::new(bar.center().x, bar.top()))
            .collect()
    }

    /// Paints the resonant spectrum bar widget.
    pub fn paint(
        &self,
        painter: &Painter,
        rect: Rect,
        accent_color: Color32,
        readout_color: Color32,
    ) {
        // Frame background
        painter.rect_filled(rect, Rounding::ZERO, Color32::from_gray(20));
        painter.rect_stroke(rect, Rounding::ZERO, Stroke::new(1.0, Color32::from_gray(50)));

        // Reference amplitude horizontal grid lines (-6dB, -12dB)
        let mid_y = rect.top() + rect.height() * 0.5;
        let qtr_y = rect.top() + rect.height() * 0.25;
        painter.line_segment(
            [Pos2::new(rect.left(), mid_y), Pos2::new(rect.right(), mid_y)],
            Stroke::new(0.5, Color32::from_gray(40)),
        );
        painter.line_segment(
            [Pos2::new(rect.left(), qtr_y), Pos2::new(rect.right(), qtr_y)],
            Stroke::new(0.5, Color32::from_gray(35)),
        );

        let spacing = 3.0;
        let slots = self.bar_rects(rect, spacing);
        let filled = self.filled_bar_rects(rect, spacing);

        for (i, slot) in slots.iter().enumerate() {
            // Unfilled slot track
            painter.rect_filled(*slot, Rounding::ZERO, Color32::from_gray(30));

            // Filled partial amplitude
            if let Some(fill) = filled.get(i) {
                let bar_color = if i == 0 {
                    readout_color
                } else {
                    accent_color
                };
                painter.rect_filled(*fill, Rounding::ZERO, bar_color);
            }
        }

        // Connecting spectral envelope line
        let poly = self.envelope_polyline(rect, spacing);
        if poly.len() >= 2 {
            for win in poly.windows(2) {
                painter.line_segment([win[0], win[1]], Stroke::new(1.5, readout_color));
            }
        }

        // Header info: Fundamental and Q
        let info = format!("{:.0} Hz | Q {:.2}", self.fundamental_hz, self.resonance_q);
        painter.text(
            Pos2::new(rect.left() + 4.0, rect.top() + 4.0),
            Align2::LEFT_TOP,
            &info,
            FontId::monospace(9.0),
            readout_color,
        );
    }

    /// Renders the interactive resonant spectrum bar widget.
    pub fn show(
        &mut self,
        ui: &mut Ui,
        accent_color: Color32,
        readout_color: Color32,
    ) -> Response {
        let desired_size = Vec2::new(160.0, 72.0);
        let (rect, response) = ui.allocate_exact_size(desired_size, Sense::click_and_drag());

        if response.clicked() || response.dragged() {
            if let Some(pos) = response.interact_pointer_pos() {
                let spacing = 3.0;
                let slots = self.bar_rects(rect, spacing);
                for (i, slot) in slots.iter().enumerate() {
                    if pos.x >= slot.left() && pos.x <= slot.right() {
                        let norm = ((slot.bottom() - pos.y) / slot.height()).clamp(0.0, 1.0);
                        self.set_partial_weight(i, norm);
                        break;
                    }
                }
            }
        }

        if ui.is_rect_visible(rect) {
            self.paint(ui.painter(), rect, accent_color, readout_color);
        }

        response
    }
}

/// Standalone percussion vector widget for rendering resonant harmonic spectrum bars.
pub fn resonant_spectrum_bar_widget(
    ui: &mut Ui,
    model: &mut ResonantSpectrumBarModel,
    accent_color: Color32,
    readout_color: Color32,
) -> Response {
    model.show(ui, accent_color, readout_color)
}

/// Headless vector model for the Nord Drum 2 Tone vs Noise balance crossfader,
/// featuring center-detent snap, soft-takeover pickup, and dual-generator color splits.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToneNoiseBalanceModel {
    /// Normalized balance position in 0.0..=1.0 (0.0 = 100% Tone, 0.5 = 50/50, 1.0 = 100% Noise).
    pub value: f32,
    /// Physical hardware controller position in 0.0..=1.0, if connected.
    pub physical_value: Option<f32>,
    /// Whether the physical control has caught up with the patch target value.
    pub is_caught: bool,
    /// Pickup threshold for soft takeover (default 0.03).
    pub catch_threshold: f32,
    /// Center detent deadzone width (default 0.02) where balance snaps cleanly to 0.50.
    pub center_deadzone: f32,
    /// Label for the Tone side (default "TONE").
    pub tone_label: String,
    /// Label for the Noise side (default "NOISE").
    pub noise_label: String,
}

/// Type alias matching the widget function naming for Tone/Noise balance sliders.
pub type ToneNoiseBalanceSliderModel = ToneNoiseBalanceModel;

impl Default for ToneNoiseBalanceModel {
    fn default() -> Self {
        Self {
            value: 0.5,
            physical_value: None,
            is_caught: true,
            catch_threshold: 0.03,
            center_deadzone: 0.02,
            tone_label: "TONE".to_string(),
            noise_label: "NOISE".to_string(),
        }
    }
}

impl ToneNoiseBalanceModel {
    /// Creates a new tone/noise balance model with initial normalized balance in 0.0..=1.0.
    #[must_use]
    pub fn new(value: f32) -> Self {
        let mut model = Self::default();
        model.set_balance(value);
        model
    }

    /// Sets custom side labels for Tone and Noise.
    #[must_use]
    pub fn with_labels(mut self, tone: impl Into<String>, noise: impl Into<String>) -> Self {
        self.tone_label = tone.into();
        self.noise_label = noise.into();
        self
    }

    /// Configures initial physical controller position for soft takeover.
    #[must_use]
    pub fn with_physical_value(mut self, physical: f32) -> Self {
        let phys = physical.clamp(0.0, 1.0);
        self.physical_value = Some(phys);
        let dist = (phys - self.value).abs();
        self.is_caught = dist <= self.catch_threshold;
        self
    }

    /// Sets custom pickup catch threshold.
    #[must_use]
    pub fn with_catch_threshold(mut self, threshold: f32) -> Self {
        self.catch_threshold = threshold.max(0.001);
        self
    }

    /// Sets custom center detent deadzone.
    #[must_use]
    pub fn with_center_deadzone(mut self, deadzone: f32) -> Self {
        self.center_deadzone = deadzone.max(0.0);
        self
    }

    /// Returns the Tone contribution fraction in 0.0..=1.0.
    #[must_use]
    pub fn tone_ratio(&self) -> f32 {
        1.0 - self.value
    }

    /// Returns the Noise contribution fraction in 0.0..=1.0.
    #[must_use]
    pub fn noise_ratio(&self) -> f32 {
        self.value
    }

    /// Returns the Tone percentage (0..100).
    #[must_use]
    pub fn tone_percentage(&self) -> u32 {
        ((1.0 - self.value) * 100.0).round() as u32
    }

    /// Returns the Noise percentage (0..100).
    #[must_use]
    pub fn noise_percentage(&self) -> u32 {
        (self.value * 100.0).round() as u32
    }

    /// Sets balance position with center detent snap.
    pub fn set_balance(&mut self, target: f32) {
        let clamped = target.clamp(0.0, 1.0);
        if (clamped - 0.5).abs() <= self.center_deadzone {
            self.value = 0.5;
        } else {
            self.value = clamped;
        }
    }

    /// Updates physical hardware position and evaluates soft takeover.
    pub fn update_physical_value(&mut self, new_phys: f32) -> SoftTakeoverOutcome {
        let clamped = new_phys.clamp(0.0, 1.0);
        self.physical_value = Some(clamped);

        if self.is_caught {
            self.set_balance(clamped);
            SoftTakeoverOutcome::Caught { new_value: self.value }
        } else {
            let dist = (clamped - self.value).abs();
            if dist <= self.catch_threshold {
                self.is_caught = true;
                self.set_balance(clamped);
                SoftTakeoverOutcome::Caught { new_value: self.value }
            } else {
                SoftTakeoverOutcome::PendingCatch { distance: dist }
            }
        }
    }

    /// Computes the slider track groove rectangle.
    #[must_use]
    pub fn track_rect(&self, total_rect: Rect) -> Rect {
        let h = 12.0;
        let margin_x = 10.0;
        Rect::from_min_size(
            Pos2::new(total_rect.left() + margin_x, total_rect.center().y - h * 0.5),
            Vec2::new((total_rect.width() - margin_x * 2.0).max(1.0), h),
        )
    }

    /// Computes the center detent position.
    #[must_use]
    pub fn center_notch_pos(&self, track: Rect) -> Pos2 {
        track.center()
    }

    /// Computes the Tone (left) portion of the track.
    #[must_use]
    pub fn tone_fill_rect(&self, track: Rect) -> Rect {
        let center_x = track.center().x;
        let thumb_x = track.left() + track.width() * self.value;
        if thumb_x < center_x {
            Rect::from_min_max(Pos2::new(thumb_x, track.top()), Pos2::new(center_x, track.bottom()))
        } else {
            Rect::from_min_max(Pos2::new(track.left(), track.top()), Pos2::new(center_x, track.bottom()))
        }
    }

    /// Computes the Noise (right) portion of the track.
    #[must_use]
    pub fn noise_fill_rect(&self, track: Rect) -> Rect {
        let center_x = track.center().x;
        let thumb_x = track.left() + track.width() * self.value;
        if thumb_x > center_x {
            Rect::from_min_max(Pos2::new(center_x, track.top()), Pos2::new(thumb_x, track.bottom()))
        } else {
            Rect::from_min_max(Pos2::new(center_x, track.top()), Pos2::new(track.right(), track.bottom()))
        }
    }

    /// Computes active slider thumb rectangle.
    #[must_use]
    pub fn thumb_rect(&self, track: Rect, thumb_width: f32) -> Rect {
        let thumb_x = track.left() + track.width() * self.value;
        let thumb_h = track.height() + 8.0;
        Rect::from_center_size(Pos2::new(thumb_x, track.center().y), Vec2::new(thumb_width, thumb_h))
    }

    /// Computes ghost thumb indicator rectangle if soft takeover is pending.
    #[must_use]
    pub fn ghost_thumb_rect(&self, track: Rect, thumb_width: f32) -> Option<Rect> {
        if self.is_caught {
            None
        } else {
            self.physical_value.map(|phys| {
                let ghost_x = track.left() + track.width() * phys;
                let thumb_h = track.height() + 8.0;
                Rect::from_center_size(Pos2::new(ghost_x, track.center().y), Vec2::new(thumb_width, thumb_h))
            })
        }
    }

    /// Paints the tone/noise balance slider.
    pub fn paint(
        &self,
        painter: &Painter,
        rect: Rect,
        tone_color: Color32,
        noise_color: Color32,
    ) {
        // Track groove
        let track = self.track_rect(rect);
        painter.rect_filled(track, Rounding::ZERO, Color32::from_gray(24));
        painter.rect_stroke(track, Rounding::ZERO, Stroke::new(1.0, Color32::from_gray(50)));

        // Tone half (left)
        let tone_half = Rect::from_min_max(track.min, Pos2::new(track.center().x, track.bottom()));
        let tone_alpha = ((1.0 - self.value) * 180.0) as u8 + 20;
        let tone_fill = Color32::from_rgba_premultiplied(
            tone_color.r() / 3,
            tone_color.g() / 3,
            tone_color.b() / 3,
            tone_alpha,
        );
        painter.rect_filled(tone_half, Rounding::ZERO, tone_fill);

        // Noise half (right)
        let noise_half = Rect::from_min_max(Pos2::new(track.center().x, track.top()), track.max);
        let noise_alpha = (self.value * 180.0) as u8 + 20;
        let noise_fill = Color32::from_rgba_premultiplied(
            noise_color.r() / 3,
            noise_color.g() / 3,
            noise_color.b() / 3,
            noise_alpha,
        );
        painter.rect_filled(noise_half, Rounding::ZERO, noise_fill);

        // Center detent vertical notch
        let notch = self.center_notch_pos(track);
        painter.line_segment(
            [
                Pos2::new(notch.x, track.top() - 3.0),
                Pos2::new(notch.x, track.bottom() + 3.0),
            ],
            Stroke::new(1.5, Color32::from_gray(140)),
        );

        // Ghost thumb indicator for soft takeover
        if let Some(ghost) = self.ghost_thumb_rect(track, 10.0) {
            painter.rect_stroke(ghost, Rounding::ZERO, Stroke::new(1.5, Color32::from_rgb(255, 179, 0)));
        }

        // Active Thumb Handle
        let thumb = self.thumb_rect(track, 12.0);
        let thumb_color = if self.value < 0.48 {
            tone_color
        } else if self.value > 0.52 {
            noise_color
        } else {
            Color32::WHITE
        };
        painter.rect_filled(thumb, Rounding::ZERO, Color32::from_gray(40));
        painter.rect_stroke(thumb, Rounding::ZERO, Stroke::new(1.5, thumb_color));

        // Percentage text labels above track
        let tone_text = format!("{} {}%", self.tone_label, self.tone_percentage());
        let noise_text = format!("{}% {}", self.noise_percentage(), self.noise_label);
        painter.text(
            Pos2::new(track.left(), rect.top() + 4.0),
            Align2::LEFT_TOP,
            &tone_text,
            FontId::proportional(10.0),
            tone_color,
        );
        painter.text(
            Pos2::new(track.right(), rect.top() + 4.0),
            Align2::RIGHT_TOP,
            &noise_text,
            FontId::proportional(10.0),
            noise_color,
        );
    }

    /// Renders the interactive tone/noise balance slider widget.
    pub fn show(
        &mut self,
        ui: &mut Ui,
        tone_color: Color32,
        noise_color: Color32,
    ) -> Response {
        let desired_size = Vec2::new(160.0, 48.0);
        let (rect, response) = ui.allocate_exact_size(desired_size, Sense::click_and_drag());

        if response.clicked() || response.dragged() {
            if let Some(pos) = response.interact_pointer_pos() {
                let track = self.track_rect(rect);
                let norm = ((pos.x - track.left()) / track.width()).clamp(0.0, 1.0);
                self.set_balance(norm);
            }
        }

        if ui.is_rect_visible(rect) {
            self.paint(ui.painter(), rect, tone_color, noise_color);
        }

        response
    }
}

/// Standalone percussion vector widget for rendering the Tone vs Noise balance crossfader.
pub fn tone_noise_balance_slider(
    ui: &mut Ui,
    model: &mut ToneNoiseBalanceModel,
    tone_color: Color32,
    noise_color: Color32,
) -> Response {
    model.show(ui, tone_color, noise_color)
}

/// Headless vector badge model for the Nord Drum 2 voice channel selector,
/// displaying channel index, voice name, CC 70 focus state, and Solo/Mute LED indicators.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VoiceFocusBadgeModel {
    /// Zero-based channel index (0..5 for Voices 1..6).
    pub voice_index: usize,
    /// Channel display name (for example "Kick", "Snare", "Hi-Hat").
    pub voice_name: String,
    /// Whether this voice currently holds editing focus via CC 70.
    pub is_focused: bool,
    /// Whether this voice is soloed.
    pub is_solo: bool,
    /// Whether this voice is muted.
    pub is_muted: bool,
    /// Assigned MIDI channel (1..16).
    pub midi_channel: u8,
    /// Assigned MIDI trigger note number (default 60 + voice_index).
    pub midi_note: u8,
    /// Continuous controller 70 index value (equal to voice_index as u8).
    pub cc70_value: u8,
}

impl VoiceFocusBadgeModel {
    /// Creates a new voice focus badge model for a voice channel index in 0..5.
    #[must_use]
    pub fn new(voice_index: usize, name: impl Into<String>) -> Self {
        let idx = voice_index.min(5);
        Self {
            voice_index: idx,
            voice_name: name.into(),
            is_focused: false,
            is_solo: false,
            is_muted: false,
            midi_channel: (idx as u8) + 1,
            midi_note: 60 + (idx as u8),
            cc70_value: idx as u8,
        }
    }

    /// Configures MIDI channel and trigger note.
    #[must_use]
    pub fn with_midi(mut self, channel: u8, note: u8) -> Self {
        self.midi_channel = channel.clamp(1, 16);
        self.midi_note = note;
        self
    }

    /// Sets initial focus state.
    #[must_use]
    pub fn with_focus(mut self, focused: bool) -> Self {
        self.is_focused = focused;
        self
    }

    /// Sets initial solo state.
    #[must_use]
    pub fn with_solo(mut self, solo: bool) -> Self {
        self.is_solo = solo;
        self
    }

    /// Sets initial mute state.
    #[must_use]
    pub fn with_muted(mut self, muted: bool) -> Self {
        self.is_muted = muted;
        self
    }

    /// Sets editing focus state.
    pub fn set_focus(&mut self, focused: bool) {
        self.is_focused = focused;
    }

    /// Toggles hardware solo state.
    pub fn toggle_solo(&mut self) -> bool {
        self.is_solo = !self.is_solo;
        self.is_solo
    }

    /// Toggles hardware mute state.
    pub fn toggle_mute(&mut self) -> bool {
        self.is_muted = !self.is_muted;
        self.is_muted
    }

    /// Returns 1-based voice number in 1..6.
    #[must_use]
    pub fn voice_number(&self) -> usize {
        self.voice_index + 1
    }

    /// Returns formatted display title string (for example "VOICE 1: KICK").
    #[must_use]
    pub fn display_title(&self) -> String {
        format!("VOICE {}: {}", self.voice_number(), self.voice_name.to_uppercase())
    }

    /// Computes layout rectangles for a horizontal strip of voice badges.
    #[must_use]
    pub fn compute_voice_strip_rects(total_rect: Rect, count: usize, spacing: f32) -> Vec<Rect> {
        if count == 0 {
            return Vec::new();
        }
        let badge_width = ((total_rect.width() - spacing * (count - 1) as f32) / count as f32).max(1.0);
        let mut rects = Vec::with_capacity(count);
        for i in 0..count {
            let left = total_rect.left() + i as f32 * (badge_width + spacing);
            rects.push(Rect::from_min_size(
                Pos2::new(left, total_rect.top()),
                Vec2::new(badge_width, total_rect.height()),
            ));
        }
        rects
    }

    /// Computes rectangles for focus LED, solo LED, and mute LED indicators inside badge_rect.
    #[must_use]
    pub fn led_indicator_rects(&self, badge_rect: Rect) -> (Rect, Rect, Rect) {
        let led_size = Vec2::new(8.0, 8.0);
        let y_top = badge_rect.top() + 6.0;
        let y_bot = badge_rect.bottom() - 14.0;

        let focus_led = Rect::from_min_size(Pos2::new(badge_rect.left() + 6.0, y_top), led_size);
        let solo_led = Rect::from_min_size(Pos2::new(badge_rect.left() + 6.0, y_bot), led_size);
        let mute_led = Rect::from_min_size(Pos2::new(badge_rect.left() + 18.0, y_bot), led_size);

        (focus_led, solo_led, mute_led)
    }

    /// Paints the voice focus badge.
    pub fn paint(
        &self,
        painter: &Painter,
        rect: Rect,
        accent_color: Color32,
        readout_color: Color32,
    ) {
        let (bg_fill, border_stroke) = if self.is_focused {
            (
                Color32::from_rgb(45, 14, 14),
                Stroke::new(2.0, accent_color),
            )
        } else {
            (
                Color32::from_gray(26),
                Stroke::new(1.0, Color32::from_gray(55)),
            )
        };

        painter.rect_filled(rect, Rounding::ZERO, bg_fill);
        painter.rect_stroke(rect, Rounding::ZERO, border_stroke);

        let (focus_led, solo_led, mute_led) = self.led_indicator_rects(rect);

        // Focus LED (Amber #FFB300 when focused)
        let focus_color = if self.is_focused {
            readout_color
        } else {
            Color32::from_gray(40)
        };
        painter.rect_filled(focus_led, Rounding::ZERO, focus_color);
        painter.rect_stroke(focus_led, Rounding::ZERO, Stroke::new(0.5, Color32::BLACK));

        // Solo LED (Amber #FFB000 when soloed)
        let solo_color = if self.is_solo {
            Color32::from_rgb(255, 176, 0)
        } else {
            Color32::from_gray(35)
        };
        painter.rect_filled(solo_led, Rounding::ZERO, solo_color);

        // Mute LED (Red #FF2020 when muted)
        let mute_color = if self.is_muted {
            Color32::from_rgb(255, 32, 32)
        } else {
            Color32::from_gray(35)
        };
        painter.rect_filled(mute_led, Rounding::ZERO, mute_color);

        // Voice Title
        let title_color = if self.is_focused {
            Color32::WHITE
        } else {
            Color32::from_gray(180)
        };
        painter.text(
            Pos2::new(focus_led.right() + 6.0, rect.top() + 5.0),
            Align2::LEFT_TOP,
            self.display_title(),
            FontId::proportional(11.0),
            title_color,
        );

        // MIDI channel and trigger note
        let midi_info = format!("CH {} | N{}", self.midi_channel, self.midi_note);
        painter.text(
            Pos2::new(mute_led.right() + 8.0, rect.bottom() - 14.0),
            Align2::LEFT_TOP,
            &midi_info,
            FontId::monospace(9.0),
            Color32::from_gray(140),
        );

        // CC 70 indicator badge
        let cc_text = format!("CC70:{}", self.cc70_value);
        painter.text(
            Pos2::new(rect.right() - 6.0, rect.bottom() - 14.0),
            Align2::RIGHT_TOP,
            &cc_text,
            FontId::monospace(8.0),
            if self.is_focused { readout_color } else { Color32::from_gray(100) },
        );
    }

    /// Renders the interactive voice focus badge widget.
    pub fn show(
        &mut self,
        ui: &mut Ui,
        accent_color: Color32,
        readout_color: Color32,
    ) -> Response {
        let desired_size = Vec2::new(120.0, 44.0);
        let (rect, response) = ui.allocate_exact_size(desired_size, Sense::click());

        if response.clicked() {
            self.set_focus(true);
        }

        if ui.is_rect_visible(rect) {
            self.paint(ui.painter(), rect, accent_color, readout_color);
        }

        response
    }
}

/// Standalone percussion vector widget for rendering a voice focus selector badge.
pub fn voice_focus_badge(
    ui: &mut Ui,
    model: &mut VoiceFocusBadgeModel,
    accent_color: Color32,
    readout_color: Color32,
) -> Response {
    model.show(ui, accent_color, readout_color)
}

// ---------------------------------------------------------------------------
// Novation Peak Custom Vector Widgets
// ---------------------------------------------------------------------------

/// Model for Novation Peak 5-wave wavetable interpolator display.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WavetableInterpolatorModel {
    /// Normalized interpolation position within the wavetable (0.0..=1.0).
    pub position: f32,
    /// Selected wavetable bank index (0..16, Novation Peak has 17 wavetables).
    pub wavetable_index: usize,
    /// Number of base waveforms interpolated across (Novation Peak standard is 5).
    pub base_wave_count: usize,
    /// Display resolution (number of polyline evaluation points).
    pub num_points: usize,
}

impl Default for WavetableInterpolatorModel {
    fn default() -> Self {
        Self {
            position: 0.0,
            wavetable_index: 0,
            base_wave_count: 5,
            num_points: 64,
        }
    }
}

impl WavetableInterpolatorModel {
    /// Creates a new wavetable interpolator display model.
    #[must_use]
    pub fn new(position: f32, wavetable_index: usize) -> Self {
        Self {
            position: position.clamp(0.0, 1.0),
            wavetable_index: wavetable_index.min(16),
            base_wave_count: 5,
            num_points: 64,
        }
    }

    /// Sets interpolation position in 0.0..=1.0.
    #[must_use]
    pub fn with_position(mut self, position: f32) -> Self {
        self.position = position.clamp(0.0, 1.0);
        self
    }

    /// Sets wavetable index (0..=16).
    #[must_use]
    pub fn with_wavetable_index(mut self, index: usize) -> Self {
        self.wavetable_index = index.min(16);
        self
    }

    /// Evaluates sample amplitude in -1.0..=1.0 for a given normalized phase (0.0..=1.0)
    /// across the 5 fundamental base waveform shapes.
    #[must_use]
    pub fn evaluate_base_wave(phase: f32, wave_type: usize) -> f32 {
        let p = phase.rem_euclid(1.0);
        use std::f32::consts::PI;
        match wave_type % 5 {
            0 => (p * 2.0 * PI).sin(),
            1 => 2.0 * (2.0 * (p - (p + 0.5).floor())).abs() - 1.0,
            2 => 2.0 * (p - (p + 0.5).floor()),
            3 => {
                if p < 0.5 {
                    1.0
                } else {
                    -1.0
                }
            }
            _ => {
                0.55 * (p * 2.0 * PI).sin()
                    + 0.30 * (p * 6.0 * PI).sin()
                    + 0.15 * (p * 10.0 * PI).sin()
            }
        }
    }

    /// Evaluates sample amplitude at `phase` linearly interpolated across the 5 waves.
    #[must_use]
    pub fn evaluate_interpolated(&self, phase: f32) -> f32 {
        let scaled_pos = self.position * (self.base_wave_count - 1) as f32;
        let idx = (scaled_pos.floor() as usize).min(self.base_wave_count - 2);
        let frac = scaled_pos - idx as f32;

        let samp_a = Self::evaluate_base_wave(phase, idx);
        let samp_b = Self::evaluate_base_wave(phase, idx + 1);
        samp_a * (1.0 - frac) + samp_b * frac
    }

    /// Generates vector points for the interpolated curve inside `rect`.
    #[must_use]
    pub fn generate_curve_points(&self, rect: Rect) -> Vec<Pos2> {
        let count = self.num_points.max(16);
        let mid_y = rect.center().y;
        let half_h = (rect.height() * 0.45).max(1.0);

        (0..=count)
            .map(|i| {
                let frac = i as f32 / count as f32;
                let x = rect.left() + frac * rect.width();
                let samp = self.evaluate_interpolated(frac);
                let y = mid_y - samp * half_h;
                Pos2::new(x, y)
            })
            .collect()
    }

    /// Generates ghost curves for all 5 underlying base waves.
    #[must_use]
    pub fn generate_ghost_waves(&self, rect: Rect) -> Vec<Vec<Pos2>> {
        let count = (self.num_points / 2).max(16);
        let mid_y = rect.center().y;
        let half_h = (rect.height() * 0.40).max(1.0);

        (0..self.base_wave_count)
            .map(|wave_idx| {
                (0..=count)
                    .map(|i| {
                        let frac = i as f32 / count as f32;
                        let x = rect.left() + frac * rect.width();
                        let samp = Self::evaluate_base_wave(frac, wave_idx);
                        let y = mid_y - samp * half_h;
                        Pos2::new(x, y)
                    })
                    .collect()
            })
            .collect()
    }

    /// Paints the wavetable interpolator into painter.
    pub fn paint(&self, painter: &Painter, rect: Rect, accent_color: Color32, bg_color: Color32) {
        painter.rect_filled(rect, Rounding::ZERO, bg_color);
        painter.rect_stroke(rect, Rounding::ZERO, Stroke::new(1.0, Color32::from_gray(60)));

        let ghost_waves = self.generate_ghost_waves(rect);
        for pts in ghost_waves {
            if pts.len() >= 2 {
                painter.add(Shape::line(pts, Stroke::new(1.0, Color32::from_white_alpha(30))));
            }
        }

        let mid_y = rect.center().y;
        painter.line_segment(
            [Pos2::new(rect.left(), mid_y), Pos2::new(rect.right(), mid_y)],
            Stroke::new(1.0, Color32::from_white_alpha(40)),
        );

        let curve_pts = self.generate_curve_points(rect);
        if curve_pts.len() >= 2 {
            painter.add(Shape::line(curve_pts, Stroke::new(2.0, accent_color)));
        }

        let text = format!("WT #{} Pos: {:.0}%", self.wavetable_index + 1, self.position * 100.0);
        painter.text(
            Pos2::new(rect.left() + 6.0, rect.top() + 6.0),
            Align2::LEFT_TOP,
            text,
            FontId::proportional(11.0),
            Color32::from_gray(200),
        );
    }

    /// Renders the wavetable interpolator widget in UI.
    pub fn show(&self, ui: &mut Ui, size: Vec2, accent_color: Color32) -> Response {
        let (rect, response) = ui.allocate_exact_size(size, Sense::hover());
        if ui.is_rect_visible(rect) {
            self.paint(ui.painter(), rect, accent_color, Color32::from_gray(18));
        }
        response
    }
}

/// Standalone widget helper for wavetable interpolator.
pub fn wavetable_interpolator_widget(
    ui: &mut Ui,
    model: &WavetableInterpolatorModel,
    size: Vec2,
    accent_color: Color32,
) -> Response {
    model.show(ui, size, accent_color)
}

/// State-Variable Filter (SVF) mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SvfFilterMode {
    LowPass12,
    LowPass24,
    BandPass,
    HighPass,
}

impl Default for SvfFilterMode {
    fn default() -> Self {
        Self::LowPass24
    }
}

impl SvfFilterMode {
    #[must_use]
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::LowPass12 => "12dB LP",
            Self::LowPass24 => "24dB LP",
            Self::BandPass => "Band Pass",
            Self::HighPass => "High Pass",
        }
    }
}

/// Headless model and vector painter for Novation Peak State-Variable Filter (SVF)
/// frequency response and pre/post drive saturation curves.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SvfFilterResponseModel {
    pub mode: SvfFilterMode,
    pub cutoff_hz: f32,
    pub resonance: f32,
    pub pre_drive: f32,
    pub post_drive: f32,
}

impl Default for SvfFilterResponseModel {
    fn default() -> Self {
        Self {
            mode: SvfFilterMode::LowPass24,
            cutoff_hz: 1000.0,
            resonance: 0.2,
            pre_drive: 0.0,
            post_drive: 0.0,
        }
    }
}

impl SvfFilterResponseModel {
    /// Creates a new SVF filter response model.
    #[must_use]
    pub fn new(cutoff_hz: f32, resonance: f32, mode: SvfFilterMode) -> Self {
        Self {
            mode,
            cutoff_hz: cutoff_hz.clamp(20.0, 20000.0),
            resonance: resonance.clamp(0.0, 1.0),
            pre_drive: 0.0,
            post_drive: 0.0,
        }
    }

    /// Sets pre-filter and post-filter drive amounts (0.0..=1.0).
    #[must_use]
    pub fn with_drive(mut self, pre_drive: f32, post_drive: f32) -> Self {
        self.pre_drive = pre_drive.clamp(0.0, 1.0);
        self.post_drive = post_drive.clamp(0.0, 1.0);
        self
    }

    /// Computes filter magnitude in decibels (dB) at a given frequency in Hz.
    #[must_use]
    pub fn magnitude_db_at(&self, freq_hz: f32) -> f32 {
        let f = freq_hz.clamp(10.0, 40000.0);
        let fc = self.cutoff_hz.clamp(20.0, 20000.0);
        let ratio = f / fc;

        let q = 0.707 + self.resonance * 11.3;
        let denom_sq = (1.0 - ratio * ratio).powi(2) + (ratio / q).powi(2);
        let denom = denom_sq.sqrt().max(1e-6);

        let base_linear = match self.mode {
            SvfFilterMode::LowPass12 => 1.0 / denom,
            SvfFilterMode::LowPass24 => (1.0 / denom).powi(2),
            SvfFilterMode::BandPass => (ratio / q) / denom,
            SvfFilterMode::HighPass => (ratio * ratio) / denom,
        };

        let drive_boost = 1.0 + self.pre_drive * 1.5 + self.post_drive * 1.0;
        let driven = base_linear * drive_boost;

        20.0 * driven.log10()
    }

    /// Generates response curve points mapped into `rect` from `min_hz` to `max_hz`.
    #[must_use]
    pub fn generate_curve_points(
        &self,
        rect: Rect,
        min_hz: f32,
        max_hz: f32,
        num_points: usize,
    ) -> Vec<Pos2> {
        let count = num_points.max(16);
        let log_min = min_hz.ln();
        let log_max = max_hz.ln();

        let min_db = -48.0;
        let max_db = 18.0;
        let db_span = max_db - min_db;

        (0..=count)
            .map(|i| {
                let frac = i as f32 / count as f32;
                let log_freq = log_min + frac * (log_max - log_min);
                let freq = log_freq.exp();
                let db = self.magnitude_db_at(freq).clamp(min_db, max_db);

                let x = rect.left() + frac * rect.width();
                let y_norm = 1.0 - (db - min_db) / db_span;
                let y = rect.top() + y_norm * rect.height();
                Pos2::new(x, y)
            })
            .collect()
    }

    /// Paints the SVF response curve into painter.
    pub fn paint(&self, painter: &Painter, rect: Rect, accent_color: Color32, bg_color: Color32) {
        painter.rect_filled(rect, Rounding::ZERO, bg_color);
        painter.rect_stroke(rect, Rounding::ZERO, Stroke::new(1.0, Color32::from_gray(60)));

        let min_hz: f32 = 20.0;
        let max_hz: f32 = 20000.0;
        let log_min = min_hz.ln();
        let log_max = max_hz.ln();

        for marker_hz in [100.0_f32, 1000.0, 10000.0] {
            let frac = (marker_hz.ln() - log_min) / (log_max - log_min);
            let x = rect.left() + frac * rect.width();
            painter.line_segment(
                [Pos2::new(x, rect.top()), Pos2::new(x, rect.bottom())],
                Stroke::new(1.0, Color32::from_white_alpha(20)),
            );
        }

        let min_db = -48.0;
        let max_db = 18.0;
        let zero_db_y = rect.top() + (1.0 - (0.0 - min_db) / (max_db - min_db)) * rect.height();
        painter.line_segment(
            [Pos2::new(rect.left(), zero_db_y), Pos2::new(rect.right(), zero_db_y)],
            Stroke::new(1.0, Color32::from_white_alpha(35)),
        );

        let pts = self.generate_curve_points(rect, min_hz, max_hz, 80);
        if pts.len() >= 2 {
            let mut poly = pts.clone();
            poly.push(Pos2::new(rect.right(), rect.bottom()));
            poly.push(Pos2::new(rect.left(), rect.bottom()));
            painter.add(Shape::convex_polygon(
                poly,
                Color32::from_rgba_premultiplied(
                    accent_color.r() / 6,
                    accent_color.g() / 6,
                    accent_color.b() / 6,
                    40,
                ),
                Stroke::NONE,
            ));
            painter.add(Shape::line(pts, Stroke::new(2.0, accent_color)));
        }

        let label = format!(
            "{} | Cutoff: {:.0}Hz | Res: {:.0}% | Pre-Drive: {:.0}% | Post-Drive: {:.0}%",
            self.mode.display_name(),
            self.cutoff_hz,
            self.resonance * 100.0,
            self.pre_drive * 100.0,
            self.post_drive * 100.0
        );
        painter.text(
            Pos2::new(rect.left() + 6.0, rect.top() + 6.0),
            Align2::LEFT_TOP,
            label,
            FontId::proportional(11.0),
            Color32::from_gray(210),
        );
    }

    /// Renders the SVF response widget in UI.
    pub fn show(&self, ui: &mut Ui, size: Vec2, accent_color: Color32) -> Response {
        let (rect, response) = ui.allocate_exact_size(size, Sense::hover());
        if ui.is_rect_visible(rect) {
            self.paint(ui.painter(), rect, accent_color, Color32::from_gray(18));
        }
        response
    }
}

/// Standalone widget helper for SVF filter response curve.
pub fn svf_filter_response_widget(
    ui: &mut Ui,
    model: &SvfFilterResponseModel,
    size: Vec2,
    accent_color: Color32,
) -> Response {
    model.show(ui, size, accent_color)
}

/// Headless model and multi-curve vector painter for 3x Novation Peak ADSR envelopes
/// (Amp Env, Mod Env 1, and Mod Env 2).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AdsrEnvelopePlotterModel {
    pub amp_env: EnvelopeCurveModel,
    pub mod_env_1: EnvelopeCurveModel,
    pub mod_env_2: EnvelopeCurveModel,
    /// Currently focused envelope index (0 = Amp, 1 = Mod 1, 2 = Mod 2).
    pub active_index: usize,
}

impl Default for AdsrEnvelopePlotterModel {
    fn default() -> Self {
        Self {
            amp_env: EnvelopeCurveModel::new(0.05, 0.25, 0.70, 0.35),
            mod_env_1: EnvelopeCurveModel::new(0.15, 0.40, 0.30, 0.50),
            mod_env_2: EnvelopeCurveModel::new(0.01, 0.10, 0.00, 0.20),
            active_index: 0,
        }
    }
}

impl AdsrEnvelopePlotterModel {
    /// Creates a 3x ADSR envelope plotter model.
    #[must_use]
    pub fn new(
        amp_env: EnvelopeCurveModel,
        mod_env_1: EnvelopeCurveModel,
        mod_env_2: EnvelopeCurveModel,
    ) -> Self {
        Self {
            amp_env,
            mod_env_1,
            mod_env_2,
            active_index: 0,
        }
    }

    /// Sets the active envelope index (0..=2).
    #[must_use]
    pub fn with_active(mut self, index: usize) -> Self {
        self.active_index = index.min(2);
        self
    }

    /// Sets active envelope index.
    pub fn set_active(&mut self, index: usize) {
        self.active_index = index.min(2);
    }

    /// Returns reference to active envelope.
    #[must_use]
    pub fn active_env(&self) -> &EnvelopeCurveModel {
        match self.active_index {
            0 => &self.amp_env,
            1 => &self.mod_env_1,
            _ => &self.mod_env_2,
        }
    }

    /// Returns mutable reference to active envelope.
    pub fn active_env_mut(&mut self) -> &mut EnvelopeCurveModel {
        match self.active_index {
            0 => &mut self.amp_env,
            1 => &mut self.mod_env_1,
            _ => &mut self.mod_env_2,
        }
    }

    /// Generates curve points for envelope `env_idx` mapped to `rect`.
    #[must_use]
    pub fn generate_envelope_points(
        &self,
        env_idx: usize,
        rect: Rect,
        num_points: usize,
    ) -> Vec<Pos2> {
        let env = match env_idx {
            0 => &self.amp_env,
            1 => &self.mod_env_1,
            _ => &self.mod_env_2,
        };
        let count = num_points.max(16);
        let top = rect.top() + 8.0;
        let bottom = rect.bottom() - 8.0;
        let height = bottom - top;

        (0..=count)
            .map(|i| {
                let frac = i as f32 / count as f32;
                let amp = env.evaluate_at(frac);
                let x = rect.left() + frac * rect.width();
                let y = bottom - amp * height;
                Pos2::new(x, y)
            })
            .collect()
    }

    /// Paints the 3x ADSR curves into painter.
    pub fn paint(&self, painter: &Painter, rect: Rect, colors: [Color32; 3], bg_color: Color32) {
        painter.rect_filled(rect, Rounding::ZERO, bg_color);
        painter.rect_stroke(rect, Rounding::ZERO, Stroke::new(1.0, Color32::from_gray(60)));

        let names = ["Amp Env", "Mod Env 1", "Mod Env 2"];

        for i in 0..3 {
            let is_active = i == self.active_index;
            let stroke_width = if is_active { 2.5 } else { 1.2 };
            let color = if is_active {
                colors[i]
            } else {
                Color32::from_rgba_premultiplied(
                    colors[i].r() / 2,
                    colors[i].g() / 2,
                    colors[i].b() / 2,
                    90,
                )
            };

            let pts = self.generate_envelope_points(i, rect, 64);
            if pts.len() >= 2 {
                if is_active {
                    let mut poly = pts.clone();
                    poly.push(Pos2::new(rect.right(), rect.bottom() - 8.0));
                    poly.push(Pos2::new(rect.left(), rect.bottom() - 8.0));
                    painter.add(Shape::convex_polygon(
                        poly,
                        Color32::from_rgba_premultiplied(
                            colors[i].r() / 6,
                            colors[i].g() / 6,
                            colors[i].b() / 6,
                            40,
                        ),
                        Stroke::NONE,
                    ));
                }
                painter.add(Shape::line(pts, Stroke::new(stroke_width, color)));
            }
        }

        let mut x_offset = rect.left() + 6.0;
        for i in 0..3 {
            let is_active = i == self.active_index;
            let text_color = if is_active {
                colors[i]
            } else {
                Color32::from_gray(140)
            };
            let label = if is_active {
                format!("[{}]", names[i])
            } else {
                names[i].to_string()
            };
            let text_pos = Pos2::new(x_offset, rect.top() + 6.0);
            let galley = painter.layout_no_wrap(label, FontId::proportional(11.0), text_color);
            x_offset += galley.size().x + 12.0;
            painter.galley(text_pos, galley, text_color);
        }
    }

    /// Renders the 3x ADSR envelope plotter in UI.
    pub fn show(&mut self, ui: &mut Ui, size: Vec2, colors: [Color32; 3]) -> Response {
        let (rect, response) = ui.allocate_exact_size(size, Sense::click());
        if response.clicked() {
            self.active_index = (self.active_index + 1) % 3;
        }
        if ui.is_rect_visible(rect) {
            self.paint(ui.painter(), rect, colors, Color32::from_gray(18));
        }
        response
    }
}

/// Standalone widget helper for 3x ADSR envelope plotter.
pub fn adsr_envelope_plotter_widget(
    ui: &mut Ui,
    model: &mut AdsrEnvelopePlotterModel,
    size: Vec2,
    colors: [Color32; 3],
) -> Response {
    model.show(ui, size, colors)
}

/// Single row item in the Novation Peak 16-slot modulation matrix grid.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModMatrixSlotItem {
    pub slot_index: usize,
    pub source_a: String,
    pub source_b: String,
    pub destination: String,
    pub depth: f32,
    pub active: bool,
}

impl ModMatrixSlotItem {
    #[must_use]
    pub fn new(slot_index: usize) -> Self {
        Self {
            slot_index,
            source_a: "Off".to_string(),
            source_b: "Direct".to_string(),
            destination: "Off".to_string(),
            depth: 0.0,
            active: false,
        }
    }
}

/// Headless model and vector table/grid painter for Novation Peak 16-Slot Mod Matrix.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModMatrixGridModel {
    pub slots: Vec<ModMatrixSlotItem>,
    pub selected_slot: Option<usize>,
}

impl Default for ModMatrixGridModel {
    fn default() -> Self {
        Self::new_empty()
    }
}

impl ModMatrixGridModel {
    /// Creates a 16-slot modulation matrix model initialized with empty slots.
    #[must_use]
    pub fn new_empty() -> Self {
        let slots = (0..16).map(ModMatrixSlotItem::new).collect();
        Self {
            slots,
            selected_slot: None,
        }
    }

    /// Sets slot values fluently.
    #[must_use]
    pub fn with_slot(
        mut self,
        idx: usize,
        src_a: impl Into<String>,
        src_b: impl Into<String>,
        dst: impl Into<String>,
        depth: f32,
    ) -> Self {
        if idx < self.slots.len() {
            self.slots[idx].source_a = src_a.into();
            self.slots[idx].source_b = src_b.into();
            self.slots[idx].destination = dst.into();
            self.slots[idx].depth = depth.clamp(-64.0, 63.0);
            self.slots[idx].active =
                self.slots[idx].destination != "Off" && self.slots[idx].depth.abs() > 0.001;
        }
        self
    }

    /// Sets slot values in-place.
    pub fn set_slot(
        &mut self,
        idx: usize,
        src_a: impl Into<String>,
        src_b: impl Into<String>,
        dst: impl Into<String>,
        depth: f32,
    ) {
        if idx < self.slots.len() {
            self.slots[idx].source_a = src_a.into();
            self.slots[idx].source_b = src_b.into();
            self.slots[idx].destination = dst.into();
            self.slots[idx].depth = depth.clamp(-64.0, 63.0);
            self.slots[idx].active =
                self.slots[idx].destination != "Off" && self.slots[idx].depth.abs() > 0.001;
        }
    }

    #[must_use]
    pub fn slot(&self, idx: usize) -> Option<&ModMatrixSlotItem> {
        self.slots.get(idx)
    }

    pub fn slot_mut(&mut self, idx: usize) -> Option<&mut ModMatrixSlotItem> {
        self.slots.get_mut(idx)
    }

    pub fn select_slot(&mut self, idx: Option<usize>) {
        self.selected_slot = idx.map(|i| i.min(15));
    }

    /// Paints the 16-slot modulation matrix into painter.
    pub fn paint(
        &self,
        painter: &Painter,
        rect: Rect,
        accent_color: Color32,
        bg_color: Color32,
    ) {
        painter.rect_filled(rect, Rounding::ZERO, bg_color);
        painter.rect_stroke(rect, Rounding::ZERO, Stroke::new(1.0, Color32::from_gray(60)));

        let header_h = 24.0;
        let row_h = (rect.height() - header_h) / 16.0;

        let header_rect = Rect::from_min_size(rect.min, Vec2::new(rect.width(), header_h));
        painter.rect_filled(header_rect, Rounding::ZERO, Color32::from_gray(28));
        painter.line_segment(
            [
                Pos2::new(rect.left(), rect.top() + header_h),
                Pos2::new(rect.right(), rect.top() + header_h),
            ],
            Stroke::new(1.0, Color32::from_gray(70)),
        );

        let col_widths = [
            rect.width() * 0.08,
            rect.width() * 0.24,
            rect.width() * 0.24,
            rect.width() * 0.28,
            rect.width() * 0.16,
        ];

        let col_titles = ["Slot", "Source A", "Source B", "Destination", "Depth"];
        let mut x_cur = rect.left();
        for (i, title) in col_titles.iter().enumerate() {
            painter.text(
                Pos2::new(x_cur + 6.0, rect.top() + 5.0),
                Align2::LEFT_TOP,
                *title,
                FontId::proportional(11.0),
                Color32::from_gray(180),
            );
            x_cur += col_widths[i];
        }

        for i in 0..16 {
            let row_y = rect.top() + header_h + i as f32 * row_h;
            let row_rect = Rect::from_min_size(
                Pos2::new(rect.left(), row_y),
                Vec2::new(rect.width(), row_h),
            );
            let is_selected = self.selected_slot == Some(i);

            if is_selected {
                painter.rect_filled(
                    row_rect,
                    Rounding::ZERO,
                    Color32::from_rgba_premultiplied(
                        accent_color.r() / 4,
                        accent_color.g() / 4,
                        accent_color.b() / 4,
                        60,
                    ),
                );
            } else if i % 2 == 1 {
                painter.rect_filled(row_rect, Rounding::ZERO, Color32::from_gray(22));
            }

            let slot = &self.slots[i];
            let text_color = if slot.active {
                Color32::from_gray(230)
            } else {
                Color32::from_gray(110)
            };

            let mut x_col = rect.left();
            painter.text(
                Pos2::new(x_col + 6.0, row_y + 3.0),
                Align2::LEFT_TOP,
                format!("{:02}", i + 1),
                FontId::monospace(10.0),
                if is_selected {
                    accent_color
                } else {
                    Color32::from_gray(140)
                },
            );
            x_col += col_widths[0];

            painter.text(
                Pos2::new(x_col + 6.0, row_y + 3.0),
                Align2::LEFT_TOP,
                &slot.source_a,
                FontId::proportional(10.0),
                text_color,
            );
            x_col += col_widths[1];

            painter.text(
                Pos2::new(x_col + 6.0, row_y + 3.0),
                Align2::LEFT_TOP,
                &slot.source_b,
                FontId::proportional(10.0),
                text_color,
            );
            x_col += col_widths[2];

            painter.text(
                Pos2::new(x_col + 6.0, row_y + 3.0),
                Align2::LEFT_TOP,
                &slot.destination,
                FontId::proportional(10.0),
                text_color,
            );
            x_col += col_widths[3];

            let depth_str = format!("{:+03.0}", slot.depth);
            let depth_color = if slot.depth > 0.0 {
                accent_color
            } else if slot.depth < 0.0 {
                Color32::from_rgb(255, 120, 50)
            } else {
                Color32::from_gray(100)
            };

            painter.text(
                Pos2::new(x_col + 4.0, row_y + 3.0),
                Align2::LEFT_TOP,
                depth_str,
                FontId::monospace(10.0),
                depth_color,
            );

            let bar_left = x_col + 40.0;
            let bar_w = (col_widths[4] - 46.0).max(10.0);
            let bar_mid = bar_left + bar_w * 0.5;
            let bar_y = row_y + row_h * 0.5 - 2.0;

            painter.line_segment(
                [
                    Pos2::new(bar_left, bar_y + 2.0),
                    Pos2::new(bar_left + bar_w, bar_y + 2.0),
                ],
                Stroke::new(1.0, Color32::from_gray(50)),
            );
            if slot.depth.abs() > 0.1 {
                let norm_depth = (slot.depth / 64.0).clamp(-1.0, 1.0);
                let x_target = bar_mid + norm_depth * (bar_w * 0.5);
                painter.line_segment(
                    [
                        Pos2::new(bar_mid, bar_y + 2.0),
                        Pos2::new(x_target, bar_y + 2.0),
                    ],
                    Stroke::new(3.0, depth_color),
                );
            }
        }
    }

    /// Renders the 16-slot modulation matrix in UI.
    pub fn show(&mut self, ui: &mut Ui, size: Vec2, accent_color: Color32) -> Response {
        let (rect, response) = ui.allocate_exact_size(size, Sense::click());
        if response.clicked() {
            if let Some(pos) = response.interact_pointer_pos() {
                let header_h = 24.0;
                let rel_y = pos.y - rect.top() - header_h;
                if rel_y >= 0.0 {
                    let row_h = (rect.height() - header_h) / 16.0;
                    let clicked_slot = (rel_y / row_h).floor() as usize;
                    if clicked_slot < 16 {
                        self.selected_slot = Some(clicked_slot);
                    }
                }
            }
        }
        if ui.is_rect_visible(rect) {
            self.paint(ui.painter(), rect, accent_color, Color32::from_gray(16));
        }
        response
    }
}

/// Standalone widget helper for 16-slot mod matrix grid.
pub fn mod_matrix_grid_widget(
    ui: &mut Ui,
    model: &mut ModMatrixGridModel,
    size: Vec2,
    accent_color: Color32,
) -> Response {
    model.show(ui, size, accent_color)
}

/// Headless model and tactile vector painter for Novation Peak Animate 1 and 2 buttons/pads.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct AnimatePadsModel {
    pub animate1_active: bool,
    pub animate1_hold: bool,
    pub animate2_active: bool,
    pub animate2_hold: bool,
}

impl AnimatePadsModel {
    /// Creates a new Animate pads model.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn trigger_animate1(&mut self) {
        self.animate1_active = true;
    }

    pub fn release_animate1(&mut self) {
        if !self.animate1_hold {
            self.animate1_active = false;
        }
    }

    pub fn toggle_hold1(&mut self) {
        self.animate1_hold = !self.animate1_hold;
        if self.animate1_hold {
            self.animate1_active = true;
        } else {
            self.animate1_active = false;
        }
    }

    pub fn trigger_animate2(&mut self) {
        self.animate2_active = true;
    }

    pub fn release_animate2(&mut self) {
        if !self.animate2_hold {
            self.animate2_active = false;
        }
    }

    pub fn toggle_hold2(&mut self) {
        self.animate2_hold = !self.animate2_hold;
        if self.animate2_hold {
            self.animate2_active = true;
        } else {
            self.animate2_active = false;
        }
    }

    /// Computes layout rectangles: (pad_1, hold_1, pad_2, hold_2).
    #[must_use]
    pub fn compute_pad_rects(&self, total_rect: Rect) -> (Rect, Rect, Rect, Rect) {
        let gap = 12.0;
        let half_w = (total_rect.width() - gap) * 0.5;
        let pad_h = total_rect.height() * 0.70;
        let hold_h = total_rect.height() - pad_h - 4.0;

        let col1_x = total_rect.left();
        let col2_x = total_rect.left() + half_w + gap;

        let pad1 = Rect::from_min_size(
            Pos2::new(col1_x, total_rect.top()),
            Vec2::new(half_w, pad_h),
        );
        let hold1 = Rect::from_min_size(
            Pos2::new(col1_x, total_rect.top() + pad_h + 4.0),
            Vec2::new(half_w, hold_h),
        );

        let pad2 = Rect::from_min_size(
            Pos2::new(col2_x, total_rect.top()),
            Vec2::new(half_w, pad_h),
        );
        let hold2 = Rect::from_min_size(
            Pos2::new(col2_x, total_rect.top() + pad_h + 4.0),
            Vec2::new(half_w, hold_h),
        );

        (pad1, hold1, pad2, hold2)
    }

    /// Paints the tactile Animate pads into painter.
    pub fn paint(&self, painter: &Painter, total_rect: Rect, accent_color: Color32) {
        let (pad1, hold1, pad2, hold2) = self.compute_pad_rects(total_rect);

        let draw_pad = |rect: Rect, label: &str, active: bool, hold: bool| {
            let bg = if active {
                accent_color
            } else {
                Color32::from_gray(32)
            };
            let border_color = if active {
                Color32::WHITE
            } else if hold {
                accent_color
            } else {
                Color32::from_gray(80)
            };
            let text_color = if active {
                Color32::BLACK
            } else {
                Color32::WHITE
            };

            painter.rect_filled(rect, Rounding::ZERO, bg);
            painter.rect_stroke(
                rect,
                Rounding::ZERO,
                Stroke::new(if active { 2.5 } else { 1.5 }, border_color),
            );

            let center = rect.center();
            painter.text(
                center,
                Align2::CENTER_CENTER,
                label,
                FontId::proportional(14.0),
                text_color,
            );
        };

        let draw_hold = |rect: Rect, hold: bool| {
            let bg = if hold {
                Color32::from_rgb(200, 140, 20)
            } else {
                Color32::from_gray(24)
            };
            let text_color = if hold {
                Color32::BLACK
            } else {
                Color32::from_gray(160)
            };

            painter.rect_filled(rect, Rounding::ZERO, bg);
            painter.rect_stroke(rect, Rounding::ZERO, Stroke::new(1.0, Color32::from_gray(70)));
            painter.text(
                rect.center(),
                Align2::CENTER_CENTER,
                if hold { "HOLD [ON]" } else { "HOLD" },
                FontId::proportional(11.0),
                text_color,
            );
        };

        draw_pad(pad1, "ANIMATE 1", self.animate1_active, self.animate1_hold);
        draw_hold(hold1, self.animate1_hold);

        draw_pad(pad2, "ANIMATE 2", self.animate2_active, self.animate2_hold);
        draw_hold(hold2, self.animate2_hold);
    }

    /// Renders the Animate pads in UI.
    pub fn show(&mut self, ui: &mut Ui, size: Vec2, accent_color: Color32) -> Response {
        let (total_rect, response) = ui.allocate_exact_size(size, Sense::click_and_drag());
        let (pad1, hold1, pad2, hold2) = self.compute_pad_rects(total_rect);

        if let Some(pos) = response.interact_pointer_pos() {
            if response.clicked() || response.drag_started() {
                if pad1.contains(pos) {
                    if self.animate1_hold {
                        self.toggle_hold1();
                    } else {
                        self.trigger_animate1();
                    }
                } else if hold1.contains(pos) {
                    self.toggle_hold1();
                } else if pad2.contains(pos) {
                    if self.animate2_hold {
                        self.toggle_hold2();
                    } else {
                        self.trigger_animate2();
                    }
                } else if hold2.contains(pos) {
                    self.toggle_hold2();
                }
            } else if response.drag_stopped() {
                if !self.animate1_hold {
                    self.release_animate1();
                }
                if !self.animate2_hold {
                    self.release_animate2();
                }
            }
        }

        if ui.is_rect_visible(total_rect) {
            self.paint(ui.painter(), total_rect, accent_color);
        }

        response
    }
}

/// Standalone widget helper for Animate 1/2 performance pads.
pub fn animate_pads_widget(
    ui: &mut Ui,
    model: &mut AnimatePadsModel,
    size: Vec2,
    accent_color: Color32,
) -> Response {
    model.show(ui, size, accent_color)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rotary_knob_angle_calculations() {
        let knob = RotaryKnobModel::new("filter_cutoff", "Cutoff", 0.5);
        let center_angle = knob.current_angle();
        assert!((center_angle - 0.0).abs() < 1e-4);

        let min_angle = knob.angle_for_norm(0.0);
        let max_angle = knob.angle_for_norm(1.0);
        assert!((min_angle - (-135.0_f32.to_radians())).abs() < 1e-4);
        assert!((max_angle - 135.0_f32.to_radians()).abs() < 1e-4);
    }

    #[test]
    fn rotary_knob_soft_takeover_catch_logic() {
        let mut knob = RotaryKnobModel::new("resonance", "Resonance", 0.70)
            .with_catch_threshold(0.05)
            .with_physical_value(0.10);

        assert!(!knob.is_caught);
        assert_eq!(knob.value, 0.70);

        // Movement below threshold stays pending
        let res1 = knob.update_physical_value(0.30);
        assert!(matches!(res1, SoftTakeoverOutcome::PendingCatch { .. }));
        assert!(!knob.is_caught);
        assert_eq!(knob.value, 0.70);

        // Movement crossing or reaching threshold catches the control
        let res2 = knob.update_physical_value(0.68);
        assert!(matches!(res2, SoftTakeoverOutcome::Caught { new_value } if (new_value - 0.68).abs() < 1e-4));
        assert!(knob.is_caught);
        assert_eq!(knob.value, 0.68);

        // After catch, changes track directly
        let res3 = knob.update_physical_value(0.85);
        assert!(matches!(res3, SoftTakeoverOutcome::Caught { new_value } if (new_value - 0.85).abs() < 1e-4));
        assert_eq!(knob.value, 0.85);
    }

    #[test]
    fn rotary_knob_ghost_ring_points_present_only_when_pending() {
        let center = Pos2::new(100.0, 100.0);
        let mut knob = RotaryKnobModel::new("lfo_speed", "Speed", 0.80)
            .with_physical_value(0.20);

        assert!(!knob.is_caught);
        let ghost_pts = knob.ghost_ring_points(center, 40.0, 16);
        assert!(ghost_pts.is_some());
        assert!(!ghost_pts.unwrap().is_empty());

        let ghost_line = knob.ghost_pointer_endpoints(center, 10.0, 30.0);
        assert!(ghost_line.is_some());

        // Catch the knob
        knob.update_physical_value(0.80);
        assert!(knob.is_caught);
        assert!(knob.ghost_ring_points(center, 40.0, 16).is_none());
        assert!(knob.ghost_pointer_endpoints(center, 10.0, 30.0).is_none());
    }

    #[test]
    fn step_led_button_state_and_toggles() {
        let mut step = StepLedButtonModel::new(0).with_note("C", 0);
        assert_eq!(step.step_number, 1);
        assert!(!step.gate_on);
        assert_eq!(step.led_state, StepLedState::Off);

        step.toggle_gate();
        assert!(step.gate_on);
        assert_eq!(step.led_state, StepLedState::On);

        step.toggle_accent();
        assert!(step.accent);

        step.toggle_slide();
        assert!(step.slide);

        step.set_current_step(true);
        assert_eq!(step.led_state, StepLedState::Blinking);
    }

    #[test]
    fn step_led_button_rects_spacing() {
        let total = Rect::from_min_size(Pos2::new(0.0, 0.0), Vec2::new(160.0, 40.0));
        let rects = StepLedButtonModel::step_rects(total, 4, 4.0);
        assert_eq!(rects.len(), 4);
        assert_eq!(rects[0].left(), 0.0);
        assert!((rects[3].right() - 160.0).abs() < 1e-3);
    }

    #[test]
    fn envelope_curve_evaluation_and_proportions() {
        let env = EnvelopeCurveModel::new(0.10, 0.20, 0.50, 0.30);
        let (fa, fd, fh, fr) = env.stage_fractions();
        assert!(((fa + fd + fh + fr) - 1.0).abs() < 1e-4);

        // At t = 0, envelope is at zero
        assert!((env.evaluate_at(0.0) - 0.0).abs() < 1e-4);

        // At attack peak, amplitude reaches 1.0
        let peak_amp = env.evaluate_at(fa);
        assert!((peak_amp - 1.0).abs() < 1e-3);

        // At sustain phase, amplitude is equal to sustain level
        let sustain_amp = env.evaluate_at(fa + fd + fh * 0.5);
        assert!((sustain_amp - 0.50).abs() < 1e-3);

        // At t = 1.0, release reaches 0.0
        assert!((env.evaluate_at(1.0) - 0.0).abs() < 1e-4);
    }

    #[test]
    fn envelope_curve_polygon_generation() {
        let env = EnvelopeCurveModel::default();
        let frame = Rect::from_min_size(Pos2::new(0.0, 0.0), Vec2::new(200.0, 100.0));
        let polyline = env.generate_polyline(frame, 32);
        assert_eq!(polyline.len(), 33);

        let polygon = env.generate_filled_polygon(frame, 32);
        assert_eq!(polygon.len(), 35); // 33 curve points + bottom right + bottom left
    }

    #[test]
    fn key_overlay_badge_bs2_25_keys_layout() {
        let keys = KeyOverlayBadgeModel::bs2_25_keys();
        assert_eq!(keys.len(), 25);
        assert_eq!(keys[0].note_name, "C");
        assert_eq!(keys[0].function_label.as_deref(), Some("OSC 1"));
        assert_eq!(keys[24].note_name, "C");
        assert_eq!(keys[24].function_label.as_deref(), Some("AFX / PARA"));

        let total_rect = Rect::from_min_size(Pos2::new(0.0, 0.0), Vec2::new(300.0, 100.0));
        let (white_rects, black_rects) = KeyOverlayBadgeModel::calculate_25_key_layout(total_rect);
        assert_eq!(white_rects.len(), 15);
        assert_eq!(black_rects.len(), 10);
        assert_eq!(white_rects.len() + black_rects.len(), 25);
    }

    #[test]
    fn drum_pad_velocity_trigger_and_decay() {
        let mut pad = DrumPadVelocityModel::new(0, "Kick");
        assert_eq!(pad.channel_index, 0);
        assert_eq!(pad.voice_number(), 1);
        assert_eq!(pad.midi_note, 60);
        assert_eq!(pad.velocity, 0);
        assert_eq!(pad.peak_velocity, 0);
        assert!(!pad.is_triggered);
        assert_eq!(pad.normalized_velocity(), 0.0);
        assert_eq!(pad.normalized_peak(), 0.0);

        // Strike pad with velocity 110
        pad.trigger(110);
        assert!(pad.is_triggered);
        assert_eq!(pad.velocity, 110);
        assert_eq!(pad.peak_velocity, 110);
        assert!((pad.normalized_velocity() - (110.0 / 127.0)).abs() < 1e-4);

        // Subsequent strike with lower velocity 80 does not lower peak
        pad.trigger(80);
        assert_eq!(pad.velocity, 80);
        assert_eq!(pad.peak_velocity, 110);

        // Release trigger
        pad.release();
        assert!(!pad.is_triggered);
        assert_eq!(pad.velocity, 80);

        // Peak decay
        pad.decay_peak(30);
        assert_eq!(pad.peak_velocity, 80);
        pad.decay_peak(100);
        assert_eq!(pad.peak_velocity, 0);
    }

    #[test]
    fn drum_pad_velocity_rect_geometry() {
        let total = Rect::from_min_size(Pos2::new(0.0, 0.0), Vec2::new(300.0, 200.0));
        let pads = DrumPadVelocityModel::compute_pad_rects(total, 6, 3, 6.0);
        assert_eq!(pads.len(), 6);
        for p in &pads {
            assert!(p.width() > 50.0);
            assert!(p.height() > 50.0);
            assert!(total.contains_rect(*p));
        }

        let model = DrumPadVelocityModel::new(0, "Snare").with_velocity(64);
        let bar = model.velocity_bar_rect(pads[0]);
        let fill = model.velocity_fill_rect(pads[0]);
        assert!(pads[0].contains_rect(bar));
        assert!(bar.contains_rect(fill));
        assert!((fill.width() - bar.width() * (64.0 / 127.0)).abs() < 1.0);
    }

    #[test]
    fn resonant_spectrum_bar_presets_and_harmonics() {
        let bell = ResonantSpectrumBarModel::from_preset("bell");
        assert_eq!(bell.partial_count, 8);
        assert_eq!(bell.harmonic_weights.len(), 8);
        assert_eq!(bell.fundamental_hz, 520.0);
        assert_eq!(bell.resonance_q, 0.90);

        let membrane = ResonantSpectrumBarModel::from_preset("membrane");
        assert_eq!(membrane.fundamental_hz, 140.0);
        assert!(membrane.harmonic_weights[0] > membrane.harmonic_weights[7]);

        let mut custom = ResonantSpectrumBarModel::new(10)
            .with_fundamental(440.0)
            .with_resonance_q(0.80)
            .with_spectral_tilt(-0.20)
            .with_decay_damping(0.40);
        assert_eq!(custom.partial_count, 10);
        custom.recompute_harmonics();
        for w in &custom.harmonic_weights {
            assert!(*w >= 0.0 && *w <= 1.0);
        }

        custom.set_partial_weight(3, 0.95);
        assert_eq!(custom.harmonic_weights[3], 0.95);
    }

    #[test]
    fn resonant_spectrum_bar_geometry() {
        let model = ResonantSpectrumBarModel::from_preset("cymbal");
        let total = Rect::from_min_size(Pos2::new(0.0, 0.0), Vec2::new(240.0, 80.0));
        let slots = model.bar_rects(total, 4.0);
        assert_eq!(slots.len(), 8);

        let filled = model.filled_bar_rects(total, 4.0);
        assert_eq!(filled.len(), 8);
        for (i, fill) in filled.iter().enumerate() {
            assert!(slots[i].contains_rect(*fill));
            assert!(fill.height() <= slots[i].height());
        }

        let poly = model.envelope_polyline(total, 4.0);
        assert_eq!(poly.len(), 8);
    }

    #[test]
    fn tone_noise_balance_center_detent_and_percentages() {
        let mut balance = ToneNoiseBalanceModel::new(0.50);
        assert_eq!(balance.value, 0.50);
        assert_eq!(balance.tone_ratio(), 0.50);
        assert_eq!(balance.noise_ratio(), 0.50);
        assert_eq!(balance.tone_percentage(), 50);
        assert_eq!(balance.noise_percentage(), 50);

        // Center detent snapping within 0.02 deadzone (0.49 snaps to 0.50)
        balance.set_balance(0.49);
        assert_eq!(balance.value, 0.50);
        balance.set_balance(0.515);
        assert_eq!(balance.value, 0.50);

        // Pure Tone (0.0)
        balance.set_balance(0.0);
        assert_eq!(balance.value, 0.0);
        assert_eq!(balance.tone_percentage(), 100);
        assert_eq!(balance.noise_percentage(), 0);

        // Tone dominant (0.20)
        balance.set_balance(0.20);
        assert_eq!(balance.tone_percentage(), 80);
        assert_eq!(balance.noise_percentage(), 20);

        // Pure Noise (1.0)
        balance.set_balance(1.0);
        assert_eq!(balance.value, 1.0);
        assert_eq!(balance.tone_percentage(), 0);
        assert_eq!(balance.noise_percentage(), 100);
    }

    #[test]
    fn tone_noise_balance_soft_takeover_catch() {
        let mut balance = ToneNoiseBalanceModel::new(0.75)
            .with_catch_threshold(0.04)
            .with_physical_value(0.20);

        assert!(!balance.is_caught);
        assert_eq!(balance.value, 0.75);

        let total = Rect::from_min_size(Pos2::new(0.0, 0.0), Vec2::new(200.0, 40.0));
        let track = balance.track_rect(total);
        assert!(balance.ghost_thumb_rect(track, 10.0).is_some());

        // Movement below threshold stays pending
        let res1 = balance.update_physical_value(0.40);
        assert!(matches!(res1, SoftTakeoverOutcome::PendingCatch { .. }));
        assert!(!balance.is_caught);
        assert_eq!(balance.value, 0.75);

        // Movement reaching threshold catches control
        let res2 = balance.update_physical_value(0.73);
        assert!(matches!(res2, SoftTakeoverOutcome::Caught { .. }));
        assert!(balance.is_caught);
        assert_eq!(balance.value, 0.73);
        assert!(balance.ghost_thumb_rect(track, 10.0).is_none());
    }

    #[test]
    fn voice_focus_badge_state_and_strip_layout() {
        let mut badges: Vec<VoiceFocusBadgeModel> = (0..6)
            .map(|i| VoiceFocusBadgeModel::new(i, format!("Voice {}", i + 1)))
            .collect();
        assert_eq!(badges.len(), 6);
        assert_eq!(badges[0].voice_number(), 1);
        assert_eq!(badges[0].cc70_value, 0);
        assert_eq!(badges[5].voice_number(), 6);
        assert_eq!(badges[5].cc70_value, 5);

        // Strip layout
        let total = Rect::from_min_size(Pos2::new(0.0, 0.0), Vec2::new(600.0, 50.0));
        let rects = VoiceFocusBadgeModel::compute_voice_strip_rects(total, 6, 8.0);
        assert_eq!(rects.len(), 6);
        assert_eq!(rects[0].left(), 0.0);

        // Indicators
        let (focus_led, solo_led, mute_led) = badges[0].led_indicator_rects(rects[0]);
        assert!(rects[0].contains_rect(focus_led));
        assert!(rects[0].contains_rect(solo_led));
        assert!(rects[0].contains_rect(mute_led));

        // Focus and solo/mute toggles
        assert!(!badges[0].is_focused);
        badges[0].set_focus(true);
        assert!(badges[0].is_focused);

        assert!(!badges[0].is_solo);
        badges[0].toggle_solo();
        assert!(badges[0].is_solo);

        assert!(!badges[0].is_muted);
        badges[0].toggle_mute();
        assert!(badges[0].is_muted);
    }

    #[test]
    fn wavetable_interpolator_evaluation_and_curves() {
        let wt = WavetableInterpolatorModel::new(0.0, 3);
        assert_eq!(wt.wavetable_index, 3);
        assert_eq!(wt.position, 0.0);

        // At position 0.0, wave matches sine wave (base wave 0)
        let s0 = wt.evaluate_interpolated(0.25);
        assert!((s0 - 1.0).abs() < 1e-3);

        // Interpolate to 1.0 (complex wave)
        let wt1 = wt.with_position(1.0);
        assert_eq!(wt1.position, 1.0);

        let rect = Rect::from_min_size(Pos2::new(0.0, 0.0), Vec2::new(200.0, 100.0));
        let curve = wt1.generate_curve_points(rect);
        assert_eq!(curve.len(), 65);
        assert_eq!(curve[0].x, 0.0);
        assert_eq!(curve[64].x, 200.0);

        let ghosts = wt1.generate_ghost_waves(rect);
        assert_eq!(ghosts.len(), 5);
    }

    #[test]
    fn svf_filter_response_magnitude_and_modes() {
        let svf_lp = SvfFilterResponseModel::new(1000.0, 0.5, SvfFilterMode::LowPass24);
        assert_eq!(svf_lp.cutoff_hz, 1000.0);

        // Low frequency magnitude is close to 0 dB
        let db_low = svf_lp.magnitude_db_at(100.0);
        assert!(db_low.abs() < 3.0);

        // High frequency is strongly attenuated in 24dB LP
        let db_high = svf_lp.magnitude_db_at(10000.0);
        assert!(db_high < -30.0);

        // Test HP mode
        let svf_hp = SvfFilterResponseModel::new(1000.0, 0.2, SvfFilterMode::HighPass);
        assert!(svf_hp.magnitude_db_at(100.0) < -20.0);
        assert!(svf_hp.magnitude_db_at(10000.0).abs() < 3.0);

        // Pre/post drive increases gain
        let svf_driven = svf_lp.with_drive(0.8, 0.5);
        assert!(svf_driven.magnitude_db_at(100.0) > db_low);

        let rect = Rect::from_min_size(Pos2::new(0.0, 0.0), Vec2::new(300.0, 150.0));
        let pts = svf_driven.generate_curve_points(rect, 20.0, 20000.0, 60);
        assert_eq!(pts.len(), 61);
    }

    #[test]
    fn adsr_envelope_plotter_active_selection_and_points() {
        let mut plotter = AdsrEnvelopePlotterModel::default();
        assert_eq!(plotter.active_index, 0);

        plotter.set_active(1);
        assert_eq!(plotter.active_index, 1);
        assert_eq!(plotter.active_env().decay, 0.40);

        let rect = Rect::from_min_size(Pos2::new(0.0, 0.0), Vec2::new(240.0, 120.0));
        let pts = plotter.generate_envelope_points(0, rect, 32);
        assert_eq!(pts.len(), 33);
        assert_eq!(pts[0].x, 0.0);
        assert_eq!(pts[32].x, 240.0);
    }

    #[test]
    fn mod_matrix_grid_slot_mutation_and_selection() {
        let mut grid = ModMatrixGridModel::new_empty();
        assert_eq!(grid.slots.len(), 16);
        assert_eq!(grid.selected_slot, None);

        grid.set_slot(0, "Lfo1", "Direct", "FilterCutoff", 45.0);
        assert!(grid.slots[0].active);
        assert_eq!(grid.slots[0].source_a, "Lfo1");
        assert_eq!(grid.slots[0].destination, "FilterCutoff");
        assert_eq!(grid.slots[0].depth, 45.0);

        grid.select_slot(Some(0));
        assert_eq!(grid.selected_slot, Some(0));
    }

    #[test]
    fn animate_pads_trigger_hold_and_layout() {
        let mut pads = AnimatePadsModel::new();
        assert!(!pads.animate1_active);
        assert!(!pads.animate1_hold);

        // Momentary trigger
        pads.trigger_animate1();
        assert!(pads.animate1_active);
        pads.release_animate1();
        assert!(!pads.animate1_active);

        // Latching Hold
        pads.toggle_hold1();
        assert!(pads.animate1_hold);
        assert!(pads.animate1_active);
        pads.release_animate1(); // release does nothing while hold is on
        assert!(pads.animate1_active);

        // Hold toggle off
        pads.toggle_hold1();
        assert!(!pads.animate1_hold);
        assert!(!pads.animate1_active);

        // Rect layout geometry
        let total = Rect::from_min_size(Pos2::new(0.0, 0.0), Vec2::new(200.0, 100.0));
        let (p1, h1, p2, h2) = pads.compute_pad_rects(total);
        assert!(total.contains_rect(p1));
        assert!(total.contains_rect(h1));
        assert!(total.contains_rect(p2));
        assert!(total.contains_rect(h2));
        assert!(!p1.intersects(p2));
    }
}
