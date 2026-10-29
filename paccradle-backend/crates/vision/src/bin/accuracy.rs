//! Accuracy report (M0-11): measures every photo listed in `truth.csv`, compares the result
//! with the caliper sizes and writes `REPORT.md` against the CLAUDE.md §5.6 targets.
//!
//! ```text
//! accuracy [--spike-dir crates/vision/spike]
//! ```
//! Reads `<spike-dir>/truth.csv` and `<spike-dir>/photos/`, writes `<spike-dir>/REPORT.md`.

use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use vision::{CaptureInput, CaptureMode, PaperSize};

const DEFAULT_SPIKE_DIR: &str = "crates/vision/spike";
/// Products up to this height count as "flat or low" for photo mode (CLAUDE.md §5.6).
const LOW_PRODUCT_MAX_HEIGHT_MM: f64 = 20.0;
const P90: f64 = 0.9;

/// One row of truth.csv: a photo and the product's caliper sizes.
struct TruthRow {
    photo: String,
    object: String,
    mode: CaptureMode,
    paper: PaperSize,
    length_mm: Option<f64>,
    width_mm: Option<f64>,
    diameter_mm: Option<f64>,
    height_mm: Option<f64>,
}

/// The §5.6 groups and their targets (P90, median).
#[derive(Clone, Copy, PartialEq)]
enum Group {
    Trace,
    PhotoLow,
    PhotoTall,
}

impl Group {
    fn of(row: &TruthRow) -> Self {
        match (row.mode, row.height_mm) {
            (CaptureMode::Trace, _) => Self::Trace,
            (CaptureMode::Photo, Some(height)) if height > LOW_PRODUCT_MAX_HEIGHT_MM => {
                Self::PhotoTall
            }
            (CaptureMode::Photo, _) => Self::PhotoLow,
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::Trace => "Trace mode",
            Self::PhotoLow => "Photo, flat or low (≤ 20 mm)",
            Self::PhotoTall => "Photo, tall (> 20 mm; v1 target needs pose correction)",
        }
    }

    /// (P90, median) targets in mm.
    fn targets(self) -> (f64, f64) {
        match self {
            Self::Trace => (1.0, 0.5),
            Self::PhotoLow => (1.5, 0.8),
            Self::PhotoTall => (2.0, 1.0),
        }
    }
}

/// What happened to one photo.
struct Outcome {
    row: TruthRow,
    /// Measured "L × W" or "Ø" text, or the error message.
    measured: String,
    /// Absolute errors in mm (one per compared dimension); empty when the photo failed.
    errors: Vec<f64>,
    confidence: Option<f64>,
}

