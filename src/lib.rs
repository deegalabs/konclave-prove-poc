//! PoC: how long does one 2-action Orchard / Ironwood proof take in WASM on a phone?
//!
//! What is proved: the real Halo2 Action circuit of orchard 0.15.5, for the circuit version of the
//! chosen pool, over a bundle built here with the orchard builder (a zero-value change output, padded
//! to 2 actions by the builder, fixed seed). The PCZT Prover role of `pczt` 0.9.3 parses a PCZT and
//! then calls exactly this `create_proof`; the PoC skips the PCZT envelope and measures the proof.
//! Nothing here touches a vault, a key of ours or a network.
use orchard::{
    builder::{Builder, BundleType},
    bundle::BundleVersion,
    circuit::{OrchardCircuitVersion, ProvingKey, VerifyingKey},
    keys::{FullViewingKey, Scope, SpendAuthorizingKey, SpendingKey},
    value::NoteValue,
    Anchor,
};
use rand::{rngs::StdRng, SeedableRng};
use wasm_bindgen::prelude::*;

pub mod scan;

/// Fixed seeds so every run proves the same statement shape.
const KEY_SEED: [u8; 32] = [7u8; 32];
const RNG_SEED: u64 = 0x4b6f_6e63_6c61_7665; // "Konclave"

pub const ORCHARD_VERSION: &str = "0.15.5";

fn parse(pool: &str) -> Result<(BundleVersion, OrchardCircuitVersion), String> {
    match pool {
        // Orchard pool as it ran between NU6.2 and NU6.3.
        "orchard-v2" => Ok((BundleVersion::orchard_v2(), OrchardCircuitVersion::FixedPostNu6_2)),
        // Orchard pool at NU6.3 and later (withdraw-only): same circuit as Ironwood.
        "orchard-v3" => Ok((BundleVersion::orchard_v3(), OrchardCircuitVersion::PostNu6_3)),
        // Ironwood pool (NU6.3).
        "ironwood-v3" => Ok((BundleVersion::ironwood_v3(), OrchardCircuitVersion::PostNu6_3)),
        other => Err(format!("unknown pool {other}")),
    }
}

type Unproven = orchard::Bundle<
    orchard::builder::InProgress<orchard::builder::Unproven, orchard::builder::Unauthorized>,
    i64,
>;
type Proven = orchard::Bundle<
    orchard::builder::InProgress<orchard::Proof, orchard::builder::Unauthorized>,
    i64,
>;

/// One benchmark session: a proving key, then any number of (prepare, prove, verify) rounds.
/// Each phase is its own call so the page can time them separately.
#[wasm_bindgen]
pub struct Poc {
    pool: String,
    bundle_version: BundleVersion,
    circuit: OrchardCircuitVersion,
    pk: Option<ProvingKey>,
    vk: Option<VerifyingKey>,
    unproven: Option<Unproven>,
    proven: Option<Proven>,
}

#[wasm_bindgen]
impl Poc {
    #[wasm_bindgen(constructor)]
    pub fn new(pool: &str) -> Result<Poc, JsError> {
        let (bundle_version, circuit) = parse(pool).map_err(|e| JsError::new(&e))?;
        Ok(Poc {
            pool: pool.to_string(),
            bundle_version,
            circuit,
            pk: None,
            vk: None,
            unproven: None,
            proven: None,
        })
    }

    /// Builds the proving key (runtime, no download). Done once per page load.
    pub fn build_key(&mut self) {
        self.pk = Some(ProvingKey::build(self.circuit));
    }

    /// Builds the unproven 2-action bundle (one output, padded to 2 actions). Returns the number of actions.
    pub fn prepare(&mut self) -> Result<u32, JsError> {
        self.prepare_n(1)
    }

    /// Builds an unproven bundle with `outputs` zero-value outputs to the test address. The builder pads
    /// to at least 2 actions, so the action count is max(2, outputs). Returns the number of actions.
    pub fn prepare_n(&mut self, outputs: u32) -> Result<u32, JsError> {
        let bundle = self.build_bundle(outputs).map_err(|e| JsError::new(&e))?;
        let n = bundle.actions().len() as u32;
        self.unproven = Some(bundle);
        Ok(n)
    }

