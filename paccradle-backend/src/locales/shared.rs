use serde_json::{Map, Value};

use super::Locale;

/// The first language in an `Accept-Language` value (highest `q` first) that the API speaks,
/// or English. `id-ID,id;q=0.9,en;q=0.8` gives English until Bahasa Indonesia is added.
pub(super) fn pick_locale(accept_language: &str) -> Locale {
    let mut wanted: Vec<(&str, f32)> = accept_language
        .split(',')
        .filter_map(|item| {
            let mut parts = item.split(';').map(str::trim);
            let tag = parts.next().filter(|tag| !tag.is_empty())?;
            let quality = parts
                .find_map(|param| param.strip_prefix("q="))
                .map_or(Some(1.0), |q| q.parse::<f32>().ok())?;
            (quality > 0.0).then_some((tag, quality))
        })
        .collect();
    // Stable, so equal weights keep the caller's order.
    wanted.sort_by(|a, b| b.1.total_cmp(&a.1));
    wanted
        .iter()
        .find_map(|(tag, _)| {
            let primary = tag.split('-').next().unwrap_or(tag);
            Locale::ALL
                .into_iter()
                .find(|locale| primary.eq_ignore_ascii_case(locale.tag()))
        })
        .unwrap_or_default()
}

/// Puts meta values into a message's `{name}` placeholders. Strings go in as they are, other
/// values as JSON; a placeholder without a value stays as written.
pub fn fill(template: &str, meta: &Map<String, Value>) -> String {
    let mut out = String::with_capacity(template.len());
    let mut rest = template;
    while let Some(open) = rest.find('{') {
        out.push_str(&rest[..open]);
        let after = &rest[open + 1..];
        let Some(close) = after.find('}') else {
            out.push_str(&rest[open..]);
            return out;
        };
        let name = &after[..close];
        match meta.get(name) {
            Some(Value::String(text)) => out.push_str(text),
            Some(value) => out.push_str(&value.to_string()),
            None => out.push_str(&rest[open..open + close + 2]),
        }
        rest = &after[close + 1..];
    }
    out.push_str(rest);
    out
}
