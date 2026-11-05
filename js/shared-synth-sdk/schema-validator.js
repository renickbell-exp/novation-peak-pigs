// SPDX-License-Identifier: GPL-3.0-or-later
/**
 * @file Unified JSON Schema Specifications, Canonical Profiles, and Validators
 * across all 4 Synthesizer Projects (nkEditor, ASM Hydrasynth, BS2 + TD-3, and Nord Drum 2).
 */

import fs from 'node:fs';
import path from 'node:path';

/**
 * Standard Draft-07 JSON Schema for SynthPatch.
 */
export const SYNTH_PATCH_SCHEMA = Object.freeze({
  $schema: 'http://json-schema.org/draft-07/schema#',
  title: 'SynthPatch',
  description: 'Standard cross-project synthesizer patch document',
  type: 'object',
  required: ['meta', 'parameters'],
  properties: {
    meta: {
      type: 'object',
      required: ['id', 'name'],
      properties: {
        id: { type: 'string' },
        name: { type: 'string' },
        category: { type: 'string', default: 'Init' },
        color: { type: 'string', pattern: '^#[0-9a-fA-F]{6}$', default: '#888888' },
        favorite: { type: 'boolean', default: false },
        bank: { type: 'string', default: 'A' },
        slot: { type: 'integer', minimum: 0, default: 0 },
        createdAt: { type: 'string', format: 'date-time' },
        modifiedAt: { type: 'string', format: 'date-time' },
        tags: { type: 'array', items: { type: 'string' }, default: [] },
        author: { type: ['string', 'null'] },
        notes: { type: ['string', 'null'] },
      },
    },
    parameters: {
      type: 'object',
      description: 'Key-value map of parameter identifiers to numbers',
      additionalProperties: { type: 'number' },
    },
    sysex: {
      type: 'object',
      properties: {
        rawDump: {
          type: ['array', 'null'],
          items: { type: 'integer', minimum: 0, maximum: 255 },
        },
      },
    },
  },
});

/**
 * Standard Draft-07 JSON Schema for DeviceSynthesizerProfile.
 */
export const DEVICE_PROFILE_SCHEMA = Object.freeze({
  $schema: 'http://json-schema.org/draft-07/schema#',
  title: 'DeviceSynthesizerProfile',
  description: 'Device synthesis engine and controller layout specification',
  type: 'object',
  required: ['id', 'name', 'manufacturer', 'version', 'category', 'default_midi_channel', 'parameters'],
  properties: {
    id: { type: 'string' },
    name: { type: 'string' },
    manufacturer: { type: 'string' },
    version: { type: 'string' },
    category: {
      type: 'string',
      enum: ['synthesizer', 'controller_surface', 'hybrid'],
    },
    default_midi_channel: { type: 'integer', minimum: 0, maximum: 15 },
    parameters: {
      type: 'array',
      items: {
        type: 'object',
        required: ['id', 'name', 'module', 'address', 'range'],
        properties: {
          id: { type: 'string' },
          name: { type: 'string' },
          module: { type: 'string' },
          address: { type: 'object' },
          range: { type: 'object' },
          curve: { type: 'string' },
          is_locked: { type: 'boolean' },
        },
      },
    },
    modules: {
      type: 'array',
      items: { type: 'string' },
    },
    controller_controls: {
      type: 'array',
      items: {
        type: 'object',
        required: ['id', 'name', 'kind', 'cc', 'has_led', 'group'],
        properties: {
          id: { type: 'string' },
          name: { type: 'string' },
          kind: { type: 'string' },
          cc: { type: 'integer', minimum: 0, maximum: 127 },
          channel: { type: ['integer', 'null'] },
          has_led: { type: 'boolean' },
          led_cc: { type: ['integer', 'null'] },
          track_index: { type: ['integer', 'null'] },
          group: { type: 'string' },
        },
      },
    },
    tags: {
      type: 'array',
      items: { type: 'string' },
    },
  },
});

/**
 * Standard Draft-07 JSON Schema for ControllerSurfaceMapping.
 */
export const CONTROLLER_MAPPING_SCHEMA = Object.freeze({
  $schema: 'http://json-schema.org/draft-07/schema#',
  title: 'ControllerSurfaceMapping',
  description: 'Tactile surface hardware binding configuration across switchable banks',
  type: 'object',
  required: ['surface_name', 'version', 'banks'],
  properties: {
    surface_name: { type: 'string' },
    version: { type: 'string' },
    active_bank: { type: 'integer', minimum: 1, maximum: 3 },
    banks: {
      type: 'array',
      items: {
        type: 'object',
        required: ['bank_index', 'name', 'target_mode', 'controls'],
        properties: {
          bank_index: { type: 'integer' },
          name: { type: 'string' },
          target_mode: { type: 'string' },
          controls: {
            type: 'array',
            items: {
              type: 'object',
              required: ['hardware_cc', 'label', 'action'],
              properties: {
                hardware_cc: { type: 'integer', minimum: 0, maximum: 127 },
                label: { type: 'string' },
                action: { type: 'string' },
                target_device: { type: 'string' },
                target_param: { type: 'string' },
                led_feedback_cc: { type: ['integer', 'null'] },
              },
            },
          },
        },
      },
    },
  },
});

/**
 * Standard Draft-07 JSON Schema for ThemeConfig.
 */
export const THEME_CONFIG_SCHEMA = Object.freeze({
  $schema: 'http://json-schema.org/draft-07/schema#',
  title: 'ThemeConfig',
  description: 'Unified visualizer and editor theme palette configuration across all 4 synth projects',
  type: 'object',
  required: ['preference', 'accent', 'overrides', 'scale', 'true_black_and_white'],
  properties: {
    preference: { type: 'string' },
    two_bowls_bg: { type: ['string', 'null'] },
    accent: { type: 'string' },
    overrides: { type: 'object' },
    scale: { type: 'number', minimum: 0.5, maximum: 2.0 },
    true_black_and_white: { type: 'boolean' },
  },
});

// =============================================================================
// Canonical Device Profiles across 4 Synthesizer Projects
// =============================================================================

export const ASM_HYDRASYNTH_PROFILE = Object.freeze({
  id: 'asm_hydrasynth',
  name: 'ASM Hydrasynth Keyboard/Desktop',
  manufacturer: 'Ashun Sound Machines',
  version: '2.0.0',
  category: 'synthesizer',
  default_midi_channel: 0,
  modules: ['Osc1', 'Osc2', 'Osc3', 'Mutator1', 'Mutator2', 'Filter1', 'Filter2', 'Amp', 'Lfo1', 'Env1'],
  parameters: [
    {
      id: 'filter_1_cutoff',
      name: 'Filter 1 Cutoff',
      module: 'Filter1',
      address: { kind: 'Cc', cc: 74 },
      range: { min: 0.0, max: 127.0, step: 1.0, default_value: 64.0 },
      curve: 'Logarithmic',
      is_locked: false,
    },
    {
      id: 'filter_1_resonance',
      name: 'Filter 1 Resonance',
      module: 'Filter1',
      address: { kind: 'Cc', cc: 71 },
      range: { min: 0.0, max: 127.0, step: 1.0, default_value: 0.0 },
      curve: 'Linear',
      is_locked: false,
    },
  ],
  controller_controls: [],
  tags: ['wavetable', 'polyphonic', 'hydrasynth'],
});

