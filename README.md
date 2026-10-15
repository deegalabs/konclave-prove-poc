# konclave-prove-poc

How long does one Orchard or Ironwood proof take in WebAssembly, single thread, on a phone browser? This is a small
measurement harness, not a product. It was written to put a number on a claim we had only estimated.

It proves one 2-action bundle with the real Halo2 Action circuit of the [`orchard`](https://crates.io/crates/orchard)
crate (0.15.5), for the Orchard (NU6.2 circuit) or the Ironwood (NU6.3 circuit) pool, and verifies the proof.
The bundle is synthetic: built with the orchard builder from fixed seeds, one zero-value output, padded to 2 actions.
It uses no vault, no key of ours, no network.

## Results (2026-10-15)

2-action proof, WebAssembly, single thread, median of 5 runs, every proof verified. Raw output is in `results/`.

| Device | RAM | Browser | Pool | Key build | Prove (median) |
|---|---|---|---|---|---|
| Galaxy S24 Ultra | 12 GB | Firefox 157 | Ironwood | 15.6 s | 25.9 s |
| Galaxy S24 Ultra | 12 GB | Chrome 155 | Ironwood | 17.1 s | 35.0 s |
| Galaxy A72 | 6 GB | Chrome 155 | Ironwood | 33.6 s | 49.6 s |
| Galaxy A14 | 4 GB | Chrome 146 | Ironwood | 137.0 s | 278.8 s |
| Desktop (WSL2, 16 logical cores) | n/a | Chromium 151 headless | Ironwood | 15.2 s | 21.4 s |

The key build happens once per page load, before the first proof. Orchard and Ironwood take the same time (same circuit
size). RAM figures are as stated by the device owner. Phones were attached to USB (adb reverse) during the runs.

## What this does not show

- Four devices are not "Android". The spread between the fastest and slowest is about 11x.
- Two actions only. ChainSafe's own `results/proving.csv` grows steeply with the number of spends (20 spends: 122 s).
  A payroll with many payees was not measured.
- Synthetic bundle, no PCZT envelope, no real note witnesses.
- No scanning, no battery, no thermal curve over minutes (the S24 Chrome run 3 at 76.7 s and the A14 run 3 at 558.8 s
  are unexplained outliers, no temperature was recorded), no iOS, no background tab.
- On each phone the first (cold) run was the fastest; the medians include it.

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
