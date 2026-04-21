// main.rs

use std::sync::Arc;

use common::{types, FinalizedTx};
use config::{
    constants::CONTRACT_INPUT_PATH,
    runparams::{MyAdaptorSignatureScheme, MyCryptoUtils, MyOracle, MySignature},
    MAX_OUTCOME,
};
use dlc_controller::{very_simple_controller::VerySimpleController, ControllerType, DlcController};
use secp256k1_zkp::Secp256k1;

mod adaptor_signature_scheme;
mod common;
mod config;
mod crypto_utils;
mod dlc_computation;
mod dlc_controller;
mod dlc_storage;
mod oracle;
mod parser;

mod bench {
    use std::time::Duration;
    #[cfg(feature = "enable-benchmarks")]
    use std::time::Instant;

    #[cfg(feature = "enable-benchmarks")]
    pub fn measure_step<R, F: FnOnce() -> R>(
        label: &str,
        steps: &mut Vec<(String, Duration)>,
        f: F,
    ) -> R {
        let start = Instant::now();
        let result = f();
        let duration = start.elapsed();
        println!("{}: {}ms", label, duration.as_millis());
        steps.push((label.to_string(), duration));
        result
    }

    #[cfg(not(feature = "enable-benchmarks"))]
    pub fn measure_step<R, F: FnOnce() -> R>(
        _label: &str,
        _steps: &mut [(String, Duration)],
        f: F,
    ) -> R {
        // No-op version
        f()
    }

    #[cfg(feature = "enable-benchmarks")]
    fn print_party_table(steps: &[(String, Duration)], party_name: &str) {
        let mut total_time = Duration::new(0, 0);
        for (_, step_dur) in steps {
            total_time += *step_dur;
        }

        println!("\n-------------------------------------------------------------");
        println!("{:<35}{:<15}{:<15}", "STEP", "TIME", "RATIO");
        println!("-------------------------------------------------------------");

        for (label, step_dur) in steps {
            let ratio = if total_time.is_zero() {
                0.0
            } else {
                (step_dur.as_secs_f64() / total_time.as_secs_f64()) * 100.0
            };

            println!(
                "{:<35}{:<15}{:.2}%",
                label,
                format!("{}ms", step_dur.as_millis()),
                ratio
            );
        }

        println!("-------------------------------------------------------------");
        println!(
            "{:<35}{:<15}{}",
            format!("TOTAL {} RUNTIME:", party_name.to_uppercase()),
            format!("{}ms", total_time.as_millis()),
            "100.00%"
        );
        println!("-------------------------------------------------------------");
    }

    // Print independent benchmark tables for Alice and Bob.
    #[cfg(feature = "enable-benchmarks")]
    pub fn print_table(steps: &[(String, Duration)]) {
        let mut alice_steps = Vec::new();
        let mut bob_steps = Vec::new();

        for (label, duration) in steps {
            let lower = label.to_lowercase();
            if lower.contains("alice") {
                alice_steps.push((label.clone(), *duration));
            } else if lower.contains("bob") {
                bob_steps.push((label.clone(), *duration));
            }
        }

        print_party_table(&alice_steps, "Alice");
        print_party_table(&bob_steps, "Bob");
    }
}

