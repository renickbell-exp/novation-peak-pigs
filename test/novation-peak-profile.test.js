// SPDX-License-Identifier: GPL-3.0-or-later
const test = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');
const peak = require('../js/konduktiva-peak/src/index.js');
const sdk = require('../js/shared-synth-sdk/index.js');

test('NovationPeakProfile covers all 74 CCs from midi-cc-map.json', () => {
  const ccMap = peak.loadCcMap();
  assert.equal(ccMap.cc_map.length, 74);

  const profile = sdk.NOVATION_PEAK_PROFILE;
  assert.ok(profile);
  assert.equal(profile.id, 'novation_peak');
  assert.equal(profile.name, 'Novation Peak');
  assert.equal(profile.manufacturer, 'Novation');
  assert.equal(profile.category, 'synthesizer');
  assert.equal(profile.default_midi_channel, 0);

  // Check every CC from midi-cc-map.json is present in profile parameters
  for (const entry of ccMap.cc_map) {
    const found = profile.parameters.find(p => p.address && p.address.kind === 'Cc' && p.address.cc === entry.cc);
    assert.ok(found, `CC ${entry.cc} (${entry.param}) must be present in profile`);
  }
});

test('NovationPeakProfile covers 32 bipolar dual-CC pairs', () => {
  const profile = sdk.NOVATION_PEAK_PROFILE;
  assert.equal(sdk.NovationPeakProfile.DUAL_CC_PAIRS.length, 32);

  // Check all 32 dual pairs (MSB 0..31, LSB 32..63)
  for (let i = 0; i < 32; i++) {
    const msb = i;
    const lsb = i + 32;
    const pair = sdk.NovationPeakProfile.DUAL_CC_PAIRS[i];
    assert.deepEqual(pair, [msb, lsb]);

    const dualParam = profile.parameters.find(
      p => p.address && p.address.kind === 'Cc14Bit' && p.address.msb === msb && p.address.lsb === lsb
    );
    assert.ok(dualParam, `Dual-CC pair (${msb}, ${lsb}) must be present`);
  }
});

test('NovationPeakProfile covers 14-bit NRPNs, 17 wavetables, filter pre/post drive, and 16 mod matrix slots', () => {
  const profile = sdk.NOVATION_PEAK_PROFILE;

  // 17 Wavetables
  assert.equal(sdk.NovationPeakProfile.WAVETABLES.length, 17);
  assert.equal(sdk.NovationPeakProfile.WAVETABLES[0], 'Sine-Tri');
  assert.equal(sdk.NovationPeakProfile.WAVETABLES[16], 'Digi');

  // Oscillators 1..3 Wavetable & Density NRPNs
  assert.ok(profile.parameters.find(p => p.id === 'osc_1_waveform' && p.address.kind === 'Nrpn' && p.address.lsb === 64));
  assert.ok(profile.parameters.find(p => p.id === 'osc_2_waveform' && p.address.kind === 'Nrpn' && p.address.lsb === 80));
  assert.ok(profile.parameters.find(p => p.id === 'osc_3_waveform' && p.address.kind === 'Nrpn' && p.address.lsb === 96));

  // Filter Type & Slope NRPNs + Pre/Post Drive CCs
  assert.ok(profile.parameters.find(p => p.id === 'filter_type' && p.address.kind === 'Nrpn' && p.address.lsb === 48));
  assert.ok(profile.parameters.find(p => p.id === 'filter_slope' && p.address.kind === 'Nrpn' && p.address.lsb === 49));
  assert.ok(profile.parameters.find(p => p.id === 'filter_overdrive' && p.address.cc === 80)); // Pre-filter drive
  assert.ok(profile.parameters.find(p => p.id === 'filter_post_drive' && p.address.cc === 36)); // Post-filter drive

  // 3 Envelopes
  assert.ok(profile.parameters.find(p => p.id === 'amp_envelope_attack' && p.address.cc === 86));
  assert.ok(profile.parameters.find(p => p.id === 'mod_envelope_1_attack' && p.address.cc === 90));
  assert.ok(profile.parameters.find(p => p.id === 'mod_envelope_2_attack' && p.address.cc === 94));
  assert.ok(profile.parameters.find(p => p.id === 'mod_envelope_2_sustain' && p.address.cc === 117));
  assert.ok(profile.parameters.find(p => p.id === 'mod_envelope_2_release' && p.address.cc === 103));

  // 2 LFOs
  assert.ok(profile.parameters.find(p => p.id === 'lfo_1_rate' && p.address.cc === 30));
  assert.ok(profile.parameters.find(p => p.id === 'lfo_1_sync_rate' && p.address.cc === 81));
  assert.ok(profile.parameters.find(p => p.id === 'lfo_2_rate' && p.address.cc === 31));
  assert.ok(profile.parameters.find(p => p.id === 'lfo_2_sync_rate' && p.address.cc === 84));

  // 16 Modulation Matrix Slots (64 parameters)
  assert.equal(sdk.NovationPeakProfile.MOD_MATRIX_SLOTS, 16);
  for (let slot = 1; slot <= 16; slot++) {
    assert.ok(profile.parameters.find(p => p.id === `mod_matrix_slot_${slot}_source_1`), `Mod slot ${slot} src1`);
    assert.ok(profile.parameters.find(p => p.id === `mod_matrix_slot_${slot}_source_2`), `Mod slot ${slot} src2`);
    assert.ok(profile.parameters.find(p => p.id === `mod_matrix_slot_${slot}_depth`), `Mod slot ${slot} depth`);
    assert.ok(profile.parameters.find(p => p.id === `mod_matrix_slot_${slot}_destination`), `Mod slot ${slot} dest`);
  }

  // Animate 1 & 2 Holds and Buttons
  assert.ok(profile.parameters.find(p => p.id === 'animate_1_hold' && p.address.cc === 114));
  assert.ok(profile.parameters.find(p => p.id === 'animate_2_hold' && p.address.cc === 115));
  assert.ok(profile.controller_controls.find(c => c.id === 'animate_1' && c.has_led));
  assert.ok(profile.controller_controls.find(c => c.id === 'animate_2' && c.has_led));

  // Arpeggiator Gate
  assert.ok(profile.parameters.find(p => p.id === 'arp_clock_gate' && p.address.cc === 116));

  // Distortion + Chorus/Delay/Reverb FX
  assert.ok(profile.parameters.find(p => p.id === 'distortion_level' && p.address.cc === 104));
  assert.ok(profile.parameters.find(p => p.id === 'chorus_level' && p.address.cc === 105));
  assert.ok(profile.parameters.find(p => p.id === 'delay_level' && p.address.cc === 108));
  assert.ok(profile.parameters.find(p => p.id === 'reverb_level' && p.address.cc === 112));
});

