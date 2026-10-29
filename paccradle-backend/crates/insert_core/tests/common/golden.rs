//! Golden files (M1-A9): output a person has checked once, kept in `tests/golden/`, and
//! compared on every run. Numbers match within [`NUMBER_SLACK_MM`], so last-digit
//! rounding between machines passes; anything else is a change to look at.
//!
//! Accept intended changes with `UPDATE_GOLDEN=1 cargo test -p insert_core`.

use std::path::{Path, PathBuf};

/// How far numbers may drift: two steps of 0.001 mm rounding.
const NUMBER_SLACK_MM: f64 = 0.002;

/// Compares `content` with the golden file `tests/golden/<file>`, or writes it there
/// when `UPDATE_GOLDEN` is set. A difference writes the new file to the target folder
/// and says where, and which line differs first.
pub fn check((file, content): (&str, &str)) -> Result<(), String> {
    let golden = golden_dir().join(file);
    if std::env::var_os("UPDATE_GOLDEN").is_some() {
        write((&golden, content));
        return Ok(());
    }
    let Ok(expected) = std::fs::read_to_string(&golden) else {
        return Err(format!(
            "{file}: no golden file at {}. Run with UPDATE_GOLDEN=1, then look at it.",
            golden.display()
        ));
    };
    let Some(line) = first_difference((&expected, content)) else {
        return Ok(());
    };
    let actual_dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("golden");
    let actual = actual_dir.join(file);
    write((&actual, content));
    Err(format!(
        "{file}: differs from {} at line {line}.\n  new file: {}",
        golden.display(),
        actual.display()
    ))
}

/// Writes `content` to `path`, making its folder first.
fn write((path, content): (&Path, &str)) {
    if let Some(folder) = path.parent() {
        std::fs::create_dir_all(folder)
            .unwrap_or_else(|error| panic!("{}: {error}", folder.display()));
    }
    std::fs::write(path, content).unwrap_or_else(|error| panic!("{}: {error}", path.display()));
}

fn golden_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/golden")
}

/// The first line (from 1) where the drawings differ beyond number rounding, or `None`.
pub fn first_difference((expected, actual): (&str, &str)) -> Option<usize> {
    let mut expected_lines = expected.lines();
    let mut actual_lines = actual.lines();
    for line in 1.. {
        match (expected_lines.next(), actual_lines.next()) {
            (None, None) => return None,
            (Some(want), Some(got)) if same_line((want, got)) => {}
            _ => return Some(line),
        }
    }
    None
}

/// True when two lines match, with numbers within [`NUMBER_SLACK_MM`].
pub fn same_line((want, got): (&str, &str)) -> bool {
    let (want, got) = (tokens(want), tokens(got));
    want.len() == got.len()
        && want.iter().zip(&got).all(|pair| match pair {
            (Token::Number(a), Token::Number(b)) => (a - b).abs() <= NUMBER_SLACK_MM,
            (Token::Text(a), Token::Text(b)) => a == b,
            _ => false,
        })
}

#[derive(Debug)]
enum Token<'a> {
    Number(f64),
    Text(&'a str),
}

/// Splits a line into numbers (an optional minus, digits, an optional fraction) and
/// the text between them.
fn tokens(line: &str) -> Vec<Token<'_>> {
    let bytes = line.as_bytes();
    let mut tokens = Vec::new();
    let (mut start, mut at) = (0, 0);
    while at < bytes.len() {
        let number_starts = bytes[at].is_ascii_digit()
            || (bytes[at] == b'-' && bytes.get(at + 1).is_some_and(u8::is_ascii_digit));
        if !number_starts {
            at += 1;
            continue;
        }
        if start < at {
            tokens.push(Token::Text(&line[start..at]));
        }
        let mut end = at + 1;
        while end < bytes.len() && (bytes[end].is_ascii_digit() || bytes[end] == b'.') {
            end += 1;
        }
        tokens.push(match line[at..end].parse() {
            Ok(number) => Token::Number(number),
            Err(_) => Token::Text(&line[at..end]),
        });
        (start, at) = (end, end);
    }
    if start < bytes.len() {
        tokens.push(Token::Text(&line[start..]));
    }
    tokens
}
