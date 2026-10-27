// SPDX-License-Identifier: MIT OR Apache-2.0
// Runs ChainSafe's UNCHANGED demo page (built from its repo) in headless Chromium and prints console lines.
// usage: node run-cs.mjs <chromium|firefox> <threads> <spends> <distDir>
import http from 'node:http';
import { readFile } from 'node:fs/promises';
import path from 'node:path';

const [engine, threads, spends, dist] = process.argv.slice(2);
const types = { '.html': 'text/html', '.js': 'text/javascript', '.wasm': 'application/wasm', '.css': 'text/css', '.map': 'application/json' };
const srv = http.createServer(async (req, res) => {
  let p = decodeURIComponent(req.url.split('?')[0]);
  if (p === '/') p = '/index.html';
  try {
    const b = await readFile(path.join(dist, p));
    res.writeHead(200, {
      'content-type': types[path.extname(p)] ?? 'application/octet-stream',
      'cross-origin-opener-policy': 'same-origin',
      'cross-origin-embedder-policy': 'require-corp',
    });
    res.end(b);
  } catch { res.writeHead(404); res.end(); }
}).listen(0, '127.0.0.1');
await new Promise(r => srv.on('listening', r));
const url = `http://127.0.0.1:${srv.address().port}/`;

const pw = await import(process.env.PLAYWRIGHT_MODULE);
const browser = await pw[engine].launch();
const page = await browser.newPage();
const lines = [];
page.on('console', m => { lines.push(m.text()); });
page.on('pageerror', e => lines.push('PAGEERROR ' + e.message));
await page.goto(url);
await page.waitForTimeout(3000);
console.error('isolated:', await page.evaluate(() => crossOriginIsolated), 'cores:', await page.evaluate(() => navigator.hardwareConcurrency));
const nums = page.locator('input[type=number]');
await nums.nth(0).fill(String(threads));
await page.getByRole('button', { name: 'Init Threadpool' }).click();
await page.waitForTimeout(4000);
await page.locator('select').nth(1).selectOption('orchard');
// number of spends is the last number input
await nums.last().fill(String(spends));
await page.locator('button', { hasText: 'Start' }).last().click({ timeout: 25 * 60 * 1000 });
const t0 = Date.now();
const deadline = t0 + 25 * 60 * 1000;
while (Date.now() < deadline) {
  await page.waitForTimeout(2000);
  if (lines.some(l => /proof|Proof|took|ms/.test(l) && !/Initializing/.test(l))) {
    await page.waitForTimeout(3000);
    break;
  }
}
console.log(JSON.stringify({ engine, version: browser.version(), threads: Number(threads), spends: Number(spends), wallMs: Date.now() - t0, console: lines }));
await browser.close();
srv.close();
