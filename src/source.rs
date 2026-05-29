//! Input source resolution: `:::`, `::::`, or stdin.

use std::fs::File;
use std::io::{self, BufRead, BufReader, IsTerminal, Read};

/// Read all items from a buffered reader, one per line.
/// Empty lines are skipped.
fn read_lines<R: Read>(r: R) -> io::Result<Vec<String>> {
    let buf = BufReader::new(r);
    let mut items = Vec::new();
    for line in buf.lines() {
        let line = line?;
        if !line.is_empty() {
            items.push(line);
        }
    }
    Ok(items)
}

/// Resolve the item list from CLI parts.
///
/// `inline` is the slice of arguments after a `:::` marker.
/// `file` is the path after a `::::` marker.
/// If neither is present, read items from stdin (one per line).
///
/// Exactly one of `inline` / `file` may be specified.
pub fn resolve(
    inline: Option<Vec<String>>,
    file: Option<String>,
) -> io::Result<Vec<String>> {
    match (inline, file) {
        (Some(items), None) => Ok(items),
        (None, Some(path)) => read_lines(File::open(path)?),
        (None, None) => {
            // stdin
            if io::stdin().is_terminal() {
                // User invoked pfor without items and without piping anything.
                // Read stdin anyway (will block) — but emit a hint via error.
                // Actually fall through to read: if user truly wants stdin,
                // they'll Ctrl-D. If they made a mistake, --help is documented.
            }
            read_lines(io::stdin().lock())
        }
        (Some(_), Some(_)) => Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "specify either `::: args` or `:::: file`, not both",
        )),
    }
}
