# konclave-prove-poc

A measurement harness for one question: how long do the CPU-heavy parts of a shielded Zcash wallet take in WebAssembly, on one thread,
in a phone browser? It measures three things on synthetic data, with the `orchard` crate (0.15.5):

1. **Proving** one Halo2 Action-circuit proof (2 actions by default), Orchard or Ironwood circuit, verified after proving.
2. **Trial decryption plus commitment-tree append** of 10,000 compact actions, one thread. This is not a sync time and not WebZjs:
   no download, no database, no witnesses, no real chain density.
3. **Decryption split over K Web Workers** (message passing, no shared memory, no COOP/COEP headers; not wasm threads).

The page runs in your own browser: https://deegalabs.github.io/konclave-prove-poc/ . It makes no network request after load, collects nothing,
and uses no vault and no key of ours. Results are shown on the page and copied by you.

## Results

Ironwood pool, medians of the runs of each session, all proofs verified, all decryptions checked (exactly the 100 notes of the test key).
Raw output for every row is in `results/`; `node scripts/summarize.mjs results` recomputes every median below from it.

**Proving, 2 actions, one thread (seconds).** Medians per session, so a range means several sessions on the same phone.

| Device | Browser | Median(s) | Proving-key build |
|---|---|---|---|
| Galaxy S24 Ultra, 12 GB | Firefox 157/158 | 13.4, 15.5, 22.9, 25.9 | 11.5 to 15.6 s |
| Galaxy S24 Ultra, 12 GB | Chrome 155 | 23.0, 26.4, 35.0 | 13.1 to 17.1 s |
| Galaxy A72, 6 GB | Firefox 157/158 | 48.9, 48.9 | 33.3 to 33.5 s |
| Galaxy A72, 6 GB | Chrome 155 | 49.4, 49.6 | 33.6 to 37.7 s |
| Galaxy A14, 4 GB | Firefox 158 | 132.0, 132.3 | 111.3 to 111.7 s |
| Galaxy A14, 4 GB | Chrome 146/155 | 268.4, 278.8 | 137.0 to 138.8 s |
| Desktop (WSL2, 16 logical cores) | Chromium 151 headless | 21.4 | 15.2 s |

The key is built once per page load, before the first proof. Orchard and Ironwood circuits are the same size and took the same time when run back to back on a cool phone.

**Trial decryption plus tree append, one thread, 10,000 actions (seconds), session of 2026-10-27.**

| Device | Browser | Decrypt | Append | Total | Per 1,000 actions |
|---|---|---|---|---|---|
| Galaxy S24 Ultra | Firefox 158 | 1.94 | 3.65 | 5.59 | 0.56 |
| Galaxy S24 Ultra | Chrome 155 | 2.31 | 4.41 | 6.71 | 0.67 |
| Galaxy A72 | Firefox 158 | 6.09 | 12.25 | 18.33 | 1.83 |
| Galaxy A72 | Chrome 155 | 6.06 | 13.13 | 19.18 | 1.92 |
| Galaxy A14 | Firefox 158 | 19.46 | 43.21 | 62.46 | 6.25 |
| Galaxy A14 | Chrome 155 | 24.74 | 52.21 | 76.95 | 7.70 |
| Desktop, headless | Chromium 151 | 2.81 | 5.25 | 8.07 | 0.81 |

The tree append is 65% to 70% of the total on every device. It is sequential here. A real scanner multiplies the per-1,000-action figure by the
number of actions in the range it scans; this harness does not know that number for any real chain.

**Decryption over K workers: wall time and gain over one thread (the decryption median above).**

| Device, browser | One thread | K = 4 | K = 8 |
|---|---|---|---|
| S24 Ultra, Firefox | 1.94 s | 0.73 s (2.7x) | 0.56 s (3.5x) |
| S24 Ultra, Chrome | 2.31 s | 0.82 s (2.8x) | 0.57 s (4.0x) |
| A72, Firefox | 6.09 s | 2.71 s (2.2x) | 2.21 s (2.8x) |
| A72, Chrome | 6.06 s | 2.70 s (2.2x) | 2.90 s (2.1x) |
| A14, Firefox | 19.46 s | 5.77 s (3.4x) | 4.14 s (4.7x) |
| A14, Chrome | 24.74 s | 6.01 s (4.1x) | 3.19 s (7.8x) |
| Desktop, 16 logical cores | 2.82 s | 0.76 s (3.7x) | 0.47 s (6.0x), 0.40 s at K = 16 (7.0x) |

On the A72 the per-worker times fall into two groups (about 1.55 s and 2.69 s at K = 4), consistent with cores of different speed; the chip was not
inspected. The A14 in Chrome scales almost linearly. On the A14 in Firefox the K = 8 run came after a 2-minute proof, on a warmer phone than the K = 4 run.