export const KORG_NANOKONTROL2_PROFILE = Object.freeze({
  id: 'korg_nanokontrol2',
  name: 'Korg nanoKONTROL2',
  manufacturer: 'Korg',
  version: '1.0.0',
  category: 'controller_surface',
  default_midi_channel: 0,
  modules: ['Faders', 'Knobs', 'Solo', 'Mute', 'Record', 'Transport'],
  parameters: [],
  controller_controls: [
    { id: 'fader_1', name: 'Fader 1', kind: 'Fader', cc: 0, channel: 0, has_led: false, led_cc: null, track_index: 0, group: 'track_1' },
    { id: 'knob_1', name: 'Knob 1', kind: 'Knob', cc: 16, channel: 0, has_led: false, led_cc: null, track_index: 0, group: 'track_1' },
    { id: 'solo_1', name: 'Solo 1', kind: 'Button', cc: 32, channel: 0, has_led: true, led_cc: 32, track_index: 0, group: 'track_1' },
    { id: 'mute_1', name: 'Mute 1', kind: 'Button', cc: 48, channel: 0, has_led: true, led_cc: 48, track_index: 0, group: 'track_1' },
  ],
  tags: ['surface', 'tactile', 'nkeditor'],
});

export const NOVATION_BS2_PROFILE = Object.freeze({
  id: 'novation_bass_station_2',
  name: 'Novation Bass Station II',
  manufacturer: 'Novation',
  version: '2.5.0',
  category: 'synthesizer',
  default_midi_channel: 0,
  modules: ['Osc1', 'Osc2', 'SubOsc', 'Filter', 'EnvMod', 'EnvAmp', 'Lfo1', 'Lfo2', 'Effects', 'Arp'],
  parameters: [
    {
      id: 'filter_frequency',
      name: 'Filter Frequency',
      module: 'Filter',
      address: { kind: 'Cc', cc: 16 },
      range: { min: 0.0, max: 255.0, step: 1.0, default_value: 128.0 },
      curve: 'Logarithmic',
      is_locked: false,
    },
    {
      id: 'filter_resonance',
      name: 'Filter Resonance',
      module: 'Filter',
      address: { kind: 'Cc', cc: 82 },
      range: { min: 0.0, max: 127.0, step: 1.0, default_value: 0.0 },
      curve: 'Linear',
      is_locked: false,
    },
  ],
  controller_controls: [],
  tags: ['analog', 'monosynth', 'bass-station'],
});

export const BEHRINGER_TD3_PROFILE = Object.freeze({
  id: 'behringer_td3_md',
  name: 'Behringer TD-3-MO / TD-3',
  manufacturer: 'Behringer',
  version: '1.0.0',
  category: 'synthesizer',
  default_midi_channel: 1,
  modules: ['VCO', 'VCF', 'Envelope', 'Overdrive', 'SubOsc', 'Muffler'],
  parameters: [
    {
      id: 'filter_cutoff',
      name: 'Filter Cutoff',
      module: 'VCF',
      address: { kind: 'Cc', cc: 74 },
      range: { min: 0.0, max: 127.0, step: 1.0, default_value: 64.0 },
      curve: 'Logarithmic',
      is_locked: false,
    },
    {
      id: 'filter_resonance',
      name: 'Filter Resonance',
      module: 'VCF',
      address: { kind: 'Cc', cc: 71 },
      range: { min: 0.0, max: 127.0, step: 1.0, default_value: 0.0 },
      curve: 'Linear',
      is_locked: false,
    },
  ],
  controller_controls: [],
  tags: ['acid', 'bassline', 'td-3'],
});

export const NORD_DRUM_2_PROFILE = Object.freeze({
  id: 'clavia_nord_drum_2',
  name: 'Clavia Nord Drum 2',
  manufacturer: 'Clavia DMI AB',
  version: '3.0.0',
  category: 'synthesizer',
  default_midi_channel: 9, // Channel 10
  modules: ['Voice1', 'Voice2', 'Voice3', 'Voice4', 'Voice5', 'Voice6', 'Global'],
  parameters: [
    {
      id: 'voice_1_tone_wave',
      name: 'Voice 1 Tone Wave',
      module: 'Voice1',
      address: { kind: 'Cc', cc: 46 },
      range: { min: 0.0, max: 127.0, step: 1.0, default_value: 0.0 },
      curve: 'Linear',
      is_locked: false,
    },
    {
      id: 'voice_1_tone_decay',
      name: 'Voice 1 Tone Decay',
      module: 'Voice1',
      address: { kind: 'Cc', cc: 50 },
      range: { min: 0.0, max: 127.0, step: 1.0, default_value: 64.0 },
      curve: 'Linear',
      is_locked: false,
    },
    {
      id: 'voice_1_noise_filter_freq',
      name: 'Voice 1 Noise Filter Freq',
      module: 'Voice1',
      address: { kind: 'Cc', cc: 14 },
      range: { min: 0.0, max: 127.0, step: 1.0, default_value: 64.0 },
      curve: 'Linear',
      is_locked: false,
    },
    {
      id: 'global_focus_cc',
      name: 'Global Focus Voice Selection',
      module: 'Global',
      address: { kind: 'Cc', cc: 70 },
      range: { min: 0.0, max: 5.0, step: 1.0, default_value: 0.0 },
      curve: 'Linear',
      is_locked: false,
    },
  ],
  controller_controls: [
    { id: 'ch_1_button', name: 'Channel 1 Select', kind: 'Button', cc: 70, channel: 9, has_led: true, led_cc: null, track_index: 0, group: 'channel_selection' },
    { id: 'dial_main', name: 'Main Value Dial', kind: 'Encoder', cc: 15, channel: 9, has_led: false, led_cc: null, track_index: null, group: 'global_controls' },
  ],
  tags: ['percussion', 'drum-synthesizer', 'nord-drum-2'],
});

