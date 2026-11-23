use crate::messaging::error::{AppResult, ServerError};

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
    fn counts_characters_not_bytes() {
        // 3 characters, but 6 bytes
        assert!(required_text("ééé", "name", 3).is_ok());
    }
}
