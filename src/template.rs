//! Template substitution for rfor command strings.
//!
//! Recognized tokens:
//! - `{}`       -> current item (shell-quoted)
//! - `{#}`      -> 1-based job index
//! - `{.}`      -> item without last extension (shell-quoted)
//! - `{/}`      -> basename of item (shell-quoted)
//! - `{//}`     -> dirname of item (shell-quoted)
//! - `{/.}`     -> basename without extension (shell-quoted)
//! - `{name}`   -> current item if `name` matches the declared variable (bash-style)
//! - `{{`       -> literal `{`
//! - `}}`       -> literal `}`

use std::path::Path;

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

/// Remove the last extension from `item`: `photo.tar.gz` → `photo.tar`.
fn strip_extension(item: &str) -> String {
    let p = Path::new(item);
    match (p.parent(), p.file_stem()) {
        (Some(parent), Some(stem)) => {
            let parent_str = parent.to_string_lossy();
            let stem_str = stem.to_string_lossy();
            if parent_str.is_empty() {
                stem_str.into_owned()
            } else {
                format!("{}/{}", parent_str, stem_str)
            }
        }
        _ => item.to_string(),
    }
}

/// Basename of `item`: `/path/to/file.txt` → `file.txt`.
fn basename(item: &str) -> String {
    Path::new(item)
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| item.to_string())
}

/// Directory of `item`: `/path/to/file.txt` → `/path/to`.
fn dirname(item: &str) -> String {
    Path::new(item)
        .parent()
        .map(|p| {
            let s = p.to_string_lossy().into_owned();
            if s.is_empty() { ".".to_string() } else { s }
        })
        .unwrap_or_else(|| item.to_string())
}

/// Basename without extension: `/path/to/file.txt` → `file`.
fn basename_no_ext(item: &str) -> String {
    Path::new(item)
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| item.to_string())
}