fn main() -> ExitCode {
    match run() {
        Ok(path) => {
            println!("report written to {}", path.display());
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<PathBuf, Box<dyn std::error::Error>> {
    let spike_dir = spike_dir_from_args()?;
    let truth = std::fs::read_to_string(spike_dir.join("truth.csv"))?;
    let rows = parse_truth(&truth)?;
    if rows.is_empty() {
        return Err(
            "truth.csv has no rows yet: add one line per photo (see spike/README.md)".into(),
        );
    }
    let outcomes = rows
        .into_iter()
        .map(|row| evaluate(&spike_dir.join("photos"), row))
        .collect::<Vec<_>>();
    let report_path = spike_dir.join("REPORT.md");
    std::fs::write(&report_path, render_report(&outcomes)?)?;
    Ok(report_path)
}

fn spike_dir_from_args() -> Result<PathBuf, String> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.as_slice() {
        [] => Ok(PathBuf::from(DEFAULT_SPIKE_DIR)),
        [flag, dir] if flag == "--spike-dir" => Ok(PathBuf::from(dir)),
        _ => Err("usage: accuracy [--spike-dir DIR]".to_owned()),
    }
}

/// Parses truth.csv. Columns:
/// `photo,object,mode,paper,true_l_mm,true_w_mm,true_diameter_mm,height_mm,phone,notes`.
fn parse_truth(text: &str) -> Result<Vec<TruthRow>, String> {
    text.lines()
        .enumerate()
        .skip(1)
        .filter(|(_, line)| !line.trim().is_empty())
        .map(|(index, line)| {
            parse_row(line).map_err(|error| format!("truth.csv line {}: {error}", index + 1))
        })
        .collect()
}

fn parse_row(line: &str) -> Result<TruthRow, String> {
    let cells: Vec<&str> = line.split(',').map(str::trim).collect();
    let cell = |index: usize| cells.get(index).copied().unwrap_or_default();
    let number = |index: usize| -> Result<Option<f64>, String> {
        let text = cell(index);
        if text.is_empty() {
            return Ok(None);
        }
        text.parse()
            .map(Some)
            .map_err(|_| format!("`{text}` is not a number"))
    };
    let row = TruthRow {
        photo: cell(0).to_owned(),
        object: cell(1).to_owned(),
        mode: cell(2).parse()?,
        paper: cell(3).parse()?,
        length_mm: number(4)?,
        width_mm: number(5)?,
        diameter_mm: number(6)?,
        height_mm: number(7)?,
    };
    if row.photo.is_empty() {
        return Err("photo is empty".to_owned());
    }
    if row.diameter_mm.is_none() && (row.length_mm.is_none() || row.width_mm.is_none()) {
        return Err("give true_diameter_mm, or both true_l_mm and true_w_mm".to_owned());
    }
    Ok(row)
}

fn evaluate(photos_dir: &Path, row: TruthRow) -> Outcome {
    match measure_row(photos_dir, &row) {
        Ok((measured, errors, confidence)) => Outcome {
            row,
            measured,
            errors,
            confidence: Some(confidence),
        },
        Err(error) => Outcome {
            row,
            measured: format!("failed: {error}"),
            errors: Vec::new(),
            confidence: None,
        },
    }
}

fn measure_row(
    photos_dir: &Path,
    row: &TruthRow,
) -> Result<(String, Vec<f64>, f64), Box<dyn std::error::Error>> {
    // The same path a capture job takes (M1-C1): file bytes in, §5.5 result out.
    let bytes = std::fs::read(photos_dir.join(&row.photo))?;
    let input = CaptureInput {
        paper: row.paper,
        mode: row.mode,
        pen_width_mm: vision::DEFAULT_PEN_WIDTH_MM,
    };
    let result = vision::run((&input, &bytes))?;
    let product = result.products.first().ok_or("no product")?;
    let (l, w) = (product.bbox_mm.l, product.bbox_mm.w);

    // Round truth: compare the diameter (or both box sides if the shape wasn't seen as round).
    if let Some(true_diameter) = row.diameter_mm {
        return Ok(match product.diameter_mm {
            Some(diameter) => (
                format!("Ø {diameter:.2}"),
                vec![(diameter - true_diameter).abs()],
                product.confidence,
            ),
            None => (
                format!("{l:.2} × {w:.2}"),
                vec![(l - true_diameter).abs(), (w - true_diameter).abs()],
                product.confidence,
            ),
        });
    }
    let (true_l, true_w) = (
        row.length_mm.unwrap_or_default(),
        row.width_mm.unwrap_or_default(),
    );
    Ok((
        format!("{l:.2} × {w:.2}"),
        vec![(l - true_l).abs(), (w - true_w).abs()],
        product.confidence,
    ))
}

fn truth_text(row: &TruthRow) -> String {
    match (row.diameter_mm, row.length_mm, row.width_mm) {
        (Some(diameter), _, _) => format!("Ø {diameter:.2}"),
        (None, Some(l), Some(w)) => format!("{l:.2} × {w:.2}"),
        _ => "?".to_owned(),
    }
}

/// Nearest-rank percentile of a sorted list.
fn percentile(sorted: &[f64], share: f64) -> f64 {
    let rank = (share * sorted.len() as f64).ceil() as usize;
    sorted[rank.clamp(1, sorted.len()) - 1]
}

fn render_report(outcomes: &[Outcome]) -> Result<String, std::fmt::Error> {
    let mut out = String::new();
    writeln!(out, "# Vision accuracy report (M0-11)\n")?;
    writeln!(
        out,
        "Generated by `cargo run -p vision --release --bin accuracy`. Targets from CLAUDE.md §5.6. \
         Errors are absolute, in mm; P90 uses the nearest-rank method.\n"
    )?;
    writeln!(out, "## Summary\n")?;
    writeln!(
        out,
        "| Group | Photos | Measured dimensions | Median | P90 | Target median / P90 | Pass |"
    )?;
    writeln!(out, "| --- | --- | --- | --- | --- | --- | --- |")?;
    for group in [Group::Trace, Group::PhotoLow, Group::PhotoTall] {
        let in_group: Vec<&Outcome> = outcomes
            .iter()
            .filter(|o| Group::of(&o.row) == group)
            .collect();
        if in_group.is_empty() {
            continue;
        }
        let mut errors: Vec<f64> = in_group
            .iter()
            .flat_map(|o| o.errors.iter().copied())
            .collect();
        errors.sort_by(f64::total_cmp);
        let (target_p90, target_median) = group.targets();
        if errors.is_empty() {
            writeln!(
                out,
                "| {} | {} | 0 | – | – | {target_median} / {target_p90} | no data |",
                group.label(),
                in_group.len()
            )?;
            continue;
        }
        let (median, p90) = (percentile(&errors, 0.5), percentile(&errors, P90));
        let pass = if median <= target_median && p90 <= target_p90 {
            "yes"
        } else {
            "**no**"
        };
        writeln!(
            out,
            "| {} | {} | {} | {median:.2} | {p90:.2} | {target_median} / {target_p90} | {pass} |",
            group.label(),
            in_group.len(),
            errors.len()
        )?;
    }
    let failed = outcomes.iter().filter(|o| o.errors.is_empty()).count();
    writeln!(
        out,
        "\nPhotos that could not be measured: {failed} of {}.\n",
        outcomes.len()
    )?;

    writeln!(out, "## Per photo\n")?;
    writeln!(
        out,
        "| Photo | Object | Mode | Truth (mm) | Measured (mm) | Max error | Confidence |"
    )?;
    writeln!(out, "| --- | --- | --- | --- | --- | --- | --- |")?;
    for outcome in outcomes {
        let max_error = outcome
            .errors
            .iter()
            .copied()
            .fold(None, |max: Option<f64>, e| {
                Some(max.map_or(e, |m| m.max(e)))
            })
            .map_or("–".to_owned(), |e| format!("{e:.2}"));
        let confidence = outcome
            .confidence
            .map_or("–".to_owned(), |c| format!("{c:.2}"));
        writeln!(
            out,
            "| {} | {} | {:?} | {} | {} | {max_error} | {confidence} |",
            outcome.row.photo,
            outcome.row.object,
            outcome.row.mode,
            truth_text(&outcome.row),
            outcome.measured,
        )?;
    }
    Ok(out)
}
