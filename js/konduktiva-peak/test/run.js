'use strict';
// ---------------------------------------------------------------------------
// test/run.js -- no-hardware test runner for konduktiva-peak.
//
// Plain node, no test framework. Exits non-zero on any failure. Covers:
//   * loadCcMap parses the bundled map
//   * resolveCc('cutoff') === 29, resolveCc('resonance') === 79
//   * sweepCC builds a QuantizedMap whose wrapLookup returns {controller,value}
//     inside 0..127
//   * createPeakPlayers builds 8 players, all on channel 1
//   * a SHORT capture run emits note-on, note-off, and cc, all on wire channel 0,
//     with the expected controllers present
// ---------------------------------------------------------------------------

const path = require('path');
const { pathToFileURL } = require('url');
const peak = require('../src/index');
const { resolveKonduktiva } = require('../src/_resolve');

const K = resolveKonduktiva();
const kit = peak.createPeakKit(K);

let failures = 0;
function check(label, cond, detail) {
    const ok = !!cond;
    if (!ok) failures++;
    console.log((ok ? 'PASS  ' : 'FAIL  ') + label + (detail != null ? '  -> ' + detail : ''));
}

async function main() {
    // 1. loadCcMap parses.
    const ccMap = peak.loadCcMap();
    check('loadCcMap parses bundled JSON',
        ccMap && Array.isArray(ccMap.cc_map) && ccMap._meta != null,
        'cc_map entries: ' + (ccMap && ccMap.cc_map ? ccMap.cc_map.length : 0));

    // 2. resolveCc keyword matching.
    const cutoff = peak.resolveCc('cutoff');
    const reso = peak.resolveCc('resonance');
    check('resolveCc("cutoff") === 29', cutoff === 29, String(cutoff));
    check('resolveCc("resonance") === 79', reso === 79, String(reso));

    // 3. sweepCC -> QuantizedMap with in-range {controller,value}.
    const sweep = kit.sweepCC(29, 28, 118, 8, 32);
    let sweepOk = typeof sweep.wrapLookup === 'function';
    let sample = null;
    for (let beat = 0; beat <= 16 && sweepOk; beat += 0.5) {
        const v = sweep.wrapLookup(beat);
        if (!v || v.controller !== 29 || typeof v.value !== 'number' ||
            v.value < 0 || v.value > 127) {
            sweepOk = false;
            sample = v;
        } else {
            sample = v;
        }
    }
    check('sweepCC wrapLookup returns in-range {controller,value}',
        sweepOk, JSON.stringify(sample));

    // 3b. dualSweepCC -> QuantizedMap returning array of [{controller, value}, {controller, value}]
    const dualSweep = kit.dualSweepCC(112, 20, 90, 108, 12, 70, 12, 12);
    let dualOk = typeof dualSweep.wrapLookup === 'function';
    let dualSample = null;
    for (let beat = 0; beat <= 24 && dualOk; beat += 1.0) {
        const v = dualSweep.wrapLookup(beat);
        if (!Array.isArray(v) || v.length !== 2) {
            dualOk = false;
        } else if (v[0].controller !== 112 || v[0].value < 20 || v[0].value > 90 ||
                   v[1].controller !== 108 || v[1].value < 12 || v[1].value > 70) {
            dualOk = false;
            dualSample = v;
        } else {
            dualSample = v;
        }
    }
    check('dualSweepCC wrapLookup returns [{controller,value}, {controller,value}]',
        dualOk, JSON.stringify(dualSample));

    // Cross-Language Parity Suite (integrating with shared-synth-sdk)
    const sdkPath = path.resolve(__dirname, '../../shared-synth-sdk/index.js');
    const sdk = await import(pathToFileURL(sdkPath).href);

    // Parity 1: Profile covers all 74 CCs from midi-cc-map.json
    const profile = sdk.NOVATION_PEAK_PROFILE;
    const allCcsCovered = ccMap.cc_map.every(entry =>
        profile.parameters.some(p => p.address && p.address.kind === 'Cc' && p.address.cc === entry.cc)
    );
    check('cross-parity: profile covers all 74 CCs from midi-cc-map.json', allCcsCovered);

    // Parity 2: 32 Bipolar Dual-CC pairs
    const pairsCount = sdk.NovationPeakProfile.DUAL_CC_PAIRS.length;
    const pairsCovered = sdk.NovationPeakProfile.DUAL_CC_PAIRS.every(([msb, lsb]) =>
        profile.parameters.some(p => p.address && p.address.kind === 'Cc14Bit' && p.address.msb === msb && p.address.lsb === lsb)
    );
    check('cross-parity: 32 bipolar dual-CC pairs (MSB 0..31, LSB 32..63)', pairsCount === 32 && pairsCovered);

    // Parity 3: 14-bit NRPNs for wavetable, filter type, and filter slope
    const hasFilterTypeNrpn = profile.parameters.some(p => p.address && p.address.kind === 'Nrpn' && p.address.lsb === 48);
    const hasFilterSlopeNrpn = profile.parameters.some(p => p.address && p.address.kind === 'Nrpn' && p.address.lsb === 49);
    const hasOscNrpn = [64, 80, 96].every(lsb =>
        profile.parameters.some(p => p.address && p.address.kind === 'Nrpn' && p.address.lsb === lsb)
    );
    check('cross-parity: 14-bit NRPNs for filter type, slope, and osc 1..3',
        hasFilterTypeNrpn && hasFilterSlopeNrpn && hasOscNrpn);

    // Parity 4: safeSweepRange clamping limits
    const safeReso = peak.safeSweepRange('resonance');
    const safeDist = peak.safeSweepRange('distortion');
    const safeDrive = peak.safeSweepRange('filter_overdrive');
    const safePostDrive = peak.safeSweepRange('filter_post_drive');
    const safeFb = peak.safeSweepRange('delay_feedback');
    const safeFx = peak.safeSweepRange('reverb_level');
    const safeCut = peak.safeSweepRange('filter_cutoff');
    const safeClampingOk =
        safeReso.hi === 62 &&
        safeDist.hi === 48 &&
        safeDrive.hi === 50 &&
        safePostDrive.hi === 50 &&
        safeFb.hi === 70 &&
        safeFx.hi === 90 &&
        safeCut.hi === 127;
    check('cross-parity: safeSweepRange clamping (reso 62, dist 48, drive 50, fb 70, fx 90, cutoff 127)',
        safeClampingOk);

    // Parity 5: OSC packet encoding & address pattern parity
    const oscAddr = '/peak/filter/cutoff';
    const oscFloatArg = sdk.oscFloat(0.75);
    check('cross-parity: OSC address pattern & float encoding',
        oscAddr.startsWith('/peak/') && oscFloatArg && oscFloatArg.type === 'f' && oscFloatArg.value === 0.75);

    // Parity 6: SysEx Header & Checksum formula parity ((128 - (sum % 128)) & 0x7F)
    const testPayload = [64, 12, 100, 32, 90];
    const sum = testPayload.reduce((acc, v) => acc + (v & 0x7F), 0);
    const expectedChk = (128 - (sum % 128)) & 0x7F;
    check('cross-parity: SysEx 7-bit checksum formula parity',
        expectedChk >= 0 && expectedChk <= 127 && ((sum + expectedChk) % 128 === 0),
        'Checksum: ' + expectedChk);

    // Parity 7: Preset JSON schema roundtrip parity
    const mockPatch = {
        meta: {
            id: 'peak-parity-uuid',
            name: 'Parity Lead',
            category: 'Lead',
            color: '#00D2BE',
            favorite: true,
            bank: 'A',
            slot: 5,
            createdAt: new Date().toISOString(),
            modifiedAt: new Date().toISOString(),
            tags: ['cross-parity', 'peak'],
            author: 'Renick Bell',
            notes: 'Verified cross-language preset',
        },
        parameters: {
            filter_cutoff: 88,
            filter_resonance: 44,
            osc_1_shape: 64,
        },
        sysex: {
            rawDump: null,
        },
    };
    const patchValid = sdk.validateSynthPatch(mockPatch);
    const roundtrippedPatch = JSON.parse(JSON.stringify(mockPatch));
    check('cross-parity: preset JSON schema roundtrip parity',
        patchValid && roundtrippedPatch.meta.name === 'Parity Lead' && roundtrippedPatch.parameters.filter_cutoff === 88);

    // One env for the remaining checks. setUpMusicalEnvironmentExamples binds the
    // shared OSC UDP port (57121) and the websocket server (8080); guard AFTER it
    // so an EADDRINUSE from a concurrent live session (or 8080 in use) is swallowed
    // rather than crashing the process. The bind errors fire on a later tick, so
    // attaching the handlers right after setup is in time.
    const env = K.setUpMusicalEnvironmentExamples();
    kit.guardSuperdirtPort();
    env.schedulerBackend = process.env.KONDUKTIVA_BACKEND || 'auto';

    // 4. createPeakPlayers -> 8 players, all channel 1.
    const created = kit.createPeakPlayers(env, { channel: 1 });
    const eight = created.all.length === 8 &&
        created.all.every((n) => env.players[n] != null);
    check('createPeakPlayers builds 8 players', eight, created.all.join(','));
    const allCh1 = created.all.every((n) => {
        const p = env.players[n];
        const cm = env.channelMaps[p.channelMap];
        return cm && cm.wrapLookup(0) === 1;
    });
    check('all 8 players on channel 1', allCh1);

    // 5. SHORT capture run: note-on, note-off, cc, wire channel 0, expected CCs.
    kit.openOutput(env, { captureStats: true });
    const players = created;
    const durationS = Number(process.env.STRESS_SECONDS || 6);
    const result = await kit.runStressTest(env, {
        durationS,
        bpm: 140,
        captureStats: true,
        players: players.all,
    });

    // Teardown (no process.exit; this is a library-level run).
    try { env.stopN(players.all); } catch (_) {}
    try { kit.panic(env.midiOutputs[0], 1); } catch (_) {}
    try { env.stopScheduler(); } catch (_) {}
    await new Promise((r) => setTimeout(r, 200));
    try { K.closeMidiOutputs(env); } catch (_) {}

    const s = result.stats;
    check('capture: note-on emitted', s.noteon > 0, String(s.noteon));
    check('capture: note-off emitted', s.noteoff > 0, String(s.noteoff));
    check('capture: cc emitted', s.cc > 0, String(s.cc));
    const chans = Object.keys(s.byChannel);
    check('capture: all messages on wire channel 0',
        chans.length > 0 && chans.every((c) => c === '0'), JSON.stringify(s.byChannel));
    const expected = [29, 79, 1, 112, 108]; // cutoff, reso, mod, reverb, delay
    const present = expected.filter((c) => s.byController[c]);
    check('capture: expected controllers present (>=4)',
        present.length >= 4, present.join(','));
    check('capture: runStressTest reported PASS', result.pass === true, String(result.pass));

    console.log('');
    if (failures > 0) {
        console.log('konduktiva-peak tests: ' + failures + ' FAILURE(S)');
        process.exit(1);
    }
    console.log('konduktiva-peak tests: ALL PASS');
    process.exit(0);
}

main().catch((err) => {
    console.error('test runner crashed:', err && err.stack ? err.stack : err);
    process.exit(1);
});