**Proving cost against the number of actions** (desktop, Node 24, 3 runs each, `results/2026-10-16-scaling-desktop-node-outputs*.json`): 2 actions 18.9 s,
4 actions 37.4 s, 8 actions 69.7 s, 16 actions 129.4 s; wasm memory 101 MB to 396 MB. Not measured on phones.

**Cross-check against ChainSafe's benchmark** ([`ChainSafe/zcash-wasm-benchmark`](https://github.com/ChainSafe/zcash-wasm-benchmark) at `2c6d6db`, unchanged, built with
its pinned `nightly-2024-02-26`, headless Chromium 151, same desktop, Orchard, 1 spend): 1 thread 21.2 s, 4 threads 7.6 s and 10.3 s. Our one-thread proof on the same
machine: 21.4 s. Same order of magnitude, not the same code: theirs is orchard 0.8 and a 1-spend dummy transaction. Their results are printed to the console, so they
were not collected on the phones. `chainsafe-run.mjs` is the Playwright driver used.

## Method

- One thread of wasm (`wasm32-unknown-unknown`, release, LTO), each task in a Web Worker so the page stays responsive. No `SharedArrayBuffer`.
- Proving: a bundle built with the orchard builder (zero-value outputs to a fixed test key, empty-tree anchor), `create_proof` timed alone; `prepare` and
  `verify` are timed separately. The proof is then signed with the test key and checked with `verify_proof`.
- Scanning: 10,000 compact actions per pool, written by `examples/gen_fixture.rs` from fixed seeds (the file hashes are reproducible); 1 in 100 is for the test key.
  `batch::try_compact_note_decryption` with one prepared incoming viewing key, then `Frontier::append` of every commitment. Each decrypt call also builds the per-action
  domain and clones the actions. Download of the fixture and its parsing are timed and reported but not included.
- Every run is listed; the first run of a session is marked cold. Medians are recomputed from the raw JSON by `scripts/summarize.mjs`.
- Each result carries a device label typed on the page, the user agent, the harness commit (`meta.commit`) and the browser's own `hardwareConcurrency`.

## Reproduce

```
./build.sh                          # rustup (stable, wasm32-unknown-unknown) and wasm-pack
python3 -m http.server 8080         # in this folder
cargo test --release                # native: proofs verify, decryption finds exactly the test key's notes, fixtures are refused when malformed
cargo run --release --example gen_fixture   # regenerates fixtures/*.bin byte for byte
node scripts/summarize.mjs results  # medians from the raw JSON
```

For a phone: `adb reverse tcp:8080 tcp:8080` over USB and open `http://localhost:8080`, or open the Pages link above. The page is static and single threaded, so it
needs no special headers. Threaded wasm would need cross-origin isolation; this harness neither builds nor uses it.

## What this does not show

- Three phones are not "Android". The same phone gave proving medians from 13.4 s to 25.9 s (Firefox) and 23.0 s to 35.0 s (Chrome) across sessions, with single runs
  up to 76.7 s and 192.5 s in Chrome on the S24. No temperature was recorded, so heat is a guess, not a finding. On the A14 in Chrome one verify took 1,222 s, cause unknown.
- Synthetic data: no PCZT envelope, no real note witnesses, no real chain density, no download, no database, no spend detection. Scanning is a proxy for the CPU work of a scan.
- The tree append is a plain frontier append. Reading `shardtree` 0.8.0 shows its batch insertion also hashes adjacent ephemeral leaves eagerly, but it was not measured inside a wallet.
- Web Workers are not wasm threads; the worker figures do not predict a `wasm-bindgen-rayon` build.
- No battery, no thermal curve, no iOS, no background tab. Phones were attached to USB (adb reverse) during every cable run.
- Browser and version differ between some rows (for example the A14 proving rows), so a pair of rows is not a ranking of browsers.

## Provenance of the files in `results/`

- The `commit` field in the JSON of 2026-10-15 and 16 is the commit of the development branch the page was built from, not a commit of this repository. The source files of that
  build are byte-identical to this repository's first commit (`src/lib.rs`, `index.html`, `worker.js`, `node-run.mjs`, `Cargo.lock`). Files of 2026-10-24 and later carry `ad147d6` or a later commit of this repository.
- Files named `*-session.json` hold the page's "Copy all" output; where a chat split a paste, the parts were joined at the break and nothing else was edited (each file's `note` says so).
- Files without a device label in the JSON take their device from the file name.

## Credits and licence

Proving and note decryption are done by the `orchard` crate, `halo2_proofs`, `zcash_note_encryption` and `incrementalmerkletree` from the Zcash ecosystem; the benchmark comparison is ChainSafe's work.
This harness is dual licensed Apache-2.0 / MIT.
