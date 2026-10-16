// Headless sanity run of the same wasm in Node (V8). Not a browser result: use index.html for those.
// usage: node node-run.mjs <pool> <runs> [outputs]
import { readFile } from 'node:fs/promises';
import init, { Poc } from './pkg/konclave_prove_poc.js';

const pool = process.argv[2] ?? 'orchard-v2';
const runs = Number(process.argv[3] ?? 2);
const outputs = Number(process.argv[4] ?? 1); // actions = max(2, outputs)
const bytes = await readFile(new URL('./pkg/konclave_prove_poc_bg.wasm', import.meta.url));
const wasm = await init({ module_or_path: bytes });
const poc = new Poc(pool);
let t = performance.now();
poc.build_key();
const keyBuildMs = Math.round(performance.now() - t);
const out = { pool, outputs, node: process.version, keyBuildMs, runs: [] };
for (let i = 0; i < runs; i++) {
  const actions = poc.prepare_n(outputs);
  t = performance.now();
  poc.prove();
  const proveMs = Math.round(performance.now() - t);
  t = performance.now();
  poc.verify();
  out.runs.push({ run: i + 1, actions, proveMs, verifyMs: Math.round(performance.now() - t), verified: true });
}
out.wasmMemoryPeakBytes = wasm.memory.buffer.byteLength;
console.log(JSON.stringify(out));
