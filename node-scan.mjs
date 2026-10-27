// SPDX-License-Identifier: MIT OR Apache-2.0
// Headless sanity run of the scanning measurement in Node (V8). Not a browser result.
// usage: node node-scan.mjs <orchard-v2|ironwood-v3> <runs>
import { readFile } from 'node:fs/promises';
import init, { Scan } from './pkg/konclave_prove_poc.js';

const pool = process.argv[2] ?? 'orchard-v2';
const runs = Number(process.argv[3] ?? 3);
const wasm = await init({ module_or_path: await readFile(new URL('./pkg/konclave_prove_poc_bg.wasm', import.meta.url)) });
const fixture = new Uint8Array(await readFile(new URL(`./fixtures/compact-${pool}.bin`, import.meta.url)));
const scan = new Scan(pool, fixture);
const out = { kind: 'scan-trial-decryption-plus-tree-append', pool, node: process.version, actions: scan.actions(), expectedOurs: scan.expected_ours(), runs: [] };
for (let i = 0; i < runs; i++) {
  let t = performance.now();
  const ours = scan.decrypt();
  const decryptMs = Math.round(performance.now() - t);
  t = performance.now();
  const appended = scan.append();
  const appendMs = Math.round(performance.now() - t);
  out.runs.push({ run: i + 1, decryptMs, appendMs, totalMs: decryptMs + appendMs, ours, appended, ok: ours === out.expectedOurs && appended === out.actions });
}
out.wasmMemoryPeakBytes = wasm.memory.buffer.byteLength;
console.log(JSON.stringify(out));
