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
///
/// Iterates over `char` boundaries so multi-byte UTF-8 in the template
/// (e.g. `echo café {}`) passes through without corruption.
pub fn render(template: &str, item: &str, index: usize) -> String {
    let mut out = String::with_capacity(template.len() + item.len());
    let mut chars = template.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '{' {
            match chars.peek() {
                Some('{') => {
                    chars.next();
                    out.push('{');
                }
                Some('}') => {
                    chars.next();
                    out.push_str(&shell_quote(item));
                }
                Some('#') => {
                    // Peek two ahead: need `#` then `}`.
                    chars.next(); // consume '#'
                    if chars.peek() == Some(&'}') {
                        chars.next(); // consume '}'
                        out.push_str(&index.to_string());
                    } else {
                        // Not `{#}`, emit literally.
                        out.push('{');
                        out.push('#');
                    }
                }
                _ => out.push('{'),
            }
        } else if ch == '}' {
            if chars.peek() == Some(&'}') {
                chars.next();
                out.push('}');
            } else {
                out.push('}');
            }
        } else {
            out.push(ch);
        }
    }
    out
}

/// GNU parallel tokens that pfor does not support in v1.
/// Each entry is (token, description).
const UNSUPPORTED_TOKENS: &[(&str, &str)] = &[
    ("{.}", "item without extension"),
    ("{/}", "basename of item"),
    ("{//}", "dirname of item"),
    ("{/.}", "basename without extension"),
    ("{%}", "job slot number"),
];

/// Check a template for GNU parallel tokens that pfor doesn't support.
/// Emits one warning per unsupported token found (to stderr, once per run).
pub fn warn_unsupported_tokens(template: &str) {
    for &(token, desc) in UNSUPPORTED_TOKENS {
        if template.contains(token) {
            eprintln!(
                "pfor: warning: `{}` ({}) is not supported in pfor v1 \
                 and will be passed through literally",
                token, desc
            );
        }
    }
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

    #[test]
    fn utf8_template_preserved() {
        assert_eq!(render("echo café {}", "ñ", 1), "echo café 'ñ'");
    }

    #[test]
    fn utf8_emoji_in_template() {
        assert_eq!(render("🚀 {}", "world", 1), "🚀 'world'");
    }
}
