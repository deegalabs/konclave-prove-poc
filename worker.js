// SPDX-License-Identifier: MIT OR Apache-2.0
// Runs the benchmark off the main thread so the page stays responsive. Single thread, no SharedArrayBuffer.
import init, { Poc } from './pkg/konclave_prove_poc.js';

self.onmessage = async (ev) => {
  const { pool, runs, verify, outputs = 1 } = ev.data;
  const out = { pool, outputs, runs: [], error: null };
  try {
    const t0 = performance.now();
    const resp = await fetch('./pkg/konclave_prove_poc_bg.wasm');
    const bytes = await resp.arrayBuffer();
    out.wasmBytes = bytes.byteLength;
    const wasm = await init({ module_or_path: bytes });
    out.wasmLoadMs = Math.round(performance.now() - t0);
    const poc = new Poc(pool);
    const k0 = performance.now();
    poc.build_key();
    out.keyBuildMs = Math.round(performance.now() - k0);
    out.wasmMemoryAfterKeyBytes = wasm.memory.buffer.byteLength;
    self.postMessage({ progress: `key built in ${out.keyBuildMs} ms` });
    for (let i = 0; i < runs; i++) {
      const a0 = performance.now();
      const actions = poc.prepare_n(outputs);
      const prepareMs = Math.round(performance.now() - a0);
      const p0 = performance.now();
      poc.prove();
      const ms = Math.round(performance.now() - p0);
      let verifyMs = null;
      if (verify) {
        const v0 = performance.now();
        poc.verify();
        verifyMs = Math.round(performance.now() - v0);
      }
      out.runs.push({ run: i + 1, kind: i === 0 ? 'cold' : 'warm', prepareMs, proveMs: ms, verifyMs, actions, verified: verify });
      self.postMessage({ progress: `run ${i + 1}/${runs}: prove ${ms} ms` });
    }
    out.wasmMemoryPeakBytes = wasm.memory.buffer.byteLength; // linear memory only grows, so this is the peak
    if (performance.memory) out.jsHeapUsedBytes = performance.memory.usedJSHeapSize;
  } catch (e) {
    out.error = String(e && e.message ? e.message : e);
  }
  self.postMessage({ done: out });
};
