//! CLI definition and arg parsing.

use clap::{ArgAction, Parser};

/// pfor: a parallel for-loop replacement with live output and a sticky progress bar.
#[derive(Parser, Debug)]
#[command(
    name = "pfor",
    version,
    about = "Parallel for-loop replacement with live output and a sticky progress bar",
    long_about = None,
    after_help = EXAMPLES,
    disable_help_flag = false,
    trailing_var_arg = true,
)]
pub struct Cli {
    /// Number of parallel jobs. 1 (default) = sequential. 0 = number of logical CPUs.
    #[arg(short = 'j', long = "jobs", default_value_t = 1, value_name = "N")]
    pub jobs: usize,

    /// Stop scheduling new jobs after the first failure (in-flight jobs still finish).
    #[arg(long = "halt-on-fail", action = ArgAction::SetTrue)]
    pub halt_on_fail: bool,

    /// Command template plus optional `:::` items or `::::` argfile.
    ///
    /// The first positional is the template (e.g. `'echo {}'`).
    /// Anything after a `:::` token becomes inline items.
    /// `:::: FILE` reads items from FILE (one per line).
    /// If neither separator is present, items are read from stdin.
    #[arg(value_name = "TEMPLATE [::: ARGS... | :::: FILE]", required = true)]
    pub rest: Vec<String>,
}

const EXAMPLES: &str = "\
Tokens in the template:
  {}    current item (shell-quoted automatically)
  {#}   1-based job index
  {{    literal '{'
  }}    literal '}'

Examples:
  # Inline args (::: form)
  pfor 'echo {}' ::: alpha beta gamma

  # Items from a file (:::: form)
  pfor -j 4 'curl -sO {}' :::: urls.txt

  # Items from stdin
  printf '%s\\n' a b c | pfor 'echo job {#}: {}'

  # Stop on first failure
  pfor --halt-on-fail 'flaky-cmd {}' ::: 1 2 3 4
";

/// Parsed result of splitting positional arguments into their components.
#[derive(Debug)]
pub struct SplitArgs {
    /// The command template string (e.g. `"echo {}"`).
    pub template: String,
    /// Inline items provided after `:::`, if any.
    pub inline_items: Option<Vec<String>>,
    /// Path to an argfile provided after `::::`, if any.
    pub argfile: Option<String>,
}

/// Split `rest` into template, inline items, and argfile.
///
/// Recognizes the `:::` and `::::` separator tokens. Returns an error
/// string suitable for printing to stderr if the args are malformed.
pub fn split_rest(rest: Vec<String>) -> Result<SplitArgs, String> {
    if rest.is_empty() {
        return Err("missing TEMPLATE argument".into());
    }
    let template = rest[0].clone();

    // Find first separator.
    let mut sep_idx: Option<usize> = None;
    let mut sep_kind: Option<&str> = None;
    for (i, a) in rest.iter().enumerate().skip(1) {
        if a == ":::" {
            sep_idx = Some(i);
            sep_kind = Some(":::");
            break;
        }
        if a == "::::" {
            sep_idx = Some(i);
            sep_kind = Some("::::");
            break;
        }
    }

    match (sep_idx, sep_kind) {
        (None, _) => {
            // Extras without a separator are not allowed; user might be
            // confused. If they passed more than just the template, error.
            if rest.len() > 1 {
                return Err(format!(
                    "unexpected positional arguments after TEMPLATE: {:?}. \
                     Did you forget `:::` or `::::`?",
                    &rest[1..]
                ));
            }
            Ok(SplitArgs { template, inline_items: None, argfile: None })
        }
        (Some(i), Some(":::")) => {
            let items: Vec<String> = rest[i + 1..].to_vec();
            if items.is_empty() {
                return Err("`:::` provided but no items followed it".into());
            }
            // Reject if a second separator sneaked into the items.
            if let Some(pos) = items.iter().position(|a| a == "::::" || a == ":::") {
                return Err(format!(
                    "mixed separators: found `{}` after `:::`. \
                     Use either `:::` or `::::`, not both.",
                    items[pos]
                ));
            }
            Ok(SplitArgs { template, inline_items: Some(items), argfile: None })
        }
        (Some(i), Some("::::")) => {
            let after = &rest[i + 1..];
            if after.len() != 1 {
                return Err(format!(
                    "`::::` expects exactly one FILE argument, got {}",
                    after.len()
                ));
            }
            Ok(SplitArgs { template, inline_items: None, argfile: Some(after[0].clone()) })
        }
        _ => unreachable!(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn split_inline() {
        let s =
            split_rest(vec!["echo {}".into(), ":::".into(), "a".into(), "b".into()]).unwrap();
        assert_eq!(s.template, "echo {}");
        assert_eq!(s.inline_items.unwrap(), vec!["a", "b"]);
        assert!(s.argfile.is_none());
    }

    #[test]
    fn split_file() {
        let s =
            split_rest(vec!["echo {}".into(), "::::".into(), "items.txt".into()]).unwrap();
        assert_eq!(s.template, "echo {}");
        assert!(s.inline_items.is_none());
        assert_eq!(s.argfile.unwrap(), "items.txt");
    }

    #[test]
    fn split_no_sep() {
        let s = split_rest(vec!["echo {}".into()]).unwrap();
        assert_eq!(s.template, "echo {}");
        assert!(s.inline_items.is_none());
        assert!(s.argfile.is_none());
    }

    #[test]
    fn extras_without_sep_err() {
        assert!(split_rest(vec!["echo {}".into(), "stray".into()]).is_err());
    }

    #[test]
    fn double_colons_empty_err() {
        assert!(split_rest(vec!["echo {}".into(), ":::".into()]).is_err());
    }

    #[test]
    fn mixed_separators_rejected() {
        let result = split_rest(vec![
            "echo {}".into(),
            ":::".into(),
            "a".into(),
            "::::".into(),
            "file.txt".into(),
        ]);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("mixed separators"));
    }
}