export const NOVATION_PEAK_PROFILE = Object.freeze({
  id: 'novation_peak',
  name: 'Novation Peak',
  manufacturer: 'Novation',
  version: '2.0.0',
  category: 'synthesizer',
  default_midi_channel: 0,
  modules: [
    'Oscillator 1', 'Oscillator 2', 'Oscillator 3', 'Mixer', 'Filter',
    'Amp Envelope', 'Mod Envelope 1', 'Mod Envelope 2', 'LFO 1', 'LFO 2',
    'Effects', 'Performance', 'Arpeggiator', 'Modulation Matrix', 'Dual-CC Pairs',
  ],
  parameters: [
    // -------------------------------------------------------------------------
    // 74 CC Parameters from midi-cc-map.json
    // -------------------------------------------------------------------------
    { id: 'mod_wheel', name: 'Mod Wheel', module: 'Performance', address: { kind: 'Cc', cc: 1 }, range: { min: 0.0, max: 127.0, step: 1.0, default_value: 0.0 }, curve: 'Linear', is_locked: false },
    { id: 'osc_1_range', name: 'Osc 1 Range', module: 'Oscillator 1', address: { kind: 'Cc', cc: 3 }, range: { min: 0.0, max: 3.0, step: 1.0, default_value: 1.0 }, curve: 'Linear', is_locked: false },
    { id: 'glide_time', name: 'Glide Time', module: 'Performance', address: { kind: 'Cc', cc: 5 }, range: { min: 0.0, max: 127.0, step: 1.0, default_value: 0.0 }, curve: 'Exponential', is_locked: false },
    { id: 'osc_1_mod_2_pitch', name: 'Osc 1 Mod 2 Pitch', module: 'Oscillator 1', address: { kind: 'Cc', cc: 9 }, range: { min: -64.0, max: 63.0, step: 1.0, default_value: 0.0 }, curve: 'Linear', is_locked: false },
    { id: 'osc_1_shape', name: 'Osc 1 Shape', module: 'Oscillator 1', address: { kind: 'Cc', cc: 12 }, range: { min: 0.0, max: 127.0, step: 1.0, default_value: 64.0 }, curve: 'Linear', is_locked: false },
    { id: 'osc_1_coarse', name: 'Osc 1 Coarse', module: 'Oscillator 1', address: { kind: 'Cc', cc: 14 }, range: { min: -12.0, max: 12.0, step: 1.0, default_value: 0.0 }, curve: 'Linear', is_locked: false },
    { id: 'osc_1_fine', name: 'Osc 1 Fine', module: 'Oscillator 1', address: { kind: 'Cc', cc: 15 }, range: { min: -100.0, max: 100.0, step: 1.0, default_value: 0.0 }, curve: 'Linear', is_locked: false },
    { id: 'osc_1_lfo_2_pitch', name: 'Osc 1 LFO 2 Pitch', module: 'Oscillator 1', address: { kind: 'Cc', cc: 16 }, range: { min: -64.0, max: 63.0, step: 1.0, default_value: 0.0 }, curve: 'Linear', is_locked: false },
    { id: 'osc_2_coarse', name: 'Osc 2 Coarse', module: 'Oscillator 2', address: { kind: 'Cc', cc: 17 }, range: { min: -12.0, max: 12.0, step: 1.0, default_value: 0.0 }, curve: 'Linear', is_locked: false },
    { id: 'osc_2_fine', name: 'Osc 2 Fine', module: 'Oscillator 2', address: { kind: 'Cc', cc: 18 }, range: { min: -100.0, max: 100.0, step: 1.0, default_value: 0.0 }, curve: 'Linear', is_locked: false },
    { id: 'osc_2_lfo_2_pitch', name: 'Osc 2 LFO 2 Pitch', module: 'Oscillator 2', address: { kind: 'Cc', cc: 19 }, range: { min: -64.0, max: 63.0, step: 1.0, default_value: 0.0 }, curve: 'Linear', is_locked: false },
    { id: 'osc_3_coarse', name: 'Osc 3 Coarse', module: 'Oscillator 3', address: { kind: 'Cc', cc: 20 }, range: { min: -12.0, max: 12.0, step: 1.0, default_value: 0.0 }, curve: 'Linear', is_locked: false },
    { id: 'osc_3_fine', name: 'Osc 3 Fine', module: 'Oscillator 3', address: { kind: 'Cc', cc: 21 }, range: { min: -100.0, max: 100.0, step: 1.0, default_value: 0.0 }, curve: 'Linear', is_locked: false },
    { id: 'osc_3_lfo_2_pitch', name: 'Osc 3 LFO 2 Pitch', module: 'Oscillator 3', address: { kind: 'Cc', cc: 22 }, range: { min: -64.0, max: 63.0, step: 1.0, default_value: 0.0 }, curve: 'Linear', is_locked: false },
    { id: 'osc_1_mix', name: 'Osc 1 Mix', module: 'Mixer', address: { kind: 'Cc', cc: 23 }, range: { min: 0.0, max: 127.0, step: 1.0, default_value: 127.0 }, curve: 'Linear', is_locked: false },
    { id: 'osc_2_mix', name: 'Osc 2 Mix', module: 'Mixer', address: { kind: 'Cc', cc: 24 }, range: { min: 0.0, max: 127.0, step: 1.0, default_value: 0.0 }, curve: 'Linear', is_locked: false },
    { id: 'osc_3_mix', name: 'Osc 3 Mix', module: 'Mixer', address: { kind: 'Cc', cc: 25 }, range: { min: 0.0, max: 127.0, step: 1.0, default_value: 0.0 }, curve: 'Linear', is_locked: false },
    { id: 'ring_mod_mix', name: 'Ring Mod Mix', module: 'Mixer', address: { kind: 'Cc', cc: 26 }, range: { min: 0.0, max: 127.0, step: 1.0, default_value: 0.0 }, curve: 'Linear', is_locked: false },
    { id: 'noise_mix', name: 'Noise Mix', module: 'Mixer', address: { kind: 'Cc', cc: 27 }, range: { min: 0.0, max: 127.0, step: 1.0, default_value: 0.0 }, curve: 'Linear', is_locked: false },
    { id: 'lfo_1_to_filter_depth', name: 'LFO 1 -> Filter Depth', module: 'Filter', address: { kind: 'Cc', cc: 28 }, range: { min: -64.0, max: 63.0, step: 1.0, default_value: 0.0 }, curve: 'Linear', is_locked: false },
    { id: 'filter_cutoff', name: 'Filter Cutoff', module: 'Filter', address: { kind: 'Cc', cc: 29 }, range: { min: 0.0, max: 127.0, step: 1.0, default_value: 127.0 }, curve: 'Exponential', is_locked: false },
    { id: 'lfo_1_rate', name: 'LFO 1 Rate', module: 'LFO 1', address: { kind: 'Cc', cc: 30 }, range: { min: 0.0, max: 127.0, step: 1.0, default_value: 64.0 }, curve: 'Linear', is_locked: false },
    { id: 'lfo_2_rate', name: 'LFO 2 Rate', module: 'LFO 2', address: { kind: 'Cc', cc: 31 }, range: { min: 0.0, max: 127.0, step: 1.0, default_value: 64.0 }, curve: 'Linear', is_locked: false },
    { id: 'osc_1_lfo_1_shape', name: 'Osc 1 LFO 1 Shape', module: 'Oscillator 1', address: { kind: 'Cc', cc: 33 }, range: { min: -64.0, max: 63.0, step: 1.0, default_value: 0.0 }, curve: 'Linear', is_locked: false },
    { id: 'osc_1_vsync', name: 'Osc 1 VSync', module: 'Oscillator 1', address: { kind: 'Cc', cc: 34 }, range: { min: 0.0, max: 127.0, step: 1.0, default_value: 0.0 }, curve: 'Linear', is_locked: false },
    { id: 'glide_on', name: 'Glide On', module: 'Performance', address: { kind: 'Cc', cc: 35 }, range: { min: 0.0, max: 1.0, step: 1.0, default_value: 0.0 }, curve: 'Linear', is_locked: false },
    { id: 'filter_post_drive', name: 'Filter Post Drive', module: 'Filter', address: { kind: 'Cc', cc: 36 }, range: { min: 0.0, max: 127.0, step: 1.0, default_value: 0.0 }, curve: 'Linear', is_locked: false },
    { id: 'osc_2_range', name: 'Osc 2 Range', module: 'Oscillator 2', address: { kind: 'Cc', cc: 37 }, range: { min: 0.0, max: 3.0, step: 1.0, default_value: 1.0 }, curve: 'Linear', is_locked: false },
    { id: 'osc_2_mod_2_pitch', name: 'Osc 2 Mod 2 Pitch', module: 'Oscillator 2', address: { kind: 'Cc', cc: 38 }, range: { min: -64.0, max: 63.0, step: 1.0, default_value: 0.0 }, curve: 'Linear', is_locked: false },
    { id: 'osc_2_shape', name: 'Osc 2 Shape', module: 'Oscillator 2', address: { kind: 'Cc', cc: 39 }, range: { min: 0.0, max: 127.0, step: 1.0, default_value: 64.0 }, curve: 'Linear', is_locked: false },
    { id: 'osc_2_mod_1_shape', name: 'Osc 2 Mod 1 Shape', module: 'Oscillator 2', address: { kind: 'Cc', cc: 40 }, range: { min: -64.0, max: 63.0, step: 1.0, default_value: 0.0 }, curve: 'Linear', is_locked: false },
    { id: 'osc_2_lfo_1_shape', name: 'Osc 2 LFO 1 Shape', module: 'Oscillator 2', address: { kind: 'Cc', cc: 41 }, range: { min: -64.0, max: 63.0, step: 1.0, default_value: 0.0 }, curve: 'Linear', is_locked: false },
    { id: 'osc_2_vsync', name: 'Osc 2 VSync', module: 'Oscillator 2', address: { kind: 'Cc', cc: 42 }, range: { min: 0.0, max: 127.0, step: 1.0, default_value: 0.0 }, curve: 'Linear', is_locked: false },
    { id: 'osc_3_mod_2_pitch', name: 'Osc 3 Mod 2 Pitch', module: 'Oscillator 3', address: { kind: 'Cc', cc: 43 }, range: { min: -64.0, max: 63.0, step: 1.0, default_value: 0.0 }, curve: 'Linear', is_locked: false },
    { id: 'osc_3_vsync', name: 'Osc 3 VSync', module: 'Oscillator 3', address: { kind: 'Cc', cc: 44 }, range: { min: 0.0, max: 127.0, step: 1.0, default_value: 0.0 }, curve: 'Linear', is_locked: false },
    { id: 'osc_3_range', name: 'Osc 3 Range', module: 'Oscillator 3', address: { kind: 'Cc', cc: 65 }, range: { min: 0.0, max: 3.0, step: 1.0, default_value: 1.0 }, curve: 'Linear', is_locked: false },
    { id: 'osc_3_shape', name: 'Osc 3 Shape', module: 'Oscillator 3', address: { kind: 'Cc', cc: 71 }, range: { min: 0.0, max: 127.0, step: 1.0, default_value: 64.0 }, curve: 'Linear', is_locked: false },
    { id: 'osc_3_mod_1_pitch', name: 'Osc 3 Mod 1 Pitch', module: 'Oscillator 3', address: { kind: 'Cc', cc: 72 }, range: { min: -64.0, max: 63.0, step: 1.0, default_value: 0.0 }, curve: 'Linear', is_locked: false },
    { id: 'osc_3_lfo_1_shape', name: 'Osc 3 LFO 1 Shape', module: 'Oscillator 3', address: { kind: 'Cc', cc: 73 }, range: { min: -64.0, max: 63.0, step: 1.0, default_value: 0.0 }, curve: 'Linear', is_locked: false },
    { id: 'filter_track', name: 'Filter Track', module: 'Filter', address: { kind: 'Cc', cc: 75 }, range: { min: 0.0, max: 127.0, step: 1.0, default_value: 127.0 }, curve: 'Linear', is_locked: false },
    { id: 'osc_3_to_filter', name: 'Osc 3 -> Filter', module: 'Filter', address: { kind: 'Cc', cc: 76 }, range: { min: 0.0, max: 127.0, step: 1.0, default_value: 0.0 }, curve: 'Linear', is_locked: false },
    { id: 'mod_1_to_filter', name: 'Mod 1 -> Filter', module: 'Filter', address: { kind: 'Cc', cc: 78 }, range: { min: -64.0, max: 63.0, step: 1.0, default_value: 0.0 }, curve: 'Linear', is_locked: false },
    { id: 'filter_resonance', name: 'Filter Resonance', module: 'Filter', address: { kind: 'Cc', cc: 79 }, range: { min: 0.0, max: 127.0, step: 1.0, default_value: 0.0 }, curve: 'Linear', is_locked: false },
    { id: 'filter_overdrive', name: 'Filter Overdrive', module: 'Filter', address: { kind: 'Cc', cc: 80 }, range: { min: 0.0, max: 127.0, step: 1.0, default_value: 0.0 }, curve: 'Linear', is_locked: false },
    { id: 'lfo_1_sync_rate', name: 'LFO 1 Sync Rate', module: 'LFO 1', address: { kind: 'Cc', cc: 81 }, range: { min: 0.0, max: 127.0, step: 1.0, default_value: 0.0 }, curve: 'Linear', is_locked: false },
    { id: 'lfo_1_fade', name: 'LFO 1 Fade', module: 'LFO 1', address: { kind: 'Cc', cc: 82 }, range: { min: 0.0, max: 127.0, step: 1.0, default_value: 0.0 }, curve: 'Linear', is_locked: false },
    { id: 'lfo_2_range', name: 'LFO 2 Range', module: 'LFO 2', address: { kind: 'Cc', cc: 83 }, range: { min: 0.0, max: 1.0, step: 1.0, default_value: 0.0 }, curve: 'Linear', is_locked: false },
    { id: 'lfo_2_sync_rate', name: 'LFO 2 Sync Rate', module: 'LFO 2', address: { kind: 'Cc', cc: 84 }, range: { min: 0.0, max: 127.0, step: 1.0, default_value: 0.0 }, curve: 'Linear', is_locked: false },
    { id: 'lfo_2_fade', name: 'LFO 2 Fade', module: 'LFO 2', address: { kind: 'Cc', cc: 85 }, range: { min: 0.0, max: 127.0, step: 1.0, default_value: 0.0 }, curve: 'Linear', is_locked: false },
    { id: 'amp_envelope_attack', name: 'Amp Envelope Attack', module: 'Amp Envelope', address: { kind: 'Cc', cc: 86 }, range: { min: 0.0, max: 127.0, step: 1.0, default_value: 0.0 }, curve: 'Exponential', is_locked: false },
    { id: 'amp_envelope_decay', name: 'Amp Envelope Decay', module: 'Amp Envelope', address: { kind: 'Cc', cc: 87 }, range: { min: 0.0, max: 127.0, step: 1.0, default_value: 64.0 }, curve: 'Exponential', is_locked: false },
    { id: 'amp_envelope_sustain', name: 'Amp Envelope Sustain', module: 'Amp Envelope', address: { kind: 'Cc', cc: 88 }, range: { min: 0.0, max: 127.0, step: 1.0, default_value: 127.0 }, curve: 'Linear', is_locked: false },
    { id: 'amp_envelope_release', name: 'Amp Envelope Release', module: 'Amp Envelope', address: { kind: 'Cc', cc: 89 }, range: { min: 0.0, max: 127.0, step: 1.0, default_value: 32.0 }, curve: 'Exponential', is_locked: false },
    { id: 'mod_envelope_1_attack', name: 'Mod Envelope 1 Attack', module: 'Mod Envelope 1', address: { kind: 'Cc', cc: 90 }, range: { min: 0.0, max: 127.0, step: 1.0, default_value: 0.0 }, curve: 'Exponential', is_locked: false },
    { id: 'mod_envelope_1_decay', name: 'Mod Envelope 1 Decay', module: 'Mod Envelope 1', address: { kind: 'Cc', cc: 91 }, range: { min: 0.0, max: 127.0, step: 1.0, default_value: 64.0 }, curve: 'Exponential', is_locked: false },
    { id: 'mod_envelope_1_sustain', name: 'Mod Envelope 1 Sustain', module: 'Mod Envelope 1', address: { kind: 'Cc', cc: 92 }, range: { min: 0.0, max: 127.0, step: 1.0, default_value: 0.0 }, curve: 'Linear', is_locked: false },
    { id: 'mod_envelope_1_release', name: 'Mod Envelope 1 Release', module: 'Mod Envelope 1', address: { kind: 'Cc', cc: 93 }, range: { min: 0.0, max: 127.0, step: 1.0, default_value: 32.0 }, curve: 'Exponential', is_locked: false },
    { id: 'mod_envelope_2_attack', name: 'Mod Envelope 2 Attack', module: 'Mod Envelope 2', address: { kind: 'Cc', cc: 94 }, range: { min: 0.0, max: 127.0, step: 1.0, default_value: 0.0 }, curve: 'Exponential', is_locked: false },
    { id: 'mod_envelope_2_decay', name: 'Mod Envelope 2 Decay', module: 'Mod Envelope 2', address: { kind: 'Cc', cc: 95 }, range: { min: 0.0, max: 127.0, step: 1.0, default_value: 64.0 }, curve: 'Exponential', is_locked: false },
    { id: 'mod_envelope_2_release', name: 'Mod Envelope 2 Release', module: 'Mod Envelope 2', address: { kind: 'Cc', cc: 103 }, range: { min: 0.0, max: 127.0, step: 1.0, default_value: 32.0 }, curve: 'Exponential', is_locked: false },
    { id: 'distortion_level', name: 'Distortion Level', module: 'Effects', address: { kind: 'Cc', cc: 104 }, range: { min: 0.0, max: 127.0, step: 1.0, default_value: 0.0 }, curve: 'Linear', is_locked: false },
    { id: 'chorus_level', name: 'Chorus Level', module: 'Effects', address: { kind: 'Cc', cc: 105 }, range: { min: 0.0, max: 127.0, step: 1.0, default_value: 0.0 }, curve: 'Linear', is_locked: false },
    { id: 'chorus_feedback', name: 'Chorus Feedback', module: 'Effects', address: { kind: 'Cc', cc: 107 }, range: { min: 0.0, max: 127.0, step: 1.0, default_value: 0.0 }, curve: 'Linear', is_locked: false },
    { id: 'delay_level', name: 'Delay Level', module: 'Effects', address: { kind: 'Cc', cc: 108 }, range: { min: 0.0, max: 127.0, step: 1.0, default_value: 0.0 }, curve: 'Linear', is_locked: false },
    { id: 'delay_time', name: 'Delay Time', module: 'Effects', address: { kind: 'Cc', cc: 109 }, range: { min: 0.0, max: 127.0, step: 1.0, default_value: 64.0 }, curve: 'Linear', is_locked: false },
    { id: 'delay_feedback', name: 'Delay Feedback', module: 'Effects', address: { kind: 'Cc', cc: 110 }, range: { min: 0.0, max: 127.0, step: 1.0, default_value: 0.0 }, curve: 'Linear', is_locked: false },
    { id: 'reverb_level', name: 'Reverb Level', module: 'Effects', address: { kind: 'Cc', cc: 112 }, range: { min: 0.0, max: 127.0, step: 1.0, default_value: 0.0 }, curve: 'Linear', is_locked: false },
    { id: 'reverb_time', name: 'Reverb Time', module: 'Effects', address: { kind: 'Cc', cc: 113 }, range: { min: 0.0, max: 127.0, step: 1.0, default_value: 64.0 }, curve: 'Linear', is_locked: false },
    { id: 'animate_1_hold', name: 'Animate 1 Hold', module: 'Performance', address: { kind: 'Cc', cc: 114 }, range: { min: 0.0, max: 127.0, step: 1.0, default_value: 0.0 }, curve: 'Linear', is_locked: false },
    { id: 'animate_2_hold', name: 'Animate 2 Hold', module: 'Performance', address: { kind: 'Cc', cc: 115 }, range: { min: 0.0, max: 127.0, step: 1.0, default_value: 0.0 }, curve: 'Linear', is_locked: false },
    { id: 'arp_clock_gate', name: 'Arp Clock Gate', module: 'Arpeggiator', address: { kind: 'Cc', cc: 116 }, range: { min: 0.0, max: 127.0, step: 1.0, default_value: 64.0 }, curve: 'Linear', is_locked: false },
    { id: 'mod_envelope_2_sustain', name: 'Mod Envelope 2 Sustain', module: 'Mod Envelope 2', address: { kind: 'Cc', cc: 117 }, range: { min: 0.0, max: 127.0, step: 1.0, default_value: 0.0 }, curve: 'Linear', is_locked: false },
    { id: 'chorus_rate', name: 'Chorus Rate', module: 'Effects', address: { kind: 'Cc', cc: 118 }, range: { min: 0.0, max: 127.0, step: 1.0, default_value: 64.0 }, curve: 'Linear', is_locked: false },
    { id: 'osc_1_mod_1_shape', name: 'Osc 1 Mod 1 Shape', module: 'Oscillator 1', address: { kind: 'Cc', cc: 119 }, range: { min: -64.0, max: 63.0, step: 1.0, default_value: 0.0 }, curve: 'Linear', is_locked: false },

    // -------------------------------------------------------------------------
    // 32 Bipolar Dual-CC Pairs (MSB 0..31, LSB 32..63)
    // -------------------------------------------------------------------------
    ...Array.from({ length: 32 }, (_, i) => ({
      id: `dual_cc_${i}_${i + 32}`,
      name: i === 29 ? 'Filter Cutoff (14-bit Dual-CC)' : `Dual-CC MSB ${i} / LSB ${i + 32}`,
      module: 'Dual-CC Pairs',
      address: { kind: 'Cc14Bit', msb: i, lsb: i + 32 },
      range: { min: 0.0, max: 16383.0, step: 1.0, default_value: 8192.0 },
      curve: i === 29 ? 'Exponential' : 'Linear',
      is_locked: false,
    })),

    // -------------------------------------------------------------------------
    // 14-bit NRPN Descriptors: Oscillators (17 Wavetables), Filter, LFOs, Arp
    // -------------------------------------------------------------------------
    { id: 'osc_1_waveform', name: 'Osc 1 Waveform & Wavetable', module: 'Oscillator 1', address: { kind: 'Nrpn', msb: 0, lsb: 64 }, range: { min: 0.0, max: 20.0, step: 1.0, default_value: 2.0 }, curve: 'Linear', is_locked: false },
    { id: 'osc_1_density', name: 'Osc 1 Density', module: 'Oscillator 1', address: { kind: 'Nrpn', msb: 0, lsb: 66 }, range: { min: 0.0, max: 127.0, step: 1.0, default_value: 0.0 }, curve: 'Linear', is_locked: false },
    { id: 'osc_1_density_detune', name: 'Osc 1 Density Detune', module: 'Oscillator 1', address: { kind: 'Nrpn', msb: 0, lsb: 67 }, range: { min: 0.0, max: 127.0, step: 1.0, default_value: 0.0 }, curve: 'Linear', is_locked: false },
    { id: 'osc_2_waveform', name: 'Osc 2 Waveform & Wavetable', module: 'Oscillator 2', address: { kind: 'Nrpn', msb: 0, lsb: 80 }, range: { min: 0.0, max: 20.0, step: 1.0, default_value: 3.0 }, curve: 'Linear', is_locked: false },
    { id: 'osc_2_density', name: 'Osc 2 Density', module: 'Oscillator 2', address: { kind: 'Nrpn', msb: 0, lsb: 82 }, range: { min: 0.0, max: 127.0, step: 1.0, default_value: 0.0 }, curve: 'Linear', is_locked: false },
    { id: 'osc_2_density_detune', name: 'Osc 2 Density Detune', module: 'Oscillator 2', address: { kind: 'Nrpn', msb: 0, lsb: 83 }, range: { min: 0.0, max: 127.0, step: 1.0, default_value: 0.0 }, curve: 'Linear', is_locked: false },
    { id: 'osc_3_waveform', name: 'Osc 3 Waveform & Wavetable', module: 'Oscillator 3', address: { kind: 'Nrpn', msb: 0, lsb: 96 }, range: { min: 0.0, max: 20.0, step: 1.0, default_value: 0.0 }, curve: 'Linear', is_locked: false },
    { id: 'osc_3_density', name: 'Osc 3 Density', module: 'Oscillator 3', address: { kind: 'Nrpn', msb: 0, lsb: 98 }, range: { min: 0.0, max: 127.0, step: 1.0, default_value: 0.0 }, curve: 'Linear', is_locked: false },
    { id: 'osc_3_density_detune', name: 'Osc 3 Density Detune', module: 'Oscillator 3', address: { kind: 'Nrpn', msb: 0, lsb: 99 }, range: { min: 0.0, max: 127.0, step: 1.0, default_value: 0.0 }, curve: 'Linear', is_locked: false },

    { id: 'filter_type', name: 'Filter Type', module: 'Filter', address: { kind: 'Nrpn', msb: 0, lsb: 48 }, range: { min: 0.0, max: 2.0, step: 1.0, default_value: 0.0 }, curve: 'Linear', is_locked: false },
    { id: 'filter_slope', name: 'Filter Slope', module: 'Filter', address: { kind: 'Nrpn', msb: 0, lsb: 49 }, range: { min: 0.0, max: 1.0, step: 1.0, default_value: 1.0 }, curve: 'Linear', is_locked: false },
    { id: 'filter_cutoff_14bit_nrpn', name: 'Filter Cutoff 14-bit NRPN', module: 'Filter', address: { kind: 'Nrpn', msb: 0, lsb: 50 }, range: { min: 0.0, max: 16383.0, step: 1.0, default_value: 16383.0 }, curve: 'Exponential', is_locked: false },
    { id: 'filter_resonance_nrpn', name: 'Filter Resonance NRPN', module: 'Filter', address: { kind: 'Nrpn', msb: 0, lsb: 51 }, range: { min: 0.0, max: 127.0, step: 1.0, default_value: 0.0 }, curve: 'Linear', is_locked: false },

    { id: 'lfo_1_waveform', name: 'LFO 1 Waveform', module: 'LFO 1', address: { kind: 'Nrpn', msb: 0, lsb: 110 }, range: { min: 0.0, max: 3.0, step: 1.0, default_value: 1.0 }, curve: 'Linear', is_locked: false },
    { id: 'lfo_1_slew', name: 'LFO 1 Slew', module: 'LFO 1', address: { kind: 'Nrpn', msb: 0, lsb: 111 }, range: { min: 0.0, max: 127.0, step: 1.0, default_value: 0.0 }, curve: 'Linear', is_locked: false },
    { id: 'lfo_2_waveform', name: 'LFO 2 Waveform', module: 'LFO 2', address: { kind: 'Nrpn', msb: 0, lsb: 112 }, range: { min: 0.0, max: 3.0, step: 1.0, default_value: 0.0 }, curve: 'Linear', is_locked: false },
    { id: 'lfo_2_slew', name: 'LFO 2 Slew', module: 'LFO 2', address: { kind: 'Nrpn', msb: 0, lsb: 113 }, range: { min: 0.0, max: 127.0, step: 1.0, default_value: 0.0 }, curve: 'Linear', is_locked: false },

    { id: 'arp_mode', name: 'Arp Mode', module: 'Arpeggiator', address: { kind: 'Nrpn', msb: 0, lsb: 120 }, range: { min: 0.0, max: 6.0, step: 1.0, default_value: 0.0 }, curve: 'Linear', is_locked: false },
    { id: 'arp_division', name: 'Arp Division', module: 'Arpeggiator', address: { kind: 'Nrpn', msb: 0, lsb: 121 }, range: { min: 0.0, max: 15.0, step: 1.0, default_value: 3.0 }, curve: 'Linear', is_locked: false },
    { id: 'arp_octaves', name: 'Arp Octaves', module: 'Arpeggiator', address: { kind: 'Nrpn', msb: 0, lsb: 122 }, range: { min: 1.0, max: 4.0, step: 1.0, default_value: 1.0 }, curve: 'Linear', is_locked: false },

    // -------------------------------------------------------------------------
    // 16 Modulation Matrix Slots (4 NRPN parameters each = 64 parameters)
    // -------------------------------------------------------------------------
    ...Array.from({ length: 16 }, (_, idx) => {
      const slot = idx + 1;
      const baseNrpn = idx * 4;
      return [
        { id: `mod_matrix_slot_${slot}_source_1`, name: `Mod Matrix Slot ${slot} Source 1`, module: 'Modulation Matrix', address: { kind: 'Nrpn', msb: 1, lsb: baseNrpn }, range: { min: 0.0, max: 17.0, step: 1.0, default_value: 0.0 }, curve: 'Linear', is_locked: false },
        { id: `mod_matrix_slot_${slot}_source_2`, name: `Mod Matrix Slot ${slot} Source 2`, module: 'Modulation Matrix', address: { kind: 'Nrpn', msb: 1, lsb: baseNrpn + 1 }, range: { min: 0.0, max: 17.0, step: 1.0, default_value: 0.0 }, curve: 'Linear', is_locked: false },
        { id: `mod_matrix_slot_${slot}_depth`, name: `Mod Matrix Slot ${slot} Depth`, module: 'Modulation Matrix', address: { kind: 'Nrpn', msb: 1, lsb: baseNrpn + 2 }, range: { min: -64.0, max: 63.0, step: 1.0, default_value: 0.0 }, curve: 'Linear', is_locked: false },
        { id: `mod_matrix_slot_${slot}_destination`, name: `Mod Matrix Slot ${slot} Destination`, module: 'Modulation Matrix', address: { kind: 'Nrpn', msb: 1, lsb: baseNrpn + 3 }, range: { min: 0.0, max: 36.0, step: 1.0, default_value: 0.0 }, curve: 'Linear', is_locked: false },
      ];
    }).flat(),
  ],
  controller_controls: [
    { id: 'animate_1', name: 'Animate 1 Button', kind: 'Button', cc: 114, channel: 0, has_led: true, led_cc: 114, track_index: null, group: 'Performance' },
    { id: 'animate_2', name: 'Animate 2 Button', kind: 'Button', cc: 115, channel: 0, has_led: true, led_cc: 115, track_index: null, group: 'Performance' },
    { id: 'filter_cutoff_knob', name: 'Filter Frequency Knob', kind: 'Knob', cc: 29, channel: 0, has_led: false, led_cc: null, track_index: null, group: 'Filter' },
    { id: 'filter_resonance_knob', name: 'Filter Resonance Knob', kind: 'Knob', cc: 79, channel: 0, has_led: false, led_cc: null, track_index: null, group: 'Filter' },
    { id: 'filter_overdrive_knob', name: 'Filter Overdrive Knob', kind: 'Knob', cc: 80, channel: 0, has_led: false, led_cc: null, track_index: null, group: 'Filter' },
    { id: 'distortion_knob', name: 'Distortion Level Knob', kind: 'Knob', cc: 104, channel: 0, has_led: false, led_cc: null, track_index: null, group: 'Effects' },
    { id: 'delay_knob', name: 'Delay Level Knob', kind: 'Knob', cc: 108, channel: 0, has_led: false, led_cc: null, track_index: null, group: 'Effects' },
    { id: 'reverb_knob', name: 'Reverb Level Knob', kind: 'Knob', cc: 112, channel: 0, has_led: false, led_cc: null, track_index: null, group: 'Effects' },
    { id: 'chorus_knob', name: 'Chorus Level Knob', kind: 'Knob', cc: 105, channel: 0, has_led: false, led_cc: null, track_index: null, group: 'Effects' },
    { id: 'osc1_mix_knob', name: 'Osc 1 Mix Knob', kind: 'Knob', cc: 23, channel: 0, has_led: false, led_cc: null, track_index: null, group: 'Mixer' },
    { id: 'osc2_mix_knob', name: 'Osc 2 Mix Knob', kind: 'Knob', cc: 24, channel: 0, has_led: false, led_cc: null, track_index: null, group: 'Mixer' },
    { id: 'osc3_mix_knob', name: 'Osc 3 Mix Knob', kind: 'Knob', cc: 25, channel: 0, has_led: false, led_cc: null, track_index: null, group: 'Mixer' },
    { id: 'amp_attack_slider', name: 'Amp Attack Slider', kind: 'Fader', cc: 86, channel: 0, has_led: false, led_cc: null, track_index: null, group: 'Amp Envelope' },
    { id: 'amp_decay_slider', name: 'Amp Decay Slider', kind: 'Fader', cc: 87, channel: 0, has_led: false, led_cc: null, track_index: null, group: 'Amp Envelope' },
    { id: 'amp_sustain_slider', name: 'Amp Sustain Slider', kind: 'Fader', cc: 88, channel: 0, has_led: false, led_cc: null, track_index: null, group: 'Amp Envelope' },
    { id: 'amp_release_slider', name: 'Amp Release Slider', kind: 'Fader', cc: 89, channel: 0, has_led: false, led_cc: null, track_index: null, group: 'Amp Envelope' },
    { id: 'lfo1_rate_knob', name: 'LFO 1 Rate Knob', kind: 'Knob', cc: 30, channel: 0, has_led: false, led_cc: null, track_index: null, group: 'LFO 1' },
    { id: 'lfo2_rate_knob', name: 'LFO 2 Rate Knob', kind: 'Knob', cc: 31, channel: 0, has_led: false, led_cc: null, track_index: null, group: 'LFO 2' },
  ],
  tags: ['hybrid', 'polyphonic', 'fpga_nco', 'analogue_filter', 'novation_peak', '8_voice'],
});

