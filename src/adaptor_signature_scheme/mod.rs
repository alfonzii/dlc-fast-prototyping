// src/adaptor_signature_scheme/mod.rs

use secp256k1_zkp::{Keypair, Message, PublicKey, SecretKey};

pub trait AdaptorSignatureScheme {
    type AdaptorSignature: Clone;
    type Signature;

    fn pre_sign(
        signing_keypair: &Keypair,
        message: &Message,
        anticipation_point: &PublicKey,
    ) -> Self::AdaptorSignature;

    fn pre_verify(
        verification_key: &PublicKey,
        message: &Message,
        anticipation_point: &PublicKey,
        adaptor_signature: &Self::AdaptorSignature,
    ) -> bool;

    fn adapt(
        adaptor_signature: &Self::AdaptorSignature,
        attestation: &SecretKey,
    ) -> Self::Signature;

    #[allow(dead_code)] // delete if used
    fn extract(
        signature: &Self::Signature,
        adaptor_signature: &Self::AdaptorSignature,
        anticipation_point: &PublicKey,
    ) -> types::Attestation;
}

mod ecdsa_zkp_adaptor;
mod schnorr_zkp_adaptor;

#[cfg(feature = "ecdsa")]
pub use ecdsa_zkp_adaptor::EcdsaAdaptorSignatureScheme;
#[cfg(feature = "schnorr")]
pub use schnorr_zkp_adaptor::SchnorrAdaptorSignatureScheme;

use crate::common::types;
