//! Generates the deterministic compact-action corpora used by the scanning measurement. Native only.
//!
//!   cargo run --release --example gen_fixture            # writes fixtures/compact-{orchard-v2,ironwood-v3}.bin
//!
//! Each corpus is built with the orchard builder: batches of zero-value change outputs, each to a key picked from a
//! pool of 64 fixed random keys, except every 100th action which goes to OUR key (the one the scan decrypts for).
//! The compact form of each real action is stored: nullifier, cmx, ephemeral key, first 52 bytes of the ciphertext.
//! Layout: "KCPS", u32 version (1), u32 count, u32 ours, then count * 148 bytes (nf 32, cmx 32, epk 32, enc 52).
use orchard::{
    builder::{Builder, BundleType},
    bundle::BundleVersion,
    keys::{FullViewingKey, Scope, SpendingKey},
    value::NoteValue,
    Address, Anchor,
};
use konclave_prove_poc::scan::KEY_SEED;
use rand::{rngs::StdRng, Rng, RngCore, SeedableRng};
use std::{fs, path::Path};

const COUNT: usize = 10_000;
const OUTPUTS_PER_BUNDLE: usize = 16;
const OURS_EVERY: usize = 100;
// The count of "ours" below assumes whole bundles: no bundle is cut at the end.
const _: () = assert!(COUNT % OUTPUTS_PER_BUNDLE == 0);

fn key_from_seed(bytes: [u8; 32]) -> Option<(FullViewingKey, Address)> {
    let sk = Option::<SpendingKey>::from(SpendingKey::from_bytes(bytes))?;
    let fvk = FullViewingKey::from(&sk);
    let addr = fvk.address_at(0u32, Scope::External);
    Some((fvk, addr))
}

fn generate(pool: &str, bv: BundleVersion) -> Vec<u8> {
    let mut rng = StdRng::seed_from_u64(0x5ca7_0001);
    let ours = key_from_seed(KEY_SEED).unwrap();
    let mut others = Vec::new();
    while others.len() < 64 {
        let mut b = [0u8; 32];
        rng.fill_bytes(&mut b);
        if let Some(k) = key_from_seed(b) {
            others.push(k);
        }
    }
    let flags = bv.default_flags();
    let mut body = Vec::with_capacity(COUNT * 148);
    let mut produced = 0usize;
    let mut our_count = 0u32;
    while produced < COUNT {
        let mut builder = Builder::new(BundleType::DEFAULT, bv, flags, Anchor::empty_tree()).unwrap();
        let mut ours_flags = Vec::new();
        for _ in 0..OUTPUTS_PER_BUNDLE {
            let is_ours = (produced + ours_flags.len()) % OURS_EVERY == 0;
            let (fvk, addr) = if is_ours { ours.clone() } else { others[rng.gen_range(0..others.len())].clone() };
            builder.add_change_output(fvk, None, addr, NoteValue::from_raw(0), [0u8; 512]).unwrap();
            ours_flags.push(is_ours);
        }
        let (bundle, _) = builder.build::<i64>(&mut rng).unwrap().unwrap();
        // The builder shuffles actions, so ours cannot be tracked by position: count them per bundle. That is only
        // right if each change output became exactly one action, which is asserted here and re-checked by the
        // tests in src/scan.rs (they decrypt the corpus and expect exactly this many).
        assert_eq!(bundle.actions().len(), OUTPUTS_PER_BUNDLE, "{pool}: one action per change output");
        our_count += ours_flags.iter().filter(|x| **x).count() as u32;
        for a in bundle.actions().iter() {
            let enc = &a.encrypted_note().enc_ciphertext;
            body.extend_from_slice(&a.nullifier().to_bytes());
            body.extend_from_slice(&a.cmx().to_bytes());
            body.extend_from_slice(&a.encrypted_note().epk_bytes);
            body.extend_from_slice(&enc[..52]);
            produced += 1;
        }
    }
    assert_eq!(produced, COUNT);
    assert_eq!(our_count as usize, COUNT / OURS_EVERY);
    let mut out = Vec::with_capacity(16 + body.len());
    out.extend_from_slice(b"KCPS");
    out.extend_from_slice(&1u32.to_le_bytes());
    out.extend_from_slice(&(COUNT as u32).to_le_bytes());
    out.extend_from_slice(&our_count.to_le_bytes());
    out.extend_from_slice(&body);
    eprintln!("{pool}: {COUNT} actions, {our_count} ours, {} bytes", out.len());
    out
}

fn main() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures");
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join("compact-orchard-v2.bin"), generate("orchard-v2", BundleVersion::orchard_v2())).unwrap();
    fs::write(dir.join("compact-ironwood-v3.bin"), generate("ironwood-v3", BundleVersion::ironwood_v3())).unwrap();
}
// SPDX-License-Identifier: MIT OR Apache-2.0