// =============================================================================
// Canonical Themes across 4 Synthesizer Projects
// =============================================================================

export const ASM_HYDRASYNTH_THEME = Object.freeze({
  preference: 'Slate',
  two_bowls_bg: 'SlateStudio',
  accent: 'HydraOrange',
  overrides: { accent: '#FF7700', panel: '#1E2228', led_readout: '#FF8800' },
  scale: 1.0,
  true_black_and_white: false,
});

export const NKEDITOR_THEME = Object.freeze({
  preference: 'Default',
  two_bowls_bg: 'ZenMonochrome',
  accent: 'MinimalWhite',
  overrides: { accent: '#F0F0F0', panel: '#141414', led_readout: '#FFFFFF' },
  scale: 1.0,
  true_black_and_white: false,
});

export const BS2_TD3_THEME = Object.freeze({
  preference: 'Dark',
  two_bowls_bg: 'DeepNight',
  accent: 'AcidGreen',
  overrides: { accent: '#39FF14', panel: '#0F1210', led_readout: '#39FF14' },
  scale: 1.0,
  true_black_and_white: false,
});

export const NORD_DRUM_2_THEME = Object.freeze({
  preference: 'Nord',
  two_bowls_bg: 'WarmPaper',
  accent: 'NordCrimson',
  overrides: { accent: '#E52521', panel: '#1C1616', led_readout: '#FFB300' },
  scale: 1.0,
  true_black_and_white: false,
});

