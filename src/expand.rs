//! Brace expansion for item lists.
//!
//! Recognizes `{N..M}` and `{N..M..STEP}` patterns in items:
//! - Numeric: `{1..10}` → 1,2,...,10. `{10..1}` → 10,9,...,1
//! - Numeric with step: `{1..10..2}` → 1,3,5,7,9
//! - Zero-padded: `{01..05}` → 01,02,03,04,05
//! - Alphabetic: `{a..z}` → a,b,...,z

/// Expand brace expressions in a list of items.
///
/// Each item is checked for `{N..M}` or `{N..M..STEP}` patterns.
/// Items that don't match are passed through unchanged.
/// Maximum number of items a single brace expansion can produce.
/// Prevents `{1..999999999}` from consuming all memory.
const MAX_EXPANSION: usize = 1_000_000;

pub fn expand_items(items: Vec<String>) -> Result<Vec<String>, String> {
    let mut result = Vec::new();
    for item in items {
        if let Some(expanded) = try_expand(&item) {
            if expanded.len() > MAX_EXPANSION {
                return Err(format!(
                    "brace expansion `{}` would produce {} items (max {})",
                    item,
                    expanded.len(),
                    MAX_EXPANSION
                ));
            }
            result.extend(expanded);
        } else {
            result.push(item);
        }
    }
    Ok(result)
}

/// Try to expand a single item as a brace expression.
/// Returns `None` if the item is not a brace expression.
fn try_expand(item: &str) -> Option<Vec<String>> {
    // Must be exactly `{...}` with no other content.
    let trimmed = item.trim();
    if !trimmed.starts_with('{') || !trimmed.ends_with('}') {
        return None;
    }
    let inner = &trimmed[1..trimmed.len() - 1];

    // Split on ".." — we expect 2 or 3 parts.
    let parts: Vec<&str> = inner.split("..").collect();
    match parts.len() {
        2 => expand_range(parts[0], parts[1], None),
        3 => expand_range(parts[0], parts[1], Some(parts[2])),
        _ => None,
    }
}

/// Expand a range expression with optional step.
fn expand_range(start_s: &str, end_s: &str, step_s: Option<&str>) -> Option<Vec<String>> {
    // Try alphabetic first (single chars).
    if start_s.len() == 1
        && end_s.len() == 1
        && start_s.chars().next()?.is_ascii_alphabetic()
        && end_s.chars().next()?.is_ascii_alphabetic()
    {
        let start = start_s.chars().next()? as u8;
        let end = end_s.chars().next()? as u8;
        let step: u8 = match step_s {
            Some(s) => s.parse().ok()?,
            None => 1,
        };
        if step == 0 {
            return None;
        }
        let mut result = Vec::new();
        if start <= end {
            let mut c = start;
            while c <= end {
                result.push(String::from(c as char));
                c = match c.checked_add(step) {
                    Some(v) => v,
                    None => break,
                };
            }
        } else {
            let mut c = start;
            while c >= end {
                result.push(String::from(c as char));
                c = match c.checked_sub(step) {
                    Some(v) => v,
                    None => break,
                };
            }
        }
        return Some(result);
    }

    // Numeric range.
    let start: i64 = start_s.parse().ok()?;
    let end: i64 = end_s.parse().ok()?;
    let step: i64 = match step_s {
        Some(s) => {
            let v: i64 = s.parse().ok()?;
            if v == 0 {
                return None;
            }
            v.abs() // step direction is determined by start vs end
        }
        None => 1,
    };

    // Detect zero-padding: if either start or end has leading zeros.
    let pad_width = zero_pad_width(start_s).max(zero_pad_width(end_s));

    // Estimate size and guard against enormous expansions.
    let count = ((start - end).unsigned_abs() / step as u64) + 1;
    if count > MAX_EXPANSION as u64 {
        // Return a large vec that will be caught by the caller's check.
        // (We don't allocate it — we return enough items to trigger the error.)
        return Some(vec!["__overflow__".to_string(); MAX_EXPANSION + 1]);
    }

    let mut result = Vec::with_capacity(count as usize);
    if start <= end {
        let mut n = start;
        while n <= end {
            result.push(format_padded(n, pad_width));
            n += step;
        }
    } else {
        let mut n = start;
        while n >= end {
            result.push(format_padded(n, pad_width));
            n -= step;
        }
    }

    Some(result)
}

/// Detect zero-padding width from a numeric string.
/// `"05"` → 2, `"5"` → 0, `"001"` → 3, `"-05"` → 2 (ignoring sign).
fn zero_pad_width(s: &str) -> usize {
    let digits = s.trim_start_matches('-');
    if digits.len() > 1 && digits.starts_with('0') {
        digits.len()
    } else {
        0
    }
}

/// Format a number with optional zero-padding.
fn format_padded(n: i64, width: usize) -> String {
    if width > 0 {
        if n >= 0 {
            format!("{:0>width$}", n, width = width)
        } else {
            format!("-{:0>width$}", -n, width = width.saturating_sub(1))
        }
    } else {
        n.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numeric_ascending() {
        assert_eq!(
            expand_items(vec!["{1..5}".into()]).unwrap(),
            vec!["1", "2", "3", "4", "5"]
        );
    }

    #[test]
    fn numeric_descending() {
        assert_eq!(
            expand_items(vec!["{5..1}".into()]).unwrap(),
            vec!["5", "4", "3", "2", "1"]
        );
    }

    #[test]
    fn numeric_with_step() {
        assert_eq!(
            expand_items(vec!["{1..10..2}".into()]).unwrap(),
            vec!["1", "3", "5", "7", "9"]
        );
    }

    #[test]
    fn zero_padded() {
        assert_eq!(
            expand_items(vec!["{01..05}".into()]).unwrap(),
            vec!["01", "02", "03", "04", "05"]
        );
    }

    #[test]
    fn zero_padded_wide() {
        assert_eq!(
            expand_items(vec!["{001..003}".into()]).unwrap(),
            vec!["001", "002", "003"]
        );
    }

    #[test]
    fn alphabetic_ascending() {
        assert_eq!(
            expand_items(vec!["{a..e}".into()]).unwrap(),
            vec!["a", "b", "c", "d", "e"]
        );
    }

    #[test]
    fn alphabetic_descending() {
        assert_eq!(
            expand_items(vec!["{e..a}".into()]).unwrap(),
            vec!["e", "d", "c", "b", "a"]
        );
    }

    #[test]
    fn non_brace_passthrough() {
        assert_eq!(
            expand_items(vec!["hello".into(), "world".into()]).unwrap(),
            vec!["hello", "world"]
        );
    }

    #[test]
    fn mixed_items() {
        assert_eq!(
            expand_items(vec!["prefix".into(), "{1..3}".into(), "suffix".into()]).unwrap(),
            vec!["prefix", "1", "2", "3", "suffix"]
        );
    }

    #[test]
    fn single_value_range() {
        assert_eq!(expand_items(vec!["{5..5}".into()]).unwrap(), vec!["5"]);
    }

    #[test]
    fn enormous_expansion_rejected() {
        let result = expand_items(vec!["{1..9999999}".into()]);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("max"));
    }
}
