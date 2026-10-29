//! Measures one photo and prints the result as JSON (CLAUDE.md §5.5), with debug images
//! on request. Built for the accuracy spike (M0-08 to M0-10); reads photos the way a
//! capture job does (M1-C1).
//!
//! ```text
//! measure <photo> [--paper a4|letter] [--mode photo|trace] [--pen-width-mm 0.5]
//!                 [--tap X,Y] [--height-mm H] [--focal-35mm F] [--no-grabcut]
//!                 [--debug-dir DIR]
//! ```
//!
//! `--tap` picks the product as a tap does (M1-C3): a point in photo pixels. `--height-mm`
//! is the typed height and `--focal-35mm` the lens from the photo's EXIF, for the
//! tall-product warning (M1-C4).

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use opencv::core::Mat;
use opencv::imgcodecs;
use vision::{AnalyzeOptions, DebugImages, analyze};

const USAGE: &str = "usage: measure <photo> [--paper a4|letter] [--mode photo|trace] \
                     [--pen-width-mm 0.5] [--tap X,Y] [--height-mm H] [--focal-35mm F] \
                     [--no-grabcut] [--debug-dir DIR]";

struct Args {
    photo: PathBuf,
    options: AnalyzeOptions,
    debug_dir: Option<PathBuf>,
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let args = parse_args(std::env::args().skip(1).collect())?;
    let image = read_photo(&args.photo)?;
    let analysis = analyze(&image, &args.options)?;
    if let (Some(dir), Some(debug)) = (&args.debug_dir, &analysis.debug) {
        write_debug(dir, debug)?;
    }
    println!("{}", serde_json::to_string_pretty(&analysis.result)?);
    Ok(())
}

fn parse_args(raw: Vec<String>) -> Result<Args, String> {
    let mut photo = None;
    let mut options = AnalyzeOptions::default();
    let mut debug_dir = None;
    let mut iter = raw.into_iter();
    while let Some(arg) = iter.next() {
        let mut value = |name: &str| iter.next().ok_or(format!("{name} needs a value\n{USAGE}"));
        match arg.as_str() {
            "--paper" => options.paper = value("--paper")?.parse()?,
            "--mode" => options.mode = value("--mode")?.parse()?,
            "--pen-width-mm" => options.pen_width_mm = number(("--pen-width-mm", value))?,
            "--height-mm" => options.height_mm = Some(number(("--height-mm", value))?),
            "--focal-35mm" => options.focal_length_35mm = Some(number(("--focal-35mm", value))?),
            "--tap" => options.tap_point_px = Some(parse_point(&value("--tap")?)?),
            "--no-grabcut" => options.refine_with_grabcut = false,
            "--debug-dir" => debug_dir = Some(PathBuf::from(value("--debug-dir")?)),
            "-h" | "--help" => return Err(USAGE.to_owned()),
            flag if flag.starts_with("--") => return Err(format!("unknown flag {flag}\n{USAGE}")),
            path => photo = Some(PathBuf::from(path)),
        }
    }
    options.collect_debug = debug_dir.is_some();
    let photo = photo.ok_or(USAGE.to_owned())?;
    Ok(Args {
        photo,
        options,
        debug_dir,
    })
}

/// Reads the value of a number flag.
fn number(
    (name, mut value): (&str, impl FnMut(&str) -> Result<String, String>),
) -> Result<f64, String> {
    value(name)?
        .parse()
        .map_err(|_| format!("{name} must be a number"))
}

/// Reads `X,Y` in photo pixels.
fn parse_point(text: &str) -> Result<[f64; 2], String> {
    let invalid = || format!("--tap takes X,Y in photo pixels, not `{text}`");
    let (x, y) = text.split_once(',').ok_or_else(invalid)?;
    let number = |part: &str| part.trim().parse::<f64>().map_err(|_| invalid());
    Ok([number(x)?, number(y)?])
}

/// Reads a photo the way a capture job does (M1-C1): JPEG, PNG or WebP only, within the
/// pixel limits, upright by its EXIF orientation.
fn read_photo(path: &Path) -> Result<Mat, Box<dyn std::error::Error>> {
    let bytes = std::fs::read(path)?;
    Ok(vision::decode(&bytes)?)
}

fn write_debug(dir: &Path, debug: &DebugImages) -> Result<(), Box<dyn std::error::Error>> {
    std::fs::create_dir_all(dir)?;
    let images = [
        ("1-paper.jpg", &debug.paper),
        ("2-flattened.jpg", &debug.flattened),
        ("3-mask.png", &debug.mask),
        ("4-outline.jpg", &debug.outline),
    ];
    for (name, image) in images {
        let path = dir.join(name);
        let path_text = path.to_str().ok_or("debug path is not valid UTF-8")?;
        imgcodecs::imwrite_def(path_text, image)?;
    }
    eprintln!("debug images written to {}", dir.display());
    Ok(())
}
