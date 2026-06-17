// src/common/types.rs

use std::fmt;
use std::num::NonZeroU64;
use secp256k1_zkp;
use secp256k1_zkp::{PublicKey, SecretKey};
use serde::{Deserialize, Serialize};

/// -- Aliases for outcome types --
pub type AnticipationPoint = PublicKey;
pub type Attestation = SecretKey;

// Other
pub type Cet = String; // Contract Execution Transaction (esentially not signed Tx)
pub type PayoutT = u64; // Integer type for payout values (in satoshis). If will be using Bitcoin API, then Amount will be correct type
pub type ParsedContract<O: Outcome> = Vec<(O, PayoutT)>; // Compiler checker says this might be implemented in future versions, so we leave it like this
                                                         // It could be made as Vec<PayoutT> if we would consider just OutcomeU32, but we made it more robust, so it could be implemented also with different types of outcomes (eg. OutcomeBinStr).

/// The final Bitcoin transaction or any other on-chain transaction type
/// that will be broadcasted after finalization.
pub struct FinalizedTx<Sig> {
    pub payload: Cet,
    pub offerer_sig: Sig,
    pub accepter_sig: Sig,
}

impl<Sig> FinalizedTx<Sig> {
    pub fn new(payload: Cet, offerer_sig: Sig, accepter_sig: Sig) -> Self {
        Self {
            payload,
            offerer_sig,
            accepter_sig,
        }
    }
}

pub struct MultisigFundAddress {
    pub offerer_pubkey: PublicKey,
    pub accepter_pubkey: PublicKey,
}

impl MultisigFundAddress {
    pub fn new(offerer_pubkey: PublicKey, accepter_pubkey: PublicKey) -> Self {
        Self {
            offerer_pubkey,
            accepter_pubkey,
        }
    }
}

// ------------------ Outcome trait and implementations ------------------
pub trait Outcome {
    type ValueType;

    /// Return the value of this Outcome.
    fn get_value(&self) -> Self::ValueType;

    /// Return bit at the given position.
    fn get_bit(&self, position: u8) -> bool;

    /// Return true if the outcome is zero.
    fn is_zero(&self) -> bool;

    /// Serialize this Outcome into bytes to store or transmit.
    fn serialize(&self) -> Vec<u8>;
}

// A simple integer-based outcome.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OutcomeU32 {
    value: u32,
}

impl Outcome for OutcomeU32 {
    type ValueType = u32;

    fn get_value(&self) -> u32 {
        self.value
    }

    fn get_bit(&self, position: u8) -> bool {
        debug_assert!(position < 32, "Position must be less than 32");
        (self.value >> position) & 1 == 1
    }

    fn is_zero(&self) -> bool {
        self.value == 0
    }

    fn serialize(&self) -> Vec<u8> {
        self.value.to_le_bytes().to_vec()
    }
}

impl From<u32> for OutcomeU32 {
    fn from(value: u32) -> Self {
        Self { value }
    }
}

impl From<OutcomeU32> for u32 {
    fn from(outcome: OutcomeU32) -> u32 {
        outcome.value
    }
}

// A string-based binary outcome
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutcomeBinStr {
    value: String,
}

impl Outcome for OutcomeBinStr {
    type ValueType = String;

    fn get_value(&self) -> String {
        self.value.clone()
    }

    fn get_bit(&self, position: u8) -> bool {
        debug_assert!(
            position < self.value.len() as u8,
            "Position must be less than the length of the string"
        );
        self.value.chars().nth(position as usize) == Some('1')
    }

    fn is_zero(&self) -> bool {
        self.value.chars().all(|c| c == '0')
    }

    fn serialize(&self) -> Vec<u8> {
        self.value.as_bytes().to_vec()
    }
}

impl TryFrom<String> for OutcomeBinStr {
    type Error = OutcomeBinStrParseError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        if let Some(invalid_char_at) = value.chars().position(|c| c != '0' && c != '1') {
            Err(OutcomeBinStrParseError { invalid_char_at })
        } else {
            Ok(Self { value })
        }
    }
}

#[derive(Debug)]
pub struct OutcomeBinStrParseError {
    invalid_char_at: usize,
}