export const NOVATION_PEAK_THEME = Object.freeze({
  preference: 'Peak',
  two_bowls_bg: 'OxfordSlate',
  accent: 'NovationCyan',
  overrides: { accent: '#00D2BE', panel: '#181C20', led_readout: '#00E5FF' },
  scale: 1.0,
  true_black_and_white: false,
});

// =============================================================================
// Unified 5-Synthesizer Ecosystem Schema Bundle
// =============================================================================

export const UNIFIED_ECOSYSTEM_SCHEMA_BUNDLE = Object.freeze({
  version: '1.0.0',
  ecosystem_name: 'Five-Synth Ecosystem (Hydra, nkEditor, BS2/TD-3, ND2, Peak)',
  synths: Object.freeze([
    ASM_HYDRASYNTH_PROFILE,
    KORG_NANOKONTROL2_PROFILE,
    NOVATION_BS2_PROFILE,
    BEHRINGER_TD3_PROFILE,
    NORD_DRUM_2_PROFILE,
    NOVATION_PEAK_PROFILE,
  ]),
});

export class UnifiedEcosystemSchemaBundle {
  constructor(data = UNIFIED_ECOSYSTEM_SCHEMA_BUNDLE) {
    this.version = data.version;
    this.ecosystem_name = data.ecosystem_name;
    this.synths = data.synths;
  }

