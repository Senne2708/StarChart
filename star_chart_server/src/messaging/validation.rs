use crate::messaging::{
    error::{AppResult, ServerError},
    protocol::ActivityKind,
};

/// The most stars that can be awarded, taken away or cashed in at once
pub const MAX_STARS_AT_ONCE: i16 = 100;

/// Checks the star change has the right sign for its kind and isn't too big.
/// Rewards add stars; take-aways and cash-ins remove them.
pub fn validate_star_change(kind: ActivityKind, star_diff: i16) -> AppResult<()> {
    let (valid, rule) = match kind {
        ActivityKind::Reward => ((1..=MAX_STARS_AT_ONCE).contains(&star_diff), "between 1 and"),
        ActivityKind::TakeAway | ActivityKind::CashIn => {
            ((-MAX_STARS_AT_ONCE..=-1).contains(&star_diff), "between -1 and -")
        }
    };
    if !valid {
        return Err(ServerError::Validation(format!(
            "star_diff for {} must be {rule}{MAX_STARS_AT_ONCE}",
            kind_name(kind)
        )));
    }
    Ok(())
}

fn kind_name(kind: ActivityKind) -> &'static str {
    match kind {
        ActivityKind::Reward => "reward",
        ActivityKind::TakeAway => "take_away",
        ActivityKind::CashIn => "cash_in",
    }
}

/// Trims `value` and checks it is not empty and at most `max` characters.
pub fn required_text<'a>(value: &'a str, field: &str, max: usize) -> AppResult<&'a str> {
    let value = value.trim();
    if value.is_empty() {
        return Err(ServerError::Validation(format!(
            "{field} must not be empty"
        )));
    }
    if value.chars().count() > max {
        return Err(ServerError::Validation(format!(
            "{field} must be at most {max} characters"
        )));
    }
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trims_whitespace() {
        assert_eq!(required_text("  Alex  ", "name", 10).unwrap(), "Alex");
    }

    #[test]
    fn rejects_empty_and_blank() {
        for value in ["", "   ", "\t\n"] {
            let err = required_text(value, "name", 10).unwrap_err();
            assert_eq!(err.to_string(), "name must not be empty");
        }
    }

    #[test]
    fn rejects_too_long() {
        assert!(required_text("abc", "name", 3).is_ok());

        let err = required_text("abcd", "name", 3).unwrap_err();
        assert_eq!(err.to_string(), "name must be at most 3 characters");
    }

    #[test]
    fn rewards_must_be_positive() {
        assert!(validate_star_change(ActivityKind::Reward, 1).is_ok());
        assert!(validate_star_change(ActivityKind::Reward, 100).is_ok());

        for diff in [0, -1, 101] {
            assert!(validate_star_change(ActivityKind::Reward, diff).is_err(), "for {diff}");
        }
    }

    #[test]
    fn take_aways_and_cash_ins_must_be_negative() {
        for kind in [ActivityKind::TakeAway, ActivityKind::CashIn] {
            assert!(validate_star_change(kind, -1).is_ok());
            assert!(validate_star_change(kind, -100).is_ok());

            for diff in [0, 1, -101] {
                assert!(validate_star_change(kind, diff).is_err(), "for {kind:?} {diff}");
            }
        }

        let err = validate_star_change(ActivityKind::CashIn, 3).unwrap_err();
        assert_eq!(err.to_string(), "star_diff for cash_in must be between -1 and -100");
    }

    #[test]
    fn counts_characters_not_bytes() {
        // 3 characters, but 6 bytes
        assert!(required_text("ééé", "name", 3).is_ok());
    }
}