impl From<OutcomeBinStr> for String {
    fn from(outcome: OutcomeBinStr) -> Self {
        outcome.value
    }
}

impl fmt::Display for OutcomeBinStrParseError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "invalid char at position {}", self.invalid_char_at)
    }
}

impl std::error::Error for OutcomeBinStrParseError {}

// ------------------ ContractInput and related structs ------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(try_from = "raw_contract_input::ContractInput")]
pub struct ContractInput {
    offer_collateral: NonZeroU64,  // Amount (btc cargo)
    accept_collateral: NonZeroU64, // Amount (btc cargo)
    fee_rate: NonZeroU64,
    contract_info: ContractInfo,
}

impl ContractInput {
    pub fn total_collateral(&self) -> NonZeroU64 {
        self.offer_collateral.checked_add(self.accept_collateral.into()).expect("this sum was already validated")
    }

    pub fn contract_info(&self) -> &ContractInfo {
        &self.contract_info
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ContractInfo {
    contract_descriptor: ContractDescriptor,
    oracle: OracleInput,
}

impl ContractInfo {
    pub fn contract_descriptor(&self) -> &ContractDescriptor {
        &self.contract_descriptor
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ContractDescriptor {
    payout_intervals: Vec<PayoutInterval>,
}

impl ContractDescriptor {
    pub fn payout_intervals(&self) -> &[PayoutInterval] {
        &self.payout_intervals
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PayoutInterval {
    payout_points: [PayoutPoint; 2],
}

impl PayoutInterval {
    pub fn payout_points(&self) -> &[PayoutPoint; 2] {
        &self.payout_points
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PayoutPoint {
    event_outcome: u32,
    outcome_payout: PayoutT, // Amount (btc cargo)
}

impl PayoutPoint {
    pub fn event_outcome(&self) -> u32 {
        self.event_outcome
    }

    pub fn outcome_payout(&self) -> PayoutT {
        self.outcome_payout
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(try_from = "raw_contract_input::OracleInput")]
pub struct OracleInput {
    public_key: PublicKey,
    event_id: String,
    nb_digits: u8,
}

mod raw_contract_input {
    use std::num::NonZeroU64;
    use serde::{Deserialize, Serialize};
    use secp256k1_zkp::PublicKey;
    use crate::common::error::ContractError;
    use crate::config::NB_DIGITS;
    use super::PayoutT;

    #[derive(Debug, Clone, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct ContractInput {
        pub offer_collateral: NonZeroU64,  // Amount (btc cargo)
        pub accept_collateral: NonZeroU64, // Amount (btc cargo)
        pub fee_rate: NonZeroU64,
        pub contract_info: ContractInfo,
    }

    impl TryFrom<ContractInput> for super::ContractInput {
        type Error = ContractError;

        fn try_from(value: ContractInput) -> Result<super::ContractInput, ContractError> {
            // 10: Input contract must be non-empty (handled by NonZeroU64)
            /*
            if self.offer_collateral == 0 || self.accept_collateral == 0 || self.fee_rate == 0 {
                return Err(ContractError::EmptyContract);
            }
            */

            // 6: Collateral must be valid unsigned integer (secured by u64)
            /*if self.offer_collateral < 0 || self.accept_collateral < 0 {
                return Err(ContractError::InvalidCollateral);
            }*/

            // 7: If feeRate > 25 * 250 => error
            if u64::from(value.fee_rate) > 25 * 250 {
                return Err(ContractError::TooHighFeeRate);
            }

            // Validate the rest
            let sum_collaterals = value.offer_collateral.checked_add(value.accept_collateral.into())
                .ok_or(ContractError::CollateralSumOverflow)?;

            Ok(super::ContractInput {
                offer_collateral: value.offer_collateral,
                accept_collateral: value.accept_collateral,
                fee_rate: value.fee_rate,
                contract_info: value.contract_info.validate(sum_collaterals)?,
            })
        }
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct ContractInfo {
        pub contract_descriptor: ContractDescriptor,
        pub oracle: OracleInput,
    }

    impl ContractInfo {
        pub fn validate(self, max_payout: NonZeroU64) -> Result<super::ContractInfo, ContractError> {
            let oracle: super::OracleInput = self.oracle.try_into()?;
            let contract_descriptor = self.contract_descriptor
                .validate(max_payout, oracle.nb_digits)?;
            Ok(super::ContractInfo {
                contract_descriptor,
                oracle,
            })
        }
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct ContractDescriptor {
        pub payout_intervals: Vec<PayoutInterval>,
    }

    impl ContractDescriptor {
        pub fn validate(self, max_payout: NonZeroU64, nb_digits: u8) -> Result<super::ContractDescriptor, ContractError> {
            // 9: must have at least one interval
            if self.payout_intervals.is_empty() {
                return Err(ContractError::MissingIntervals);
            }

            // 2^NB_DIGITS - 1 is expected final outcome
            let expected_final_outcome = (1 << nb_digits) - 1;

            // 2: First point of first interval must start at zero
            let first_interval = &self.payout_intervals[0];
            let first_pt = first_interval
                .payout_points
                .first()
                .ok_or(ContractError::InvalidIntervalPoints)?;
            if first_pt.event_outcome != 0 {
                return Err(ContractError::InvalidFirstOutcome);
            }

            // 1. Validate each interval, check continuity
            for w in self.payout_intervals.windows(2) {
                let end_of_this = w[0].payout_points[1].event_outcome;
                let start_of_next = w[1].payout_points[0].event_outcome;
                if end_of_this != start_of_next {
                    return Err(ContractError::NonContinuousIntervals);
                }
            }

            // 3: Last point must end on 2^NB_DIGITS - 1
            let last_interval = self.payout_intervals.last().unwrap();
            let last_pt = last_interval
                .payout_points
                .last()
                .ok_or(ContractError::InvalidIntervalPoints)?;
            if last_pt.event_outcome != expected_final_outcome {
                return Err(ContractError::OutcomeRangeMismatch);
            }

            let payout_intervals = self.payout_intervals.into_iter()
                .map(|interval| interval.validate(max_payout))
                .collect::<Result<Vec<_>, _>>()?;

            Ok(super::ContractDescriptor {
                payout_intervals,
            })
        }
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct PayoutInterval {
        pub payout_points: [PayoutPoint; 2],
    }

    impl PayoutInterval {
        pub fn validate(self, max_payout: NonZeroU64) -> Result<super::PayoutInterval, ContractError> {
            // 8: Each interval should have exactly 2 points
            /*
            if self.payout_points.len() != 2 {
                return Err(ContractError::InvalidIntervalPoints);
            }
            */
            // Validate each payout point
            let [p0, p1] = self.payout_points;
            let p0 = p0.validate(max_payout)?;
            let p1 = p1.validate(max_payout)?;

            Ok(super::PayoutInterval { payout_points: [p0, p1] })
        }
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct PayoutPoint {
        pub event_outcome: u32,
        pub outcome_payout: PayoutT, // Amount (btc cargo)
    }

    impl PayoutPoint {
        pub fn validate(self, max_payout: NonZeroU64) -> Result<super::PayoutPoint, ContractError> {
            // 5: outcomePayout must be non-negative (already guaranteed by u64)
            /*if self.outcome_payout < 0 {
                return Err(ContractError::NegativePayout);
            }*/

            // 4: outcomePayout <= sum of offerCollateral and acceptCollateral
            if self.outcome_payout > u64::from(max_payout) {
                return Err(ContractError::InvalidPayout);
            }
            Ok(super::PayoutPoint {
                event_outcome: self.event_outcome,
                outcome_payout: self.outcome_payout,
            })
        }
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct OracleInput {
        pub public_key: PublicKey,
        pub event_id: String,
        pub nb_digits: u8,
    }

    impl TryFrom<OracleInput> for super::OracleInput {
        type Error = ContractError;

        fn try_from(value: OracleInput) -> Result<Self, Self::Error> {
            // 11. nb_digits from contract must match NB_DIGITS constant
            if value.nb_digits != NB_DIGITS {
                return Err(ContractError::NbDigitsMismatch);
            }
            Ok(super::OracleInput {
                public_key: value.public_key,
                event_id: value.event_id,
                nb_digits: value.nb_digits,
            })
        }
    }
}