  getProfile(id) {
    return this.synths.find(s => s.id === id);
  }

  synthesizerProfiles() {
    return this.synths.filter(s => s.category === 'synthesizer');
  }

  controllerProfiles() {
    return this.synths.filter(s => s.category === 'controller_surface');
  }

  get length() {
    return this.synths.length;
  }
}

export const NovationPeakProfile = Object.freeze({
  WAVETABLES: Object.freeze([
    'Sine-Tri', 'Sine-Saw', 'Classic', 'Saw-Pulse', 'Harmonic 1', 'Harmonic 2',
    'Chebyshev', 'Double', 'Metal', 'Resonant 1', 'Resonant 2', 'Formant 1',
    'Formant 2', 'Vocal', 'Bell', 'Organ', 'Digi',
  ]),
  DUAL_CC_PAIRS: Object.freeze(
    Array.from({ length: 32 }, (_, i) => Object.freeze([i, i + 32]))
  ),
  CC_MAP_COUNT: 74,
  MOD_MATRIX_SLOTS: 16,
  profile: NOVATION_PEAK_PROFILE,
  safeSweepRange(name) {
    const n = String(name || '').toLowerCase();
    if (n.includes('resonance') || n.includes('reso')) return { lo: 0, hi: 62 };
    if (n.includes('distortion')) return { lo: 0, hi: 48 };
    if (n.includes('overdrive') || n.includes('drive')) return { lo: 0, hi: 50 };
    if (n.includes('feedback')) return { lo: 0, hi: 70 };
    if (n.includes('reverb') || n.includes('delay') || n.includes('chorus') || n.includes('fx')) {
      return { lo: 0, hi: 90 };
    }
    return { lo: 0, hi: 127 };
  },
});

