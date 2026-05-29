//! Template substitution for pfor command strings.
//!
//! Recognized tokens:
//! - `{}`  -> current item (shell-quoted)
//! - `{#}` -> 1-based job index
//! - `{{`  -> literal `{`
//! - `}}`  -> literal `}`

/// POSIX shell-quote a string by wrapping in single quotes and escaping
/// embedded single quotes via `'\''`.
pub fn shell_quote(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('\'');
    for ch in s.chars() {
        if ch == '\'' {
            out.push_str("'\\''");
        } else {
            out.push(ch);
        }
    }
    out.push('\'');
    out
}

/// Render a template by substituting `{}` and `{#}` tokens.
pub fn render(template: &str, item: &str, index: usize) -> String {
    let bytes = template.as_bytes();
    let mut out = String::with_capacity(template.len() + item.len());
    let mut i = 0;
    while i < bytes.len() {
        let b = bytes[i];
        if b == b'{' {
            if i + 1 < bytes.len() && bytes[i + 1] == b'{' {
                out.push('{');
                i += 2;
                continue;
            }
            if i + 1 < bytes.len() && bytes[i + 1] == b'}' {
                out.push_str(&shell_quote(item));
                i += 2;
                continue;
            }
            if i + 2 < bytes.len() && bytes[i + 1] == b'#' && bytes[i + 2] == b'}' {
                out.push_str(&index.to_string());
                i += 3;
                continue;
            }
            out.push('{');
            i += 1;
            continue;
        }
        if b == b'}' {
            if i + 1 < bytes.len() && bytes[i + 1] == b'}' {
                out.push('}');
                i += 2;
                continue;
            }
            out.push('}');
            i += 1;
            continue;
        }
        out.push(b as char);
        i += 1;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn substitutes_item() {
        assert_eq!(render("echo {}", "hello", 1), "echo 'hello'");
    }

    #[test]
    fn substitutes_index() {
        assert_eq!(render("job {#}: {}", "x", 7), "job 7: 'x'");
    }

    #[test]
    fn literal_braces() {
        assert_eq!(render("{{}}", "x", 1), "{}");
    }

    #[test]
    fn quotes_special_chars() {
        assert_eq!(render("echo {}", "it's", 1), "echo 'it'\\''s'");
    }

    #[test]
    fn passes_through_unknown_brace() {
        assert_eq!(render("a{x}b", "i", 1), "a{x}b");
    }
}