test('NovationPeakProfile validates against standard schemas', () => {
  assert.equal(sdk.validateDeviceProfile(sdk.NOVATION_PEAK_PROFILE), true);
  assert.equal(sdk.validateThemeConfig(sdk.NOVATION_PEAK_THEME), true);
});

test('Novation Peak is registered as the 5th synth in UnifiedEcosystemSchemaBundle', () => {
  const bundle = new sdk.UnifiedEcosystemSchemaBundle();
  assert.equal(bundle.length, 6);

  const synths = bundle.synthesizerProfiles();
  assert.equal(synths.length, 5); // Hydrasynth, BS2, TD-3, ND2, Peak

  const peakProfile = bundle.getProfile('novation_peak');
  assert.ok(peakProfile);
  assert.equal(peakProfile.name, 'Novation Peak');

  // Controllers
  const controllers = bundle.controllerProfiles();
  assert.equal(controllers.length, 1); // nanoKONTROL2
});

test('exportAllSchemas writes novation-peak.profile.json, novation-peak.theme.json, and unified-ecosystem-bundle.json', () => {
  const tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), 'peak-sdk-test-'));
  try {
    sdk.exportAllSchemas(tmpDir);

    const peakProfPath = path.join(tmpDir, 'novation-peak.profile.json');
    const peakThemePath = path.join(tmpDir, 'novation-peak.theme.json');
    const bundlePath = path.join(tmpDir, 'unified-ecosystem-bundle.json');

    assert.ok(fs.existsSync(peakProfPath));
    assert.ok(fs.existsSync(peakThemePath));
    assert.ok(fs.existsSync(bundlePath));

    const parsedBundle = JSON.parse(fs.readFileSync(bundlePath, 'utf8'));
    assert.equal(parsedBundle.synths.length, 6);
    assert.ok(parsedBundle.synths.some(s => s.id === 'novation_peak'));
  } finally {
    fs.rmSync(tmpDir, { recursive: true, force: true });
  }
});

test('safeSweepRange prevents dangerous loudness and self-oscillation', () => {
  const safe = sdk.NovationPeakProfile.safeSweepRange;
  assert.deepEqual(safe('filter_resonance'), { lo: 0, hi: 62 });
  assert.deepEqual(safe('distortion_level'), { lo: 0, hi: 48 });
  assert.deepEqual(safe('filter_overdrive'), { lo: 0, hi: 50 });
  assert.deepEqual(safe('delay_feedback'), { lo: 0, hi: 70 });
  assert.deepEqual(safe('reverb_level'), { lo: 0, hi: 90 });
  assert.deepEqual(safe('filter_cutoff'), { lo: 0, hi: 127 });
});