// =============================================================================
// Validator Functions
// =============================================================================

/**
 * Validates a SynthPatch object against the patch schema.
 * @param {Object} patch
 * @returns {boolean}
 */
export function validateSynthPatch(patch) {
  if (!patch || typeof patch !== 'object') {
    throw new TypeError('SynthPatch must be a valid non-null object');
  }
  if (!patch.meta || typeof patch.meta !== 'object') {
    throw new TypeError('SynthPatch missing required meta object');
  }
  if (!patch.meta.id || typeof patch.meta.id !== 'string') {
    throw new TypeError('SynthPatch meta.id must be a string');
  }
  if (!patch.meta.name || typeof patch.meta.name !== 'string') {
    throw new TypeError('SynthPatch meta.name must be a string');
  }
  if (!patch.parameters || typeof patch.parameters !== 'object') {
    throw new TypeError('SynthPatch missing required parameters object');
  }
  return true;
}

/**
 * Validates a DeviceSynthesizerProfile object against the profile schema.
 * @param {Object} profile
 * @returns {boolean}
 */
export function validateDeviceProfile(profile) {
  if (!profile || typeof profile !== 'object') {
    throw new TypeError('DeviceProfile must be a valid non-null object');
  }
  const required = ['id', 'name', 'manufacturer', 'version', 'category', 'default_midi_channel', 'parameters'];
  for (const field of required) {
    if (profile[field] === undefined) {
      throw new TypeError(`DeviceProfile missing required field: ${field}`);
    }
  }
  if (!Array.isArray(profile.parameters)) {
    throw new TypeError('DeviceProfile parameters must be an array');
  }
  return true;
}

