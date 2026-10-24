//! Scanning measurement: trial decryption plus commitment-tree append, single thread, synthetic compact actions.
//!
//! What is measured: the two CPU-heavy steps of a light-client scan, on a fixed corpus of compact actions
//! generated natively by `examples/gen_fixture.rs` (deterministic seeds):
//!   decrypt: `zcash_note_encryption::batch::try_compact_note_decryption` with one prepared incoming viewing key,
//!            for the Orchard (note V2) or Ironwood (note V3) domain, building the per-action domain inside the call;
//!   append:  `Frontier::append` of every action's note commitment (Sinsemilla hashing of the Orchard tree).
//! What is NOT measured: block download, parsing of lightwalletd messages, database writes, witness upkeep for owned
//! notes, spend detection, nullifier sets, a real chain's action density. It is not WebZjs.
use incrementalmerkletree::frontier::Frontier;
use orchard::{
    keys::{FullViewingKey, PreparedIncomingViewingKey, Scope, SpendingKey},
    note::{ExtractedNoteCommitment, Nullifier},
    note_encryption::{CompactAction, IronwoodDomain, OrchardDomain},
    tree::MerkleHashOrchard,
};
use wasm_bindgen::prelude::*;
use zcash_note_encryption::{batch::try_compact_note_decryption, EphemeralKeyBytes};

/// Must match examples/gen_fixture.rs.
pub const KEY_SEED: [u8; 32] = [7u8; 32];
const MAGIC: &[u8; 4] = b"KCPS";
const HEADER: usize = 16;
const ENTRY: usize = 32 + 32 + 32 + 52;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Pool {
    Orchard,
    Ironwood,
}

/// One scanning corpus loaded from a fixture file.
#[wasm_bindgen]
pub struct Scan {
    pool: Pool,
    actions: Vec<CompactAction>,
    expected_ours: u32,
    ivk: PreparedIncomingViewingKey,
}

#[wasm_bindgen]
impl Scan {
    /// `pool` is "orchard-v2" or "ironwood-v3". `fixture` is the file bytes. Errors if the fixture is malformed.
    #[wasm_bindgen(constructor)]
    pub fn new(pool: &str, fixture: &[u8]) -> Result<Scan, JsError> {
        Self::build(pool, fixture).map_err(|e| JsError::new(&e))
    }

    pub fn actions(&self) -> u32 {
        self.actions.len() as u32
    }

    pub fn expected_ours(&self) -> u32 {
        self.expected_ours
    }

    /// Trial-decrypts every action. Returns how many decrypted for our key (must equal `expected_ours`).
    pub fn decrypt(&self) -> u32 {
        let ivks = [self.ivk.clone()];
        match self.pool {
            Pool::Orchard => {
                let outputs: Vec<(OrchardDomain, CompactAction)> = self
                    .actions
                    .iter()
                    .map(|a| (OrchardDomain::for_compact_action(a), a.clone()))
                    .collect();
                try_compact_note_decryption(&ivks, &outputs).iter().filter(|r| r.is_some()).count() as u32
            }
            Pool::Ironwood => {
                let outputs: Vec<(IronwoodDomain, CompactAction)> = self
                    .actions
                    .iter()
                    .map(|a| (IronwoodDomain::for_compact_action(a), a.clone()))
                    .collect();
                try_compact_note_decryption(&ivks, &outputs).iter().filter(|r| r.is_some()).count() as u32
            }
        }
    }

    /// Appends every action's note commitment to an empty Orchard-depth frontier. Returns the number appended.
    pub fn append(&self) -> u32 {
        let mut tree = Frontier::<MerkleHashOrchard, 32>::empty();
        let mut n = 0u32;
        for a in &self.actions {
            if tree.append(MerkleHashOrchard::from_cmx(&a.cmx())) {
                n += 1;
            }
        }
        // Touch the root so the work cannot be dropped.
        let _ = tree.root();
        n
    }
}

impl Scan {
    fn build(pool: &str, fixture: &[u8]) -> Result<Scan, String> {
        let pool = match pool {
            "orchard-v2" => Pool::Orchard,
            "ironwood-v3" => Pool::Ironwood,
            other => return Err(format!("unknown pool {other}")),
        };
        if fixture.len() < HEADER || &fixture[0..4] != MAGIC {
            return Err("not a compact-action fixture".into());
        }
        let count = u32::from_le_bytes(fixture[8..12].try_into().unwrap()) as usize;
        let ours = u32::from_le_bytes(fixture[12..16].try_into().unwrap());
        if fixture.len() != HEADER + count * ENTRY {
            return Err(format!("fixture length {} does not match {count} entries", fixture.len()));
        }
        let mut actions = Vec::with_capacity(count);
        for i in 0..count {
            let e = &fixture[HEADER + i * ENTRY..HEADER + (i + 1) * ENTRY];
            let nf: [u8; 32] = e[0..32].try_into().unwrap();
            let cmx: [u8; 32] = e[32..64].try_into().unwrap();
            let epk: [u8; 32] = e[64..96].try_into().unwrap();
            let enc: [u8; 52] = e[96..148].try_into().unwrap();
            let nf = Option::<Nullifier>::from(Nullifier::from_bytes(&nf)).ok_or("invalid nullifier in fixture")?;
            let cmx = Option::<ExtractedNoteCommitment>::from(ExtractedNoteCommitment::from_bytes(&cmx))
                .ok_or("invalid cmx in fixture")?;
            actions.push(CompactAction::from_parts(nf, cmx, EphemeralKeyBytes(epk), enc));
        }
        let sk = Option::<SpendingKey>::from(SpendingKey::from_bytes(KEY_SEED)).ok_or("bad seed")?;
        let ivk = FullViewingKey::from(&sk).to_ivk(Scope::External);
        Ok(Scan { pool, actions, expected_ours: ours, ivk: PreparedIncomingViewingKey::new(&ivk) })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(pool: &str) -> Vec<u8> {
        let path = format!("{}/fixtures/compact-{pool}.bin", env!("CARGO_MANIFEST_DIR"));
        std::fs::read(path).expect("run: cargo run --release --example gen_fixture")
    }

    #[test]
    fn decrypts_exactly_our_actions_for_each_pool() {
        for pool in ["orchard-v2", "ironwood-v3"] {
            let s = Scan::build(pool, &fixture(pool)).unwrap();
            assert!(s.actions() > 0);
            assert_eq!(s.decrypt(), s.expected_ours(), "{pool}");
            assert_eq!(s.append(), s.actions(), "{pool}");
        }
    }

    #[test]
    fn a_pool_mismatch_decrypts_nothing() {
        // Orchard-domain decryption must refuse Ironwood (V3) notes and the other way round: the lead byte differs.
        let s = Scan::build("orchard-v2", &fixture("ironwood-v3")).unwrap();
        assert_eq!(s.decrypt(), 0);
        let s = Scan::build("ironwood-v3", &fixture("orchard-v2")).unwrap();
        assert_eq!(s.decrypt(), 0);
    }

    #[test]
    fn malformed_fixtures_are_refused() {
        assert!(Scan::build("orchard-v2", b"nope").is_err());
        let mut f = fixture("orchard-v2");
        f.truncate(f.len() - 1);
        assert!(Scan::build("orchard-v2", &f).is_err());
        assert!(Scan::build("sapling", &fixture("orchard-v2")).is_err());
    }
}
