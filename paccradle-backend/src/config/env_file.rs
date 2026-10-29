//! Local `.env` files, read with the same rules as `docker run --env-file` (what production
//! uses), so one file works for both. Values are never written into the process environment:
//! [`super::AppConfig::load`] looks them up after the real environment.

use std::collections::BTreeMap;
use std::path::Path;
use std::{fs, io};

/// Reads `KEY=value` lines from `path`. A missing file gives no values.
pub fn read(path: &Path) -> io::Result<BTreeMap<String, String>> {
    match fs::read_to_string(path) {
        Ok(text) => Ok(parse(&text)),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(BTreeMap::new()),
        Err(error) => Err(error),
    }
}

/// Parses `KEY=value` lines. Blank lines and lines starting with `#` are skipped, and so are
/// lines without `=`. Quotes are kept as part of the value, as Docker keeps them.
pub fn parse(text: &str) -> BTreeMap<String, String> {
    text.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .filter_map(|line| line.split_once('='))
        .map(|(key, value)| (key.trim(), value.trim()))
        .filter(|(key, _)| !key.is_empty())
        .map(|(key, value)| (key.to_owned(), value.to_owned()))
        .collect()
}