/// Render a template by substituting `{}`, `{#}`, filename tokens, and
/// `{varname}` tokens.
///
/// When `var_name` is `Some("i")`, `{i}` is replaced with the shell-quoted
/// item. `{other}` passes through literally if `other` does not match.
/// `{}` and `{#}` always work regardless of `var_name`.
///
/// Filename tokens (`{.}`, `{/}`, `{//}`, `{/.}`) always operate on the raw
/// item and do NOT interact with named variables.
///
/// Iterates over `char` boundaries so multi-byte UTF-8 in the template
/// (e.g. `echo café {}`) passes through without corruption.
pub fn render(template: &str, item: &str, index: usize, var_name: Option<&str>) -> String {
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
                Some('/') => {
                    // Could be {/}, {//}, or {/.}
                    chars.next(); // consume first '/'
                    match chars.peek() {
                        Some('}') => {
                            // {/} — basename
                            chars.next();
                            out.push_str(&shell_quote(&basename(item)));
                        }
                        Some('/') => {
                            // {//} — dirname
                            chars.next(); // consume second '/'
                            if chars.peek() == Some(&'}') {
                                chars.next();
                                out.push_str(&shell_quote(&dirname(item)));
                            } else {
                                // Not {//}, emit literally
                                out.push_str("{//");
                            }
                        }
                        Some('.') => {
                            // {/.} — basename without extension
                            chars.next(); // consume '.'
                            if chars.peek() == Some(&'}') {
                                chars.next();
                                out.push_str(&shell_quote(&basename_no_ext(item)));
                            } else {
                                // Not {/.}, emit literally
                                out.push_str("{/.");
                            }
                        }
                        _ => {
                            // Just `{/` followed by something else — emit literally
                            out.push_str("{/");
                        }
                    }
                }
                Some('.') => {
                    // {.} — item without extension
                    chars.next(); // consume '.'
                    if chars.peek() == Some(&'}') {
                        chars.next();
                        out.push_str(&shell_quote(&strip_extension(item)));
                    } else {
                        // Not {.}, emit literally
                        out.push_str("{.");
                    }
                }
                Some(&c) if c.is_alphanumeric() || c == '_' => {
                    // Collect a potential named variable: {name}
                    let mut name = String::new();
                    while let Some(&c) = chars.peek() {
                        if c.is_alphanumeric() || c == '_' {
                            name.push(c);
                            chars.next();
                        } else {
                            break;
                        }
                    }
                    if chars.peek() == Some(&'}') {
                        chars.next(); // consume '}'
                        if var_name == Some(name.as_str()) {
                            out.push_str(&shell_quote(item));
                        } else {
                            // Not our variable — emit literally.
                            out.push('{');
                            out.push_str(&name);
                            out.push('}');
                        }
                    } else {
                        // No closing brace — emit literally.
                        out.push('{');
                        out.push_str(&name);
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

/// GNU parallel tokens that rfor does not yet support.
/// Each entry is (token, description).
const UNSUPPORTED_TOKENS: &[(&str, &str)] = &[
    ("{%}", "job slot number"),
];

/// Check a template for GNU parallel tokens that rfor doesn't support.
/// Emits one warning per unsupported token found (to stderr, once per run).
///
/// When `var_name` is set, tokens that happen to match the named variable
/// pattern are not warned about (they'll be substituted by `render`).
pub fn warn_unsupported_tokens(template: &str, var_name: Option<&str>) {
    for &(token, desc) in UNSUPPORTED_TOKENS {
        if template.contains(token) {
            // In bash-style mode, {.} etc. might look like a named var attempt
            // but these specific GNU parallel tokens are never valid var names,
            // so we always warn.
            let _ = var_name; // var_name doesn't suppress these warnings
            eprintln!(
                "rfor: warning: `{}` ({}) is not supported in rfor v1 \
                 and will be passed through literally",
                token, desc
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- GNU-parallel style (var_name = None) ---

    #[test]
    fn substitutes_item() {
        assert_eq!(render("echo {}", "hello", 1, None), "echo 'hello'");
    }

    #[test]
    fn substitutes_index() {
        assert_eq!(render("job {#}: {}", "x", 7, None), "job 7: 'x'");
    }

    #[test]
    fn literal_braces() {
        assert_eq!(render("{{}}", "x", 1, None), "{}");
    }

    #[test]
    fn quotes_special_chars() {
        assert_eq!(render("echo {}", "it's", 1, None), "echo 'it'\\''s'");
    }

    #[test]
    fn passes_through_unknown_brace() {
        assert_eq!(render("a{x}b", "i", 1, None), "a{x}b");
    }

    #[test]
    fn utf8_template_preserved() {
        assert_eq!(render("echo café {}", "ñ", 1, None), "echo café 'ñ'");
    }

    #[test]
    fn utf8_emoji_in_template() {
        assert_eq!(render("🚀 {}", "world", 1, None), "🚀 'world'");
    }

    // --- Named variable (bash-style) ---

    #[test]
    fn named_var_substitutes() {
        assert_eq!(render("echo {i}", "hello", 1, Some("i")), "echo 'hello'");
    }

    #[test]
    fn named_var_wrong_name_passes_through() {
        assert_eq!(render("echo {other}", "hello", 1, Some("i")), "echo {other}");
    }

    #[test]
    fn named_var_with_unnamed_placeholder() {
        assert_eq!(render("echo {} {i}", "x", 1, Some("i")), "echo 'x' 'x'");
    }

    #[test]
    fn named_var_with_index() {
        assert_eq!(render("job {#}: {i}", "x", 3, Some("i")), "job 3: 'x'");
    }

    #[test]
    fn named_var_multi_char() {
        assert_eq!(
            render("curl -sO {url}", "http://x.com", 1, Some("url")),
            "curl -sO 'http://x.com'"
        );
    }

    #[test]
    fn named_var_with_underscore() {
        assert_eq!(
            render("echo {my_var}", "val", 1, Some("my_var")),
            "echo 'val'"
        );
    }

    #[test]
    fn named_var_no_closing_brace_literal() {
        assert_eq!(render("echo {i", "x", 1, Some("i")), "echo {i");
    }

    // --- Filename tokens ---

    #[test]
    fn strip_ext_token() {
        assert_eq!(
            render("echo {.}", "photo.tar.gz", 1, None),
            "echo 'photo.tar'"
        );
    }

    #[test]
    fn strip_ext_no_extension() {
        assert_eq!(render("echo {.}", "README", 1, None), "echo 'README'");
    }

    #[test]
    fn basename_token() {
        assert_eq!(
            render("echo {/}", "/path/to/file.txt", 1, None),
            "echo 'file.txt'"
        );
    }

    #[test]
    fn dirname_token() {
        assert_eq!(
            render("echo {//}", "/path/to/file.txt", 1, None),
            "echo '/path/to'"
        );
    }

    #[test]
    fn dirname_token_no_dir() {
        assert_eq!(render("echo {//}", "file.txt", 1, None), "echo '.'");
    }

    #[test]
    fn basename_no_ext_token() {
        assert_eq!(
            render("echo {/.}", "/path/to/file.txt", 1, None),
            "echo 'file'"
        );
    }

    #[test]
    fn all_filename_tokens_together() {
        let t = "echo {} {.} {/} {//} {/.}";
        let result = render(t, "/a/b/c.tar.gz", 1, None);
        assert_eq!(
            result,
            "echo '/a/b/c.tar.gz' '/a/b/c.tar' 'c.tar.gz' '/a/b' 'c.tar'"
        );
    }
}
