use crate::common::{types, ContractInput, OutcomeU32, ParsedContract};
use crate::config::NB_OUTCOMES;
use crate::parser::Parser;

#[cfg(feature = "parallel-parser")]
use rayon::prelude::*;
use std::fs;
use std::io::Error;

pub struct SimpleOutU32Parser;

impl SimpleOutU32Parser {
    // Creates a Vec of length `len` where v[i] = init + i*inc

    #[cfg(not(feature = "parallel-parser"))]
    fn create_linear_seq(
        start_outcome: u32,
        len: u32,
        start_payout: types::PayoutT,
        step: f64,
    ) -> Vec<(types::OutcomeU32, types::PayoutT)> {
        let start_payout_f = start_payout as f64;
        (0..len)
            .map(|i| {
                (
                    OutcomeU32::from(start_outcome + i),
                    (start_payout_f + (i as f64 * step)).round() as u64,
                )
            })
            .collect()
    }

    #[cfg(feature = "parallel-parser")]
    fn create_linear_seq(
        start_outcome: u32,
        len: u32,
        start_payout: types::PayoutT,
        step: f64,
    ) -> Vec<(types::OutcomeU32, types::PayoutT)> {
        let start_payout_f = start_payout as f64;
        (0..len)
            .into_par_iter() // make a parallel iterator
            .map(|i| {
                (
                    OutcomeU32::from(start_outcome + i),
                    (start_payout_f + (i as f64 * step)).round() as u64,
                )
            })
            .collect()
    }

    #[cfg(not(feature = "parallel-parser"))]
    fn create_const_payout_vec(
        start_outcome: u32,
        interval_len: u32,
        constant_payout: types::PayoutT,
    ) -> Vec<(types::OutcomeU32, types::PayoutT)> {
        (0..interval_len)
            .map(|i| (OutcomeU32::from(start_outcome + i), constant_payout))
            .collect()
    }

    #[cfg(feature = "parallel-parser")]
    fn create_const_payout_vec(
        start_outcome: u32,
        interval_len: u32,
        constant_payout: types::PayoutT,
    ) -> Vec<(types::OutcomeU32, types::PayoutT)> {
        (0..interval_len)
            .into_par_iter()
            .map(|i| (OutcomeU32::from(start_outcome + i), constant_payout))
            .collect()
    }
}

impl Parser<types::OutcomeU32> for SimpleOutU32Parser {
    fn read_input(contract_path: &str) -> Result<ContractInput, Error> {
        // Read input file containing contract JSON into string
        let contract_input_str = fs::read_to_string(contract_path)?;

        // Deserialize contract input
        let contract_input: ContractInput = match serde_json::from_str(&contract_input_str) {
            Ok(ci) => ci,
            Err(e) => {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!("Error deserializing JSON: {}", e),
                ));
            }
        };
        Ok(contract_input)
    }

    fn parse_contract_input(
        contract_input: ContractInput,
    ) -> Result<ParsedContract<OutcomeU32>, Error> {
        // At this point, if we have reached here, we can safely assume that the contract is valid

        // Reserve capacity for final vector based on number of possible outcomes (we avoid reallocating by doing this)
        let mut parsed_contract =
            ParsedContract::<types::OutcomeU32>::with_capacity(NB_OUTCOMES as usize);

        // Parse contract intervals and create a vector of (outcome, payout) pairs
        for interval in contract_input
            .contract_info()
            .contract_descriptor()
            .payout_intervals()
        {
            // Get start and end points of the interval
            let start_point = &interval.payout_points()[0];
            let end_point = &interval.payout_points()[1];

            // Calculate interval length and payout difference
            let start_outcome = start_point.event_outcome();
            let end_outcome = end_point.event_outcome();
            assert!(
                end_outcome > start_outcome,
                "end outcome must be greater than start outcome"
            );
            let interval_len = end_outcome - start_outcome;

            let start_payout = start_point.outcome_payout();
            let end_payout = end_point.outcome_payout();
            let diff = (end_payout as i64) - (start_payout as i64);

            if diff == 0 {
                let repeated =
                    Self::create_const_payout_vec(start_outcome, interval_len, start_payout); // Might also use end_payout. It doesn't matter as both are same
                parsed_contract.extend(repeated.into_iter());
            } else {
                let step: f64 = (diff as f64) / (interval_len as f64);
                let linear_seq =
                    Self::create_linear_seq(start_outcome, interval_len, start_payout, step);
                parsed_contract.extend(linear_seq.into_iter());
            }
        }

        // Manually add the last interval's last point to the parsed contract, as we don't add into `repeated` last (outcome, payout) pair from interval
        let last_interval = contract_input
            .contract_info()
            .contract_descriptor()
            .payout_intervals()
            .last()
            .unwrap();
        let last_point = last_interval.payout_points().last().unwrap();
        parsed_contract.push((
            OutcomeU32::from(last_point.event_outcome()),
            last_point.outcome_payout(),
        ));

        Ok(parsed_contract)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn validate_parse_test_contracts(json_input: &str) {
        let contract_input: ContractInput =
            serde_json::from_str(json_input).expect("Serde JSON should not fail at this point!");
        assert!(contract_input.validate().is_err());
    }

    fn assert_deserialization_error(json_str: &str) {
        let parse_result = serde_json::from_str::<ContractInput>(json_str);
        assert!(
            parse_result.is_err(),
            "Expected an error, but got Ok(...) instead"
        );
    }

    // Test cases for invalid JSON

    #[test]
    fn test_deserialization_of_empty_contract() {
        assert_deserialization_error(include_str!(
            "../../input_contracts/test_contracts/empty_contract_input.json"
        ));
    }

    #[test]
    fn test_deserialization_of_negative_payout() {
        assert_deserialization_error(include_str!(
            "../../input_contracts/test_contracts/negative_payout_input.json"
        ));
    }

    // Test cases for invalid contracts (but otherwise valid JSON)

    #[test]
    fn test_invalid_collateral() {
        validate_parse_test_contracts(include_str!(
            "../../input_contracts/test_contracts/invalid_collateral_input.json"
        ));
    }

    #[test]
    fn test_excessive_feerate() {
        validate_parse_test_contracts(include_str!(
            "../../input_contracts/test_contracts/excessive_feerate_input.json"
        ));
    }

    #[test]
    fn test_last_outcome_not_final() {
        validate_parse_test_contracts(include_str!(
            "../../input_contracts/test_contracts/last_outcome_not_final_input.json"
        ));
    }

    #[test]
    fn test_invalid_payout_level() {
        validate_parse_test_contracts(include_str!(
            "../../input_contracts/test_contracts/invalid_payout_level_input.json"
        ));
    }

    #[test]
    fn test_no_intervals() {
        validate_parse_test_contracts(include_str!(
            "../../input_contracts/test_contracts/no_intervals_input.json"
        ));
    }

    #[test]
    fn test_invalid_interval_points() {
        validate_parse_test_contracts(include_str!(
            "../../input_contracts/test_contracts/invalid_interval_points_input.json"
        ));
    }

    #[test]
    fn test_nonzero_first_point() {
        validate_parse_test_contracts(include_str!(
            "../../input_contracts/test_contracts/nonzero_first_point_input.json"
        ));
    }

    #[test]
    fn test_non_continuous() {
        validate_parse_test_contracts(include_str!(
            "../../input_contracts/test_contracts/noncontinuous_input.json"
        ));
    }
}