/**
 * Validates a ControllerSurfaceMapping object against the mapping schema.
 * @param {Object} mapping
 * @returns {boolean}
 */
export function validateControllerMapping(mapping) {
  if (!mapping || typeof mapping !== 'object') {
    throw new TypeError('ControllerSurfaceMapping must be a valid non-null object');
  }
  if (!mapping.surface_name || typeof mapping.surface_name !== 'string') {
    throw new TypeError('ControllerSurfaceMapping missing surface_name');
  }
  if (!Array.isArray(mapping.banks)) {
    throw new TypeError('ControllerSurfaceMapping banks must be an array');
  }
  return true;
}

/**
 * Validates a ThemeConfig object against the theme schema.
 * @param {Object} theme
 * @returns {boolean}
 */
export function validateThemeConfig(theme) {
  if (!theme || typeof theme !== 'object') {
    throw new TypeError('ThemeConfig must be a valid non-null object');
  }
  const required = ['preference', 'accent', 'overrides', 'scale', 'true_black_and_white'];
  for (const field of required) {
    if (theme[field] === undefined) {
      throw new TypeError(`ThemeConfig missing required field: ${field}`);
    }
  }
  return true;
}

/**
 * Exports all schemas, profiles, and themes across the 4 synthesizer projects.
 * @param {string} outputDir
 */
export function exportAllSchemas(outputDir) {
  fs.mkdirSync(outputDir, { recursive: true });

  fs.writeFileSync(path.join(outputDir, 'synth-patch.schema.json'), JSON.stringify(SYNTH_PATCH_SCHEMA, null, 2));
  fs.writeFileSync(path.join(outputDir, 'device-profile.schema.json'), JSON.stringify(DEVICE_PROFILE_SCHEMA, null, 2));
  fs.writeFileSync(path.join(outputDir, 'controller-mapping.schema.json'), JSON.stringify(CONTROLLER_MAPPING_SCHEMA, null, 2));
  fs.writeFileSync(path.join(outputDir, 'theme-config.schema.json'), JSON.stringify(THEME_CONFIG_SCHEMA, null, 2));

  fs.writeFileSync(path.join(outputDir, 'asm-hydrasynth.profile.json'), JSON.stringify(ASM_HYDRASYNTH_PROFILE, null, 2));
  fs.writeFileSync(path.join(outputDir, 'korg-nanokontrol2.profile.json'), JSON.stringify(KORG_NANOKONTROL2_PROFILE, null, 2));
  fs.writeFileSync(path.join(outputDir, 'novation-bass-station-2.profile.json'), JSON.stringify(NOVATION_BS2_PROFILE, null, 2));
  fs.writeFileSync(path.join(outputDir, 'behringer-td3-md.profile.json'), JSON.stringify(BEHRINGER_TD3_PROFILE, null, 2));
  fs.writeFileSync(path.join(outputDir, 'clavia-nord-drum-2.profile.json'), JSON.stringify(NORD_DRUM_2_PROFILE, null, 2));
  fs.writeFileSync(path.join(outputDir, 'novation-peak.profile.json'), JSON.stringify(NOVATION_PEAK_PROFILE, null, 2));
  fs.writeFileSync(path.join(outputDir, 'unified-ecosystem-bundle.json'), JSON.stringify(UNIFIED_ECOSYSTEM_SCHEMA_BUNDLE, null, 2));

  fs.writeFileSync(path.join(outputDir, 'asm-hydrasynth.theme.json'), JSON.stringify(ASM_HYDRASYNTH_THEME, null, 2));
  fs.writeFileSync(path.join(outputDir, 'nkeditor.theme.json'), JSON.stringify(NKEDITOR_THEME, null, 2));
  fs.writeFileSync(path.join(outputDir, 'bs2-td3.theme.json'), JSON.stringify(BS2_TD3_THEME, null, 2));
  fs.writeFileSync(path.join(outputDir, 'clavia-nord-drum-2.theme.json'), JSON.stringify(NORD_DRUM_2_THEME, null, 2));
  fs.writeFileSync(path.join(outputDir, 'novation-peak.theme.json'), JSON.stringify(NOVATION_PEAK_THEME, null, 2));
}
