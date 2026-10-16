# konclave-prove-poc

How long does one Orchard or Ironwood proof take in WebAssembly, single thread, on a phone browser? This is a small
measurement harness, not a product. It was written to put a number on a claim we had only estimated.

It proves one 2-action bundle with the real Halo2 Action circuit of the [`orchard`](https://crates.io/crates/orchard)
crate (0.15.5), for the Orchard (NU6.2 circuit) or the Ironwood (NU6.3 circuit) pool, and verifies the proof.
The bundle is synthetic: built with the orchard builder from fixed seeds, one zero-value output, padded to 2 actions.
It uses no vault, no key of ours, no network.

## Results (2026-10-15 and 16)

2-action proof, WebAssembly, single thread, every proof verified, all 5 runs listed (the first is cold). Pool is Ironwood
unless noted. Raw output for every row is in `results/`. "Session" separates runs of the same device on different days
or after a break.

| Device | RAM | Browser | Android | Session | Key build | Prove, every run (s) | Median |
|---|---|---|---|---|---|---|---|
| Galaxy S24 Ultra | 12 GB | Firefox 157 | 16 | 1 | 15.6 s | 18.5, 25.9, 25.9, 25.9, 25.8 | 25.9 s |
| Galaxy S24 Ultra | 12 GB | Firefox 157 | 16 | 2, Orchard | 15.6 s | 17.4, 17.8, 17.4, 17.4, 17.3 | 17.4 s |
| Galaxy S24 Ultra | 12 GB | Firefox 157 | 16 | 2 | 15.6 s | 17.5, 17.4, 22.9, 25.9, 25.8 | 22.9 s |
| Galaxy S24 Ultra | 12 GB | Chrome 155 | n/a | 1 | 17.1 s | 23.8, 35.1, 76.7, 25.5, 35.0 | 35.0 s |
| Galaxy S24 Ultra | 12 GB | Chrome 155 | n/a | 2 | 17.1 s | 27.6, 192.5, 24.0, 26.4, 24.7 | 26.4 s |
| Galaxy A72 | 6 GB | Chrome 155 | n/a | 1 | 33.6 s | 49.6, 49.7, 49.6, 49.5, 49.6 | 49.6 s |
| Galaxy A72 | 6 GB | Chrome 155 | n/a | 1, Orchard | 33.5 s | 49.6, 49.7, 49.5, 49.4, 49.4 | 49.5 s |
| Galaxy A72 | 6 GB | Chrome 155 | n/a | 2 | 37.6 s | 50.0, 49.7, 49.4, 49.4, 49.4 | 49.4 s |
| Galaxy A72 | 6 GB | Firefox 157 | 14 | 2 | 33.3 s | 48.8, 49.0, 48.9, 48.9, 48.9 | 48.9 s |
| Galaxy A14 | 4 GB | Chrome 146 | n/a | 1 | 137.0 s | 180.5, 183.3, 558.8, 290.8, 278.8 | 278.8 s |
| Galaxy A14 | 4 GB | Firefox 158 | 15 | 1 | 111.2 s | 128.0, 132.3, 134.0, 135.0, 130.9 | 132.3 s |
| Desktop (WSL2, 16 logical cores) | n/a | Chromium 151 headless | n/a | 1 | 15.2 s | 21.4, 21.7, 23.1, 20.7, 20.6 | 21.4 s |

The key build happens once per page load, before the first proof. Orchard and Ironwood use circuits of the same size;
on the A72 they differ by under 1%. On the S24, Orchard was faster only because it ran first on a cool phone. RAM figures
are as stated by the device owner (the browser reports a capped bucket). Android is "n/a" where Chrome's frozen user
agent says "10"; Firefox reports the real version. Phones were attached to USB (adb reverse) during every run.

About the `commit` field in the raw JSON: it is the commit of the development branch the page was built from, not a
commit of this repository. The source files (`src/lib.rs`, `index.html`, `worker.js`, `node-run.mjs`, `Cargo.lock`) are
byte-identical to the ones in this repository's first commit; only `Cargo.toml` differs, in its description, licence and
a workspace line. The JSON was left as the page printed it.

## What this does not show

- Three phones are not "Android". The spread between the fastest and slowest median is more than 10x (17 s to 279 s).
- The browser mattered a lot on one phone (A14: 279 s on Chrome 146, 132 s on Firefox 158) and not on another (A72, within
  1%). The two A14 rows come from different browsers and versions on different days, so this is not a ranking of browsers.
- Runs on one phone drift: the S24 went from 17 s to 26 s within five runs and had one 192 s run in Chrome. No temperature
  was recorded, so "the phone heats up" is a guess, not a finding.
- Two actions only. ChainSafe's own `results/proving.csv` grows steeply with the number of spends (20 spends: 122 s).
  A payroll with many payees was not measured.
- Synthetic bundle, no PCZT envelope, no real note witnesses.
- No scanning, no battery, no thermal curve over minutes (the S24 Chrome run 3 at 76.7 s and the A14 run 3 at 558.8 s
  are unexplained outliers, no temperature was recorded), no iOS, no background tab.
- The medians include the first (cold) run. It was the fastest or among the fastest in most sessions, but not in all.

## Run it

```
./build.sh                          # needs rustup (stable, wasm32-unknown-unknown) and wasm-pack
python3 -m http.server 8080         # in this folder
```

Open `http://localhost:8080`, choose the pool, 5 runs, verify ticked, "Copy JSON".
For a phone, use `adb reverse tcp:8080 tcp:8080` over USB and open `http://localhost:8080` (localhost is a secure
context; a LAN address is not). Native check: `cargo test --release`. Headless: `node node-run.mjs ironwood-v3 3`.

## Cross-check against ChainSafe's benchmark

[`ChainSafe/zcash-wasm-benchmark`](https://github.com/ChainSafe/zcash-wasm-benchmark) at commit `2c6d6db`, unchanged,
built with its pinned `nightly-2024-02-26`, Chromium 151 on the same desktop, pool Orchard, 1 spend:

| Threads | Create proving key | Proving, 1 spend |
|---|---|---|
| 4 | 8.5 s, 11.2 s | 10.3 s, 7.6 s |
| 1 | 14.5 s | 21.2 s |

Our single-thread desktop number (21.4 s) is the same order. Not the same code: theirs is orchard 0.8 and a 1-spend
dummy transaction, ours orchard 0.15.5 and a 2-action bundle. `chainsafe-run.mjs` is the Playwright driver we used.

## Credits and licence

The proving is done by the `orchard` crate and `halo2_proofs` from the Zcash ecosystem; the benchmark comparison is
ChainSafe's work. This harness is dual licensed Apache-2.0 / MIT.
