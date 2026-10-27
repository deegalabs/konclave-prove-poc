// SPDX-License-Identifier: MIT OR Apache-2.0
// Prints one line per measurement found in results/, with medians recomputed from the raw runs.
// usage: node scripts/summarize.mjs results
// It reads the page's own JSON in all the shapes this repository holds: a bare result, a {results: [...]} session,
// a {result: {...}} wrapper and the older {runs: [result, ...]} wrapper. Nothing is written.
import { readdirSync, readFileSync } from 'node:fs';
import { join } from 'node:path';

const dir = process.argv[2] ?? 'results';
const median = (a) => { const s = [...a].sort((x, y) => x - y); const m = s.length >> 1; return s.length % 2 ? s[m] : (s[m - 1] + s[m]) / 2; };
const sec = (ms) => (ms / 1000).toFixed(2);
const isResult = (o) => o && Array.isArray(o.runs) && o.runs.length && typeof o.runs[0].run === 'number';

function collect(o) {
  if (Array.isArray(o.results)) return o.results;
  if (o.result) return [o.result];
  if (Array.isArray(o.runs) && o.runs.length && isResult(o.runs[0])) return o.runs;
  return isResult(o) ? [o] : [];
}
function browser(ua = '') {
  let m;
  if ((m = ua.match(/Firefox\/(\d+)/))) return 'Firefox ' + m[1];
  if ((m = ua.match(/HeadlessChrome\/(\d+)/))) return 'Chromium ' + m[1] + ' headless';
  if ((m = ua.match(/Chrome\/(\d+)/))) return 'Chrome ' + m[1];
  return ua ? 'other' : 'Node';
}
function device(file, r) {
  if (r.deviceLabel) return r.deviceLabel;
  const f = file.toLowerCase();
  if (f.includes('a72')) return 'Galaxy A72';
  if (f.includes('a14')) return 'Galaxy A14';
  if (f.includes('s24')) return 'Galaxy S24 Ultra';
  return 'Desktop';
}

const rows = [];
for (const file of readdirSync(dir).filter((f) => f.endsWith('.json')).sort()) {
  const o = JSON.parse(readFileSync(join(dir, file), 'utf8'));
  for (const r of collect(o)) {
    const who = `${device(file, r)} | ${browser(r.device?.userAgent)} | ${file.slice(0, 10)}`;
    const ok = r.error ? `ERROR ${r.error}` : 'ok';
    if (r.kind === 'scan-trial-decryption-plus-tree-append') {
      const d = median(r.runs.map((x) => x.decryptMs)), a = median(r.runs.map((x) => x.appendMs));
      const t = median(r.runs.map((x) => x.totalMs));
      rows.push(`${who} | scan ${r.pool} | decrypt ${sec(d)} s | append ${sec(a)} s | total ${sec(t)} s | runs ${r.runs.map((x) => sec(x.totalMs)).join(', ')} | ${ok}`);
    } else if (r.kind === 'scan-decrypt-multi-worker') {
      const w = median(r.runs.map((x) => x.wallMs));
      rows.push(`${who} | decrypt over K=${r.workers} | wall ${sec(w)} s | runs ${r.runs.map((x) => sec(x.wallMs)).join(', ')} | ${ok}`);
    } else {
      const p = median(r.runs.map((x) => x.proveMs));
      const acts = r.runs[0].actions ?? '?';
      rows.push(`${who} | prove ${r.pool} ${acts} actions | median ${sec(p)} s | key ${r.keyBuildMs ? sec(r.keyBuildMs) + ' s' : 'n/a'} | runs ${r.runs.map((x) => sec(x.proveMs)).join(', ')} | ${ok}`);
    }
  }
}
console.log(rows.join('\n'));
