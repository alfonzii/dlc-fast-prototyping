use crate::common::{self, types, OutcomeU32, ParsedContract};
use crate::parser::Parser;
use secp256k1_zkp::{Keypair, PublicKey, SecretKey, SECP256K1};

use crate::config::{MyParser, MySignature, NB_OUTCOMES};
use crate::crypto_utils::CryptoUtils;
use crate::dlc_computation::{unified_dlc_computation::UnifiedDlcComputation, DlcComputation};
use crate::dlc_controller::ControllerType;
use crate::dlc_storage::{simple_array_storage::SimpleArrayStorage, DlcStorage};
use crate::oracle::{Oracle, OracleAttestation};
use crate::{
    adaptor_signature_scheme::AdaptorSignatureScheme,
    dlc_controller::{
        ControllerType::{Accepter, Offerer},
        DlcController,
    },
};

use secp256k1_zkp::rand;
use std::io::Error;

use std::marker::PhantomData;
use std::str::FromStr;
use std::sync::Arc;

// To use different implementations of DlcStorage and MyDlcComputation for this specific controller,
// just change the type aliases below
type MyDlcStorage<T> = SimpleArrayStorage<T>;
type MyDlcComputation<A, C> = UnifiedDlcComputation<A, C>;

pub struct VerySimpleController<ASigS, CU, O>
where
    ASigS: AdaptorSignatureScheme,
    CU: CryptoUtils,
    O: Oracle,
{
    controller_type: ControllerType,
    oracle: Arc<O>,
    keypair: Keypair,
    storage: MyDlcStorage<ASigS>,
    parsed_contract: ParsedContract<OutcomeU32>,
    total_collateral: types::PayoutT,

    cp_verification_key: PublicKey,
    cp_adaptors: Vec<ASigS::AdaptorSignature>,
    oracle_attestation: OracleAttestation,

    _phantom_asig: PhantomData<ASigS>,
    _phantom_cu: PhantomData<CU>,
}

impl<ASigS, CU, O> DlcController<ASigS, CU, O> for VerySimpleController<ASigS, CU, O>
where
    ASigS: AdaptorSignatureScheme<Signature = MySignature>,
    ASigS::AdaptorSignature: Send + Sync,
    CU: CryptoUtils + Sync,
    O: Oracle,
{
    fn new(ctype: ControllerType, oracle: Arc<O>) -> Self {
        let keypair = Keypair::new(SECP256K1, &mut rand::thread_rng());
        let storage = MyDlcStorage::new(NB_OUTCOMES);
        let parsed_contract = ParsedContract::new();
        let cp_verification_key =
            SecretKey::from_str("0000000000000000000000000000000000000000000000000000000000000001")
                .unwrap()
                .public_key(SECP256K1);
        let cp_adaptors = Vec::new();
        let total_collateral: types::PayoutT = 0;
        let oracle_attestation = OracleAttestation {
            outcome: OutcomeU32::from(u32::MAX),
            attestation: SecretKey::new(&mut rand::thread_rng()),
        };

        Self {
            controller_type: ctype,
            oracle,
            keypair,
            storage,
            parsed_contract,
            total_collateral,
            cp_verification_key,
            cp_adaptors,
            oracle_attestation,
            _phantom_asig: PhantomData,
            _phantom_cu: PhantomData,
        }
    }

    fn load_input(&mut self, input_path: &str) -> Result<(), Error> {
        let contract_input = MyParser::read_input(input_path)?;
        // We created this small hack where we take out total_collateral instead of whole ContractInput. However, it can be changed, but for now it seems to be fine.
        self.total_collateral = contract_input.total_collateral().into();
        self.parsed_contract = MyParser::parse_contract_input(contract_input)?;
        Ok(())
    }

    fn init_storage(&mut self) -> Result<(), Error> {
        // Get (announcement) public key, public nonces and next attestation time from the oracle
        let event_anncmt = self.oracle.get_event_announcement(0);

        // Compute storage elements vector for all outcomes
        // create cet -> atp point -> adaptor sig -> storage element
        let storage_elements_vec = MyDlcComputation::<ASigS, CU>::compute_storage_elements_vec(
            &self.parsed_contract,
            self.total_collateral,
            &self.keypair,
            &event_anncmt.public_key,
            &event_anncmt.public_nonce,
        );

        // Put all elements into storage
        for ((outcome, _), element) in self.parsed_contract.iter().zip(storage_elements_vec) {
            self.storage.put_element(outcome, element)?;
        }
        Ok(())
    }

    fn share_verification_key(&self) -> PublicKey {
        self.keypair.public_key()
    }

    fn share_adaptors(&self) -> Vec<ASigS::AdaptorSignature> {
        self.storage.get_all_my_adaptors()
    }

    fn save_cp_verification_key(&mut self, cp_verification_key: PublicKey) {
        self.cp_verification_key = cp_verification_key;
    }

    fn save_cp_adaptors(&mut self, cp_adaptors: Vec<ASigS::AdaptorSignature>) {
        self.cp_adaptors = cp_adaptors;
    }

    fn verify_cp_adaptors(&self) -> bool {
        MyDlcComputation::<ASigS, CU>::verify_cp_adaptors(
            &self.cp_verification_key,
            &self.cp_adaptors,
            self.storage.get_all_elements_vec_ref(),
        )
    }

    fn update_cp_adaptors(&mut self) -> Result<(), Error> {
        self.storage.update_cp_adaptors(self.cp_adaptors.clone())
    }

    fn wait_attestation(&mut self) -> Result<(), Error> {
        self.oracle_attestation = self.oracle.get_event_attestation(0);

        // In future, here might be relevant adaptor optimization, ideally as some function eg. `has_winning_payout()` in `fun.rs`

        Ok(())
    }

    // If we are aware of event outcome, we can finalize winning DLC transaction which will be then broadcasted to the blockchain
    fn finalize_tx(&self) -> types::FinalizedTx<ASigS::Signature> {
        let outcome_element = self
            .storage
            .get_element(&self.oracle_attestation.outcome)
            .unwrap();

        let msg = common::fun::create_message(outcome_element.cet.as_bytes()).unwrap();

        #[cfg(feature = "ecdsa")]
        let my_sig = self.keypair.secret_key().sign_ecdsa(msg);
        #[cfg(feature = "schnorr")]
        let my_sig = self.keypair.sign_schnorr(msg);

        let cp_sig = ASigS::adapt(
            &outcome_element.cp_adaptor_signature.unwrap(),
            &self.oracle_attestation.attestation,
        );

        if self.controller_type == Offerer {
            types::FinalizedTx::<ASigS::Signature>::new(outcome_element.cet, my_sig, cp_sig)
        } else if self.controller_type == Accepter {
            types::FinalizedTx::<ASigS::Signature>::new(outcome_element.cet, cp_sig, my_sig)
        } else {
            // Fallback (or panic) if controller type is neither Offerer nor Accepter
            panic!("Unknown controller type: {:?}", self.controller_type);
        }
    }

    // fn broadcast_to_blockchain(self) -> Result<(), Error> {
    //     // ...placeholder...
    //     unimplemented!()
    // }
}
