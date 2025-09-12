//! Simple sort parser utility shared by list endpoints/queries.
//!
//! Convention:
//! - input: Some("field") => ASC
//! - input: Some("-field") => DESC
//! - input: None or unknown field => `default_col` with `default_desc`

/// Parse sort string into (column, desc) tuple.
/// - `input`: optional string like `created_at` or `-created_at`
/// - `default_col`: used when input is None or not in allowed
/// - `default_desc`: true for DESC, false for ASC
/// - `allowed`: whitelist of acceptable column names (lowercase)
#[must_use]
pub fn parse_sort(
    input: Option<String>,
    default_col: &str,
    default_desc: bool,
    allowed: &[&str],
) -> (String, bool) {
    let s = match input {
        Some(s) if !s.trim().is_empty() => s.trim().to_string(),
        _ => return (default_col.to_string(), default_desc),
    };

    let (raw_col, desc) = s
        .strip_prefix('-')
        .map_or((s.as_str(), false), |rest| (rest.trim(), true));

    let col = raw_col.to_lowercase();
    if allowed.contains(&col.as_str()) {
        (col, desc)
    } else {
        (default_col.to_string(), default_desc)
    }
}

#[cfg(test)]
mod tests {
    use super::parse_sort;

    #[test]
    fn test_parse_sort_basic() {
        let allowed = ["created_at", "updated_at", "title"];
        assert_eq!(
            parse_sort(None, "created_at", true, &allowed),
            ("created_at".into(), true)
        );
        assert_eq!(
            parse_sort(Some("title".into()), "created_at", true, &allowed),
            ("title".into(), false)
        );
        assert_eq!(
            parse_sort(Some("-title".into()), "created_at", true, &allowed),
            ("title".into(), true)
        );
        assert_eq!(
            parse_sort(Some("unknown".into()), "created_at", true, &allowed),
            ("created_at".into(), true)
        );
    }
}