// Validates (locally) final transaction which would be broadcasted to blockchain and by doing so it
// simulates blockchain acceptance or rejection of final tx.
fn finalized_tx_valid(
    finalized_tx: &FinalizedTx<MySignature>,
    multisig: &types::MultisigFundAddress,
) -> bool {
    let secp = Secp256k1::verification_only();

    let msg = match common::fun::create_message(finalized_tx.payload.as_bytes()) {
        Ok(m) => m,
        Err(_) => return false,
    };

    let (sig1_ok, sig2_ok) = {
        #[cfg(feature = "ecdsa")]
        {
            let sig1_ok = secp
                .verify_ecdsa(&msg, &finalized_tx.offerer_sig, &multisig.offerer_pubkey)
                .is_ok();
            let sig2_ok = secp
                .verify_ecdsa(&msg, &finalized_tx.accepter_sig, &multisig.accepter_pubkey)
                .is_ok();
            (sig1_ok, sig2_ok)
        }
        #[cfg(feature = "schnorr")]
        {
            let sig1_ok = secp
                .verify_schnorr(
                    &finalized_tx.offerer_sig,
                    &msg,
                    &multisig.offerer_pubkey.x_only_public_key().0,
                )
                .is_ok();
            let sig2_ok = secp
                .verify_schnorr(
                    &finalized_tx.accepter_sig,
                    &msg,
                    &multisig.accepter_pubkey.x_only_public_key().0,
                )
                .is_ok();
            (sig1_ok, sig2_ok)
        }
    };

    if sig1_ok && sig2_ok {
        println!("Transaction \"{}\" is valid.", finalized_tx.payload);
        return true;
    }
    false
}

fn main() {
    let mut steps = Vec::new();

    // Create oracle pointer, so both controllers use API of same oracle
    let oracle = Arc::new(MyOracle::new());

    println!(
        "Oracle outcome: {:?} from {:?}",
        oracle.get_outcome(),
        MAX_OUTCOME
    );

    // Create controllers
    let mut controller_alice =
        bench::measure_step("Construct controller (Alice)", &mut steps, || {
            VerySimpleController::<MyAdaptorSignatureScheme, MyCryptoUtils, MyOracle>::new(
                ControllerType::Offerer,
                Arc::clone(&oracle),
            )
        });
    let mut controller_bob = bench::measure_step("Construct controller (Bob)", &mut steps, || {
        VerySimpleController::<MyAdaptorSignatureScheme, MyCryptoUtils, MyOracle>::new(
            ControllerType::Accepter,
            Arc::clone(&oracle),
        )
    });

    // Load input files
    bench::measure_step("Load input (Alice)", &mut steps, || {
        if let Err(e) = controller_alice.load_input(CONTRACT_INPUT_PATH) {
            eprintln!("Error loading input (Alice): {}", e);
            std::process::exit(1);
        }
    });

    bench::measure_step("Load input (Bob)", &mut steps, || {
        if let Err(e) = controller_bob.load_input(CONTRACT_INPUT_PATH) {
            eprintln!("Error loading input (Bob): {}", e);
            std::process::exit(1);
        }
    });

    // Initialize storage
    bench::measure_step("Init storage (Alice)", &mut steps, || {
        controller_alice.init_storage().unwrap();
    });
    bench::measure_step("Init storage (Bob)", &mut steps, || {
        controller_bob.init_storage().unwrap();
    });

    // Share verification keys and adaptors
    bench::measure_step("Exchange keys and adaptors (Alice)", &mut steps, || {
        controller_alice.save_cp_verification_key(controller_bob.share_verification_key());
        controller_alice.save_cp_adaptors(controller_bob.share_adaptors());
    });
    bench::measure_step("Exchange keys and adaptors (Bob)", &mut steps, || {
        controller_bob.save_cp_verification_key(controller_alice.share_verification_key());
        controller_bob.save_cp_adaptors(controller_alice.share_adaptors());
    });

    // Verify counterparty adaptors
    bench::measure_step("Verify adaptors (Alice)", &mut steps, || {
        assert!(
            controller_alice.verify_cp_adaptors(),
            "Counterparty adaptors are not valid."
        );
    });
    bench::measure_step("Verify adaptors (Bob)", &mut steps, || {
        assert!(
            controller_bob.verify_cp_adaptors(),
            "Counterparty adaptors are not valid."
        );
    });

    // Update counterparty adaptors
    bench::measure_step("Update cp adaptors (Alice)", &mut steps, || {
        controller_alice.update_cp_adaptors().unwrap()
    });
    bench::measure_step("Update cp adaptors (Bob)", &mut steps, || {
        controller_bob.update_cp_adaptors().unwrap()
    });

    // Fund the multisig address
    let multisig = types::MultisigFundAddress::new(
        controller_alice.share_verification_key(),
        controller_bob.share_verification_key(),
    );

    // Wait for oracle attestation and finalize
    // INFO: for now, we finalize all results, we don't do optimistic optimization.
    bench::measure_step("Wait attestation + finalize (Alice)", &mut steps, || {
        controller_alice.wait_attestation().unwrap();
        let finalized_tx = controller_alice.finalize_tx();
        print!("Offerer: ");
        assert!(finalized_tx_valid(&finalized_tx, &multisig));
    });

    bench::measure_step("Wait attestation + finalize (Bob)", &mut steps, || {
        controller_bob.wait_attestation().unwrap();
        let finalized_tx = controller_bob.finalize_tx();
        print!("Accepter: ");
        assert!(finalized_tx_valid(&finalized_tx, &multisig));
    });

    #[cfg(feature = "enable-benchmarks")]
    bench::print_table(&steps);
}

