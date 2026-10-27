// SPDX-License-Identifier: MIT OR Apache-2.0
// Scanning measurement, off the main thread, single thread: trial decryption plus commitment-tree append of a
// fixed corpus of synthetic compact actions. Not a sync time: no download, no database, no witnesses, not WebZjs.
import init, { Scan } from './pkg/konclave_prove_poc.js';

self.onmessage = async (ev) => {
  const { pool, runs } = ev.data;
  const out = { kind: 'scan-trial-decryption-plus-tree-append', pool, runs: [], error: null };
  try {
    const t0 = performance.now();
    const wasmBytes = await (await fetch('./pkg/konclave_prove_poc_bg.wasm')).arrayBuffer();
    out.wasmBytes = wasmBytes.byteLength;
    const wasm = await init({ module_or_path: wasmBytes });
    out.wasmLoadMs = Math.round(performance.now() - t0);
    const f0 = performance.now();
    const fixture = new Uint8Array(await (await fetch(`./fixtures/compact-${pool}.bin`)).arrayBuffer());
    out.fixtureBytes = fixture.byteLength;
    out.fixtureFetchMs = Math.round(performance.now() - f0);
    const p0 = performance.now();
    const scan = new Scan(pool, fixture);
    out.parseMs = Math.round(performance.now() - p0);
    out.actions = scan.actions();
    out.expectedOurs = scan.expected_ours();
    for (let i = 0; i < runs; i++) {
      const d0 = performance.now();
      const ours = scan.decrypt();
      const decryptMs = Math.round(performance.now() - d0);
      const a0 = performance.now();
      const appended = scan.append();
      const appendMs = Math.round(performance.now() - a0);
      const ok = ours === out.expectedOurs && appended === out.actions;
      out.runs.push({ run: i + 1, kind: i === 0 ? 'cold' : 'warm', decryptMs, appendMs, totalMs: decryptMs + appendMs, ours, appended, ok });
      self.postMessage({ progress: `run ${i + 1}/${runs}: decrypt ${decryptMs} ms, append ${appendMs} ms` });
      if (!ok) throw new Error(`run ${i + 1}: decrypted ${ours} of ${out.expectedOurs} expected, appended ${appended} of ${out.actions}`);
    }
    out.wasmMemoryPeakBytes = wasm.memory.buffer.byteLength;
  } catch (e) {
    out.error = String(e && e.message ? e.message : e);
  }
  self.postMessage({ done: out });
};
