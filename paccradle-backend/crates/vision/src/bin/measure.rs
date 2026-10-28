//! Spike CLI (M0-08 to M0-10): measures one photo and prints the result as JSON
//! (CLAUDE.md §5.5).
//!
//! ```text
//! measure <photo> [--paper a4|letter] [--mode photo|trace] [--pen-width-mm 0.5]
//!                 [--no-grabcut] [--debug-dir DIR]
//! ```

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use opencv::core::Mat;
use opencv::imgcodecs;
use opencv::prelude::*;
use vision::{AnalyzeOptions, DebugImages, analyze};

const USAGE: &str = "usage: measure <photo> [--paper a4|letter] [--mode photo|trace] \
                     [--pen-width-mm 0.5] [--no-grabcut] [--debug-dir DIR]";

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
            "--pen-width-mm" => {
                options.pen_width_mm = value("--pen-width-mm")?
                    .parse()
                    .map_err(|_| "--pen-width-mm must be a number".to_owned())?;
            }
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

/// Reads a photo as BGR; OpenCV applies the EXIF orientation (CLAUDE.md §5.3 step 1).
fn read_photo(path: &Path) -> Result<Mat, Box<dyn std::error::Error>> {
    let path_text = path.to_str().ok_or("photo path is not valid UTF-8")?;
    let image = imgcodecs::imread(path_text, imgcodecs::IMREAD_COLOR)?;
    if image.empty() {
        return Err(format!("could not read {path_text}").into());
    }
    Ok(image)
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