#[cfg(test)]
mod tests {
    use crate::{
        adaptor_signature_scheme::AdaptorSignatureScheme,
        adaptor_signature_scheme::EcdsaAdaptorSignatureScheme,
    };

    use super::*;
    use rand::thread_rng;
    use secp256k1_zkp::{Keypair, Message, Secp256k1};
    use sha2::{Digest, Sha256};

    #[test]
    fn test_ecdsa_sign() {
        let secp = Secp256k1::new();
        let (secret_key, public_key) = secp.generate_keypair(&mut thread_rng());

        let hash = Sha256::digest("Alice gets 43 sats and Bob 120 sats".as_bytes());
        let hashed_message: [u8; 32] = hash.into();
        let msg = Message::from_digest_slice(&hashed_message).unwrap();

        let sig = secp.sign_ecdsa(&msg, &secret_key);

        assert!(secp.verify_ecdsa(&msg, &sig, &public_key).is_ok());
        println!("ECDSA test passed.");
    }

    #[test]
    fn test_ecdsa_adaptor_sign() {
        use common::types::OutcomeU32;
        use crypto_utils::CryptoUtils;

        let secp = Secp256k1::new();
        let mut rng = thread_rng();

        // Generate signer keypair
        let keypair = Keypair::new(&secp, &mut rng);
        // Generate nonce keypair (for anticipation point / attestation)
        let (nonce_sk, nonce_pk) = secp.generate_keypair(&mut rng);

        // Create MyCryptoUtils engine
        let crypto_utils_engine = MyCryptoUtils::new(&keypair.public_key(), &nonce_pk);

        // Create message
        let message_str = "Adaptor signature test";
        let hash = Sha256::digest(message_str.as_bytes());
        let msg = Message::from_digest_slice(&hash).unwrap();

        // Create outcome
        let outcome_value = 42u32;
        let outcome = OutcomeU32::from(outcome_value);

        // Compute anticipation point using MyCryptoUtils
        let anticipation_point = crypto_utils_engine
            .compute_anticipation_point(&outcome)
            .expect("Failed to compute anticipation point");

        // Create adaptor signature and verify pre-adaptation
        let adaptor_sig =
            EcdsaAdaptorSignatureScheme::pre_sign(&keypair, &msg, &anticipation_point);
        assert!(
            EcdsaAdaptorSignatureScheme::pre_verify(
                &keypair.public_key(),
                &msg,
                &anticipation_point,
                &adaptor_sig
            ),
            "Pre-verification failed"
        );

        // Compute attestation using MyCryptoUtils (using nonce_sk as private nonce)
        let attestation = crypto_utils_engine
            .compute_attestation(&keypair.secret_key(), &nonce_sk, &outcome)
            .expect("Failed to compute attestation");

        // Adapt the adaptor signature using computed attestation and verify signature
        let adapted_sig = EcdsaAdaptorSignatureScheme::adapt(&adaptor_sig, &attestation);
        assert!(
            secp.verify_ecdsa(&msg, &adapted_sig, &keypair.public_key())
                .is_ok(),
            "Adapted signature verification failed"
        );

        println!("ECDSA adaptor signature test passed.");
    }
}
