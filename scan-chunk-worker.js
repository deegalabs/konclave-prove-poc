// One data-parallel worker of the multi-worker decryption comparison: it holds its slice of the corpus in its own
// wasm instance (no SharedArrayBuffer, no COOP/COEP) and trial-decrypts it on 'go'. These are Web Workers with
// message passing, NOT wasm threads.
import init, { Scan } from './pkg/konclave_prove_poc.js';

let scan = null;
self.onmessage = async (ev) => {
  const m = ev.data;
  try {
    if (m.type === 'init') {
      await init({ module_or_path: await (await fetch('./pkg/konclave_prove_poc_bg.wasm')).arrayBuffer() });
      scan = new Scan(m.pool, m.fixture);
      self.postMessage({ type: 'ready', actions: scan.actions() });
    } else if (m.type === 'go') {
      const t = performance.now();
      const ours = scan.decrypt();
      self.postMessage({ type: 'done', ms: Math.round(performance.now() - t), ours });
    }
  } catch (e) {
    self.postMessage({ type: 'error', error: String(e && e.message ? e.message : e) });
  }
};