    /// The proof itself: the only thing the prove time covers.
    pub fn prove(&mut self) -> Result<(), JsError> {
        let pk = self.pk.as_ref().ok_or_else(|| JsError::new("build_key first"))?;
        let bundle = self.unproven.take().ok_or_else(|| JsError::new("prepare first"))?;
        let mut rng = StdRng::seed_from_u64(RNG_SEED ^ 1);
        let proven = bundle.create_proof(pk, &mut rng).map_err(|e| JsError::new(&format!("{e:?}")))?;
        self.proven = Some(proven);
        Ok(())
    }

    /// Signs the bundle with the fixed test key and checks the proof against the verifying key.
    pub fn verify(&mut self) -> Result<(), JsError> {
        let proven = self.proven.take().ok_or_else(|| JsError::new("prove first"))?;
        if self.vk.is_none() {
            self.vk = Some(VerifyingKey::build(self.circuit));
        }
        let sk = Option::<SpendingKey>::from(SpendingKey::from_bytes(KEY_SEED))
            .ok_or_else(|| JsError::new("bad seed"))?;
        let ask = SpendAuthorizingKey::from(&sk);
        let mut rng = StdRng::seed_from_u64(RNG_SEED ^ 2);
        let authorized = proven
            .apply_signatures(&mut rng, [0u8; 32], &[ask])
            .map_err(|e| JsError::new(&format!("{e:?}")))?;
        authorized
            .verify_proof(self.vk.as_ref().unwrap())
            .map_err(|e| JsError::new(&format!("proof did not verify: {e:?}")))
    }

    pub fn pool(&self) -> String {
        self.pool.clone()
    }
}

impl Poc {
    fn build_bundle(&self, outputs: u32) -> Result<Unproven, String> {
        if outputs == 0 || outputs > 64 {
            return Err(format!("outputs must be 1 to 64, got {outputs}"));
        }
        let sk = Option::<SpendingKey>::from(SpendingKey::from_bytes(KEY_SEED)).ok_or("bad seed")?;
        let fvk = FullViewingKey::from(&sk);
        let addr = fvk.address_at(0u32, Scope::External);
        let flags = self.bundle_version.default_flags();
        let mut b = Builder::new(BundleType::DEFAULT, self.bundle_version, flags, Anchor::empty_tree())
            .map_err(|e| format!("{e:?}"))?;
        for _ in 0..outputs {
            b.add_change_output(fvk.clone(), None, addr, NoteValue::from_raw(0), [0u8; 512])
                .map_err(|e| format!("{e:?}"))?;
        }
        let mut rng = StdRng::seed_from_u64(RNG_SEED);
        let built = b.build::<i64>(&mut rng).map_err(|e| format!("{e:?}"))?;
        built.map(|(bundle, _)| bundle).ok_or_else(|| "empty bundle".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_proving_and_scanning_harnesses_share_one_test_key_seed() {
        assert_eq!(KEY_SEED, scan::KEY_SEED);
    }

    #[test]
    fn two_actions_prove_and_verify_for_each_pool() {
        for pool in ["orchard-v2", "ironwood-v3"] {
            let mut p = Poc::new(pool).map_err(|_| ()).unwrap();
            p.build_key();
            let actions = p.prepare().map_err(|_| "prepare failed").unwrap();
            assert_eq!(actions, 2, "{pool}");
            p.prove().map_err(|_| "prove failed").unwrap();
            p.verify().map_err(|_| "verify failed").unwrap();
        }
    }

    #[test]
    fn n_outputs_give_n_actions_and_verify() {
        for pool in ["orchard-v2", "ironwood-v3"] {
            let mut p = Poc::new(pool).map_err(|_| ()).unwrap();
            p.build_key();
            for (outputs, expected) in [(1u32, 2u32), (2, 2), (4, 4), (8, 8)] {
                let actions = p.prepare_n(outputs).map_err(|_| "prepare failed").unwrap();
                assert_eq!(actions, expected, "{pool} outputs={outputs}");
                p.prove().map_err(|_| "prove failed").unwrap();
                p.verify().map_err(|_| "verify failed").unwrap();
            }
        }
    }

    #[test]
    fn zero_or_too_many_outputs_are_refused() {
        let mut p = Poc::new("ironwood-v3").map_err(|_| ()).unwrap();
        assert!(p.build_bundle(0).is_err());
        assert!(p.build_bundle(65).is_err());
    }
}
// SPDX-License-Identifier: MIT OR Apache-2.0
