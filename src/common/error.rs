// src/common/error.rs

#[derive(Debug)]
pub enum ContractError {
    InvalidFirstOutcome,
    MissingIntervals,
    InvalidIntervalPoints,
    NonContinuousIntervals,
    OutcomeRangeMismatch,
    CollateralSumOverflow,
    InvalidPayout,
    TooHighFeeRate,
    NbDigitsMismatch,
}

impl std::fmt::Display for ContractError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ContractError::InvalidFirstOutcome => write!(f, "First interval must start at 0"),
            ContractError::MissingIntervals => write!(f, "No intervals in contract descriptor"),
            ContractError::InvalidIntervalPoints => {
                write!(f, "Each payout interval must contain exactly 2 points")
            }
            ContractError::NonContinuousIntervals => {
                write!(f, "Intervals are not continuous in eventOutcome")
            }
            ContractError::OutcomeRangeMismatch => {
                write!(f, "Last outcome not matching 2^NB_DIGITS - 1")
            }
            ContractError::CollateralSumOverflow => {
                write!(f, "The sum of collaterals exceeds 2^64 - 1")
            }
            ContractError::InvalidPayout => write!(
                f,
                "outcomePayout cannot exceed sum of offerCollateral + acceptCollateral"
            ),
            ContractError::TooHighFeeRate => write!(f, "feeRate too high (> 25 * 250)"),
            ContractError::NbDigitsMismatch => {
                write!(f, "nb_digits in input does not match NB_DIGITS constant")
            }
        }
    }
}

impl std::error::Error for ContractError {}
