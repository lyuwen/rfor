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

    /// Print the commands that would be executed without running them.
    #[arg(long = "dry-run", action = ArgAction::SetTrue)]
    pub dry_run: bool,

    /// Buffer each job's output and print it as a block when the job completes.
    /// Prevents interleaving when running parallel jobs.
    #[arg(long = "group", action = ArgAction::SetTrue)]
    pub group: bool,

    /// Retry failed jobs up to N times before counting as a failure.
    #[arg(long = "retries", default_value_t = 0, value_name = "N")]
    pub retries: usize,

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
  {}       current item (shell-quoted automatically)
  {#}      1-based job index
  {.}      item without extension (photo.tar.gz → photo.tar)
  {/}      basename of item (/path/to/file.txt → file.txt)
  {//}     dirname of item (/path/to/file.txt → /path/to)
  {/.}     basename without extension (/path/to/file.txt → file)
  {var}    named variable (bash-style only, matches declared name)
  {{       literal '{'
  }}       literal '}'

Examples:
  # GNU parallel style — inline args
  pfor 'echo {}' ::: alpha beta gamma

  # GNU parallel style — items from a file
  pfor -j 4 'curl -sO {}' :::: urls.txt

  # GNU parallel style — items from stdin
  printf '%s\\n' a b c | pfor 'echo job {#}: {}'

  # Filename tokens
  pfor 'echo {.}' ::: photo.tar.gz doc.pdf
  pfor 'echo {/} is in {//}' ::: /path/to/file.txt

  # Bash for-loop style — inline items
  pfor i in a b c -- echo {i}

  # Bash for-loop style — with file
  pfor i in :::: urls.txt -- curl -sO {i}

  # Bash for-loop style — stdin
  cat items.txt | pfor i -- echo {i}

  # Bash for-loop style — with flags
  pfor -j 4 i in a b c -- echo {i}

  # Dry run — see commands without executing
  pfor --dry-run 'echo {}' ::: alpha beta gamma

  # Group output — prevent interleaving with parallel jobs
  pfor -j 4 --group 'make -C {}' ::: proj1 proj2 proj3

  # Retry failed jobs up to 3 times
  pfor --retries 3 'curl -sfO {}' ::: url1 url2 url3

  # Stop on first failure
  pfor --halt-on-fail 'flaky-cmd {}' ::: 1 2 3 4
";

/// Parsed result of splitting positional arguments into their components.
#[derive(Debug)]
pub struct SplitArgs {
    /// The command template string (e.g. `"echo {}"`).
    pub template: String,
    /// Inline items provided after `:::` or between `in` and `--`, if any.
    pub inline_items: Option<Vec<String>>,
    /// Path to an argfile provided after `::::`, if any.
    pub argfile: Option<String>,
    /// Named variable for bash-style syntax (e.g. `i` in `pfor i in ... -- ...`).
    /// `None` for GNU-parallel style.
    pub var_name: Option<String>,
}

/// Split `rest` into template, inline items, argfile, and optional variable name.
///
/// Detects two syntax families:
///
/// **Bash-style:** `VAR in ITEMS -- COMMAND` or `VAR -- COMMAND` (stdin).
/// The variable name must not contain `{}` or `{#}` (those signal GNU-parallel).
///
/// **GNU-parallel style:** `'TEMPLATE' ::: ARGS` / `:::: FILE` / stdin.
///
/// Returns an error string suitable for printing to stderr if the args are malformed.
pub fn split_rest(rest: Vec<String>) -> Result<SplitArgs, String> {
    if rest.is_empty() {
        return Err("missing TEMPLATE argument".into());
    }

    // Detection: if rest[0] contains {} or {#}, it's a GNU-parallel template.
    let first = &rest[0];
    let looks_like_template = first.contains("{}") || first.contains("{#}");

    if !looks_like_template && rest.len() >= 2 {
        if rest[1] == "in" {
            return parse_bash_with_items(&rest);
        }
        if rest[1] == "--" {
            return parse_bash_stdin(&rest);
        }
    }

    // Fall through to GNU-parallel parsing.
    parse_gnu_parallel(rest)
}

/// Validate that a variable name is a valid identifier: non-empty, starts with
/// a letter or underscore, contains only `[a-zA-Z0-9_]`.
fn validate_var_name(name: &str) -> Result<(), String> {
    if name.is_empty() {
        return Err("variable name cannot be empty".into());
    }
    let first = name.chars().next().unwrap();
    if !first.is_ascii_alphabetic() && first != '_' {
        return Err(format!(
            "invalid variable name `{}`: must start with a letter or underscore",
            name
        ));
    }
    if let Some(bad) = name.chars().find(|c| !c.is_ascii_alphanumeric() && *c != '_') {
        return Err(format!(
            "invalid variable name `{}`: contains invalid character `{}`",
            name, bad
        ));
    }
    Ok(())
}

/// Parse bash-style with items: `VAR in ITEM... -- COMMAND...`
///
/// Items between `in` and `--` become inline_items, unless they are
/// `:::: FILE`, in which case they become an argfile reference.
fn parse_bash_with_items(rest: &[String]) -> Result<SplitArgs, String> {
    let var_name = rest[0].clone();
    validate_var_name(&var_name)?;

    // Find the `--` separator.
    let sep_pos = rest.iter().position(|a| a == "--");
    let sep_pos = match sep_pos {
        Some(p) if p > 2 => p, // must be after "VAR in ..."
        Some(2) => {
            return Err(format!(
                "no items between `in` and `--`. \
                 Did you mean `pfor {} -- COMMAND` to read from stdin?",
                var_name
            ));
        }
        _ => {
            return Err(format!(
                "missing `--` separator. Bash-style syntax: \
                 pfor {} in ITEMS -- COMMAND",
                var_name
            ));
        }
    };

    let items_slice = &rest[2..sep_pos];
    let cmd_words = &rest[sep_pos + 1..];
    if cmd_words.is_empty() {
        return Err("missing command after `--`".into());
    }
    let template = cmd_words.join(" ");

    // Check if items are actually an argfile reference: `:::: FILE`
    if items_slice.len() == 2 && items_slice[0] == "::::" {
        return Ok(SplitArgs {
            template,
            inline_items: None,
            argfile: Some(items_slice[1].clone()),
            var_name: Some(var_name),
        });
    }

    // Check for stray separators in items.
    if let Some(pos) = items_slice.iter().position(|a| a == ":::") {
        return Err(format!(
            "unexpected `:::` in bash-style items at position {}. \
             Use `:::` only with GNU-parallel syntax.",
            pos + 2
        ));
    }
    if items_slice.len() == 1 && items_slice[0] == "::::" {
        return Err("`::::` expects a FILE argument after it".into());
    }

    Ok(SplitArgs {
        template,
        inline_items: Some(items_slice.to_vec()),
        argfile: None,
        var_name: Some(var_name),
    })
}

/// Parse bash-style with stdin: `VAR -- COMMAND...`
fn parse_bash_stdin(rest: &[String]) -> Result<SplitArgs, String> {
    let var_name = rest[0].clone();
    validate_var_name(&var_name)?;
    let cmd_words = &rest[2..];
    if cmd_words.is_empty() {
        return Err("missing command after `--`".into());
    }
    let template = cmd_words.join(" ");

    Ok(SplitArgs {
        template,
        inline_items: None,
        argfile: None,
        var_name: Some(var_name),
    })
}

/// Parse GNU-parallel style: `TEMPLATE [::: ARGS... | :::: FILE]`
fn parse_gnu_parallel(rest: Vec<String>) -> Result<SplitArgs, String> {
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
            if rest.len() > 1 {
                return Err(format!(
                    "unexpected positional arguments after TEMPLATE: {:?}. \
                     Did you forget `:::` or `::::`?",
                    &rest[1..]
                ));
            }
            Ok(SplitArgs {
                template,
                inline_items: None,
                argfile: None,
                var_name: None,
            })
        }
        (Some(i), Some(":::")) => {
            let items: Vec<String> = rest[i + 1..].to_vec();
            if items.is_empty() {
                return Err("`:::` provided but no items followed it".into());
            }
            if let Some(pos) = items.iter().position(|a| a == "::::" || a == ":::") {
                return Err(format!(
                    "mixed separators: found `{}` after `:::`. \
                     Use either `:::` or `::::`, not both.",
                    items[pos]
                ));
            }
            Ok(SplitArgs {
                template,
                inline_items: Some(items),
                argfile: None,
                var_name: None,
            })
        }
        (Some(i), Some("::::")) => {
            let after = &rest[i + 1..];
            if after.len() != 1 {
                return Err(format!(
                    "`::::` expects exactly one FILE argument, got {}",
                    after.len()
                ));
            }
            Ok(SplitArgs {
                template,
                inline_items: None,
                argfile: Some(after[0].clone()),
                var_name: None,
            })
        }
        _ => unreachable!(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- GNU-parallel style (existing, updated for var_name field) ---

    #[test]
    fn split_inline() {
        let s =
            split_rest(vec!["echo {}".into(), ":::".into(), "a".into(), "b".into()]).unwrap();
        assert_eq!(s.template, "echo {}");
        assert_eq!(s.inline_items.unwrap(), vec!["a", "b"]);
        assert!(s.argfile.is_none());
        assert!(s.var_name.is_none());
    }

    #[test]
    fn split_file() {
        let s =
            split_rest(vec!["echo {}".into(), "::::".into(), "items.txt".into()]).unwrap();
        assert_eq!(s.template, "echo {}");
        assert!(s.inline_items.is_none());
        assert_eq!(s.argfile.unwrap(), "items.txt");
        assert!(s.var_name.is_none());
    }

    #[test]
    fn split_no_sep() {
        let s = split_rest(vec!["echo {}".into()]).unwrap();
        assert_eq!(s.template, "echo {}");
        assert!(s.inline_items.is_none());
        assert!(s.argfile.is_none());
        assert!(s.var_name.is_none());
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

    // --- Bash-style syntax ---

    #[test]
    fn bash_inline_items() {
        let s = split_rest(vec![
            "i".into(), "in".into(), "a".into(), "b".into(), "c".into(),
            "--".into(), "echo".into(), "{i}".into(),
        ]).unwrap();
        assert_eq!(s.template, "echo {i}");
        assert_eq!(s.inline_items.unwrap(), vec!["a", "b", "c"]);
        assert!(s.argfile.is_none());
        assert_eq!(s.var_name.unwrap(), "i");
    }

    #[test]
    fn bash_stdin() {
        let s = split_rest(vec![
            "url".into(), "--".into(), "curl".into(), "-sO".into(), "{url}".into(),
        ]).unwrap();
        assert_eq!(s.template, "curl -sO {url}");
        assert!(s.inline_items.is_none());
        assert!(s.argfile.is_none());
        assert_eq!(s.var_name.unwrap(), "url");
    }

    #[test]
    fn bash_argfile() {
        let s = split_rest(vec![
            "f".into(), "in".into(), "::::".into(), "urls.txt".into(),
            "--".into(), "curl".into(), "{f}".into(),
        ]).unwrap();
        assert_eq!(s.template, "curl {f}");
        assert!(s.inline_items.is_none());
        assert_eq!(s.argfile.unwrap(), "urls.txt");
        assert_eq!(s.var_name.unwrap(), "f");
    }

    #[test]
    fn bash_no_separator_err() {
        let result = split_rest(vec![
            "i".into(), "in".into(), "a".into(), "b".into(),
        ]);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("--"));
    }

    #[test]
    fn bash_no_command_after_sep_err() {
        let result = split_rest(vec![
            "i".into(), "in".into(), "a".into(), "--".into(),
        ]);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("command"));
    }

    #[test]
    fn bash_stdin_no_command_err() {
        let result = split_rest(vec!["i".into(), "--".into()]);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("command"));
    }

    #[test]
    fn bash_empty_items_err() {
        // `pfor i in -- echo {i}` — no items between in and --
        let result = split_rest(vec![
            "i".into(), "in".into(), "--".into(), "echo".into(), "{i}".into(),
        ]);
        assert!(result.is_err());
    }

    #[test]
    fn template_with_braces_goes_gnu() {
        // rest[0] contains {} → should be treated as GNU-parallel
        let s = split_rest(vec![
            "echo {}".into(), ":::".into(), "a".into(),
        ]).unwrap();
        assert!(s.var_name.is_none());
        assert_eq!(s.template, "echo {}");
    }

    #[test]
    fn template_with_index_goes_gnu() {
        // rest[0] contains {#} → should be treated as GNU-parallel
        let s = split_rest(vec![
            "echo {#}".into(), ":::".into(), "a".into(),
        ]).unwrap();
        assert!(s.var_name.is_none());
    }
}
