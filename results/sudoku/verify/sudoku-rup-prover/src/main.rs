//! Command line front end of the Sudoku proof generator.
//!
//! ```text
//! sudoku-rup-prover prove <puzzle81> <answer81>
//! sudoku-rup-prover batch <boards.tsv> <out-dir>
//! ```
//!
//! Exit status: 0 proved (in `batch`: every board proved), 2 refused or unreadable input,
//! 64 wrong usage.

#![forbid(unsafe_code)]

use std::fs::{self, OpenOptions};
use std::io::{self, BufWriter, Write};
use std::path::Path;
use std::process::ExitCode;

use sudoku_rup_check::{format_lemma, parse_grid, parse_puzzle};
use sudoku_rup_prover::{Proof, ProveError, prove};

const USAGE: &str =
    "usage: sudoku-rup-prover prove <puzzle81> <answer81> | batch <boards.tsv> <out-dir>";

const EXIT_PROVED: u8 = 0;
const EXIT_FAILED: u8 = 2;
const EXIT_USAGE: u8 = 64;

/// The proof as DRUP text: one lemma per line, each closed by `0` (see `format_lemma`).
fn drup_text(lemmas: &[Vec<i32>]) -> String {
    let mut text = String::new();
    for lemma in lemmas {
        text.push_str(&format_lemma(lemma));
        text.push('\n');
    }
    text
}

/// Parses the two board texts and proves the board. A text that does not parse is refused with
/// `PUZZLE_FORM` or `ANSWER_FORM`.
fn prove_texts(puzzle_text: &str, answer_text: &str) -> Result<Proof, ProveError> {
    let puzzle = parse_puzzle(puzzle_text)
        .map_err(|error| ProveError::new("PUZZLE_FORM", error.to_string()))?;
    let answer = parse_grid(answer_text)
        .map_err(|error| ProveError::new("ANSWER_FORM", error.to_string()))?;
    prove(&puzzle, &answer)
}

fn usage() -> io::Result<u8> {
    eprintln!("{USAGE}");
    Ok(EXIT_USAGE)
}

/// `prove <puzzle81> <answer81>`
fn cmd_prove(args: &[String], out: &mut dyn Write) -> io::Result<u8> {
    if args.len() != 2 {
        return usage();
    }
    match prove_texts(&args[0], &args[1]) {
        Ok(proof) => {
            out.write_all(drup_text(&proof.lemmas).as_bytes())?;
            Ok(EXIT_PROVED)
        }
        Err(error) => {
            eprintln!("FAILED {} {}", error.reason, error.detail);
            Ok(EXIT_FAILED)
        }
    }
}

/// Proves one line of the boards file, `<id>\t<puzzle81>\t<answer81>`, and writes
/// `<out-dir>/<id>.drup`. Returns the label to print (the id, or `line<N>` if the id is unusable)
/// and either the summary of the proof or the reason code of the refusal.
fn batch_one(line: &str, line_no: usize, dir: &Path) -> (String, Result<String, &'static str>) {
    let fields: Vec<&str> = line.split('\t').collect();
    if fields.len() != 3 {
        return (format!("line{line_no}"), Err("BAD_LINE"));
    }
    let id = fields[0];
    if id.is_empty() || id.len() > 12 || !id.bytes().all(|b| b.is_ascii_digit()) {
        return (format!("line{line_no}"), Err("BAD_ID"));
    }
    let label = id.to_string();
    let path = dir.join(format!("{id}.drup"));
    if path.exists() {
        return (label, Err("OUT_EXISTS"));
    }
    let proof = match prove_texts(fields[1], fields[2]) {
        Ok(proof) => proof,
        Err(error) => return (label, Err(error.reason)),
    };
    // Create-only: an existing file is never overwritten.
    let mut file = match OpenOptions::new().write(true).create_new(true).open(&path) {
        Ok(file) => file,
        Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {
            return (label, Err("OUT_EXISTS"));
        }
        Err(_) => return (label, Err("WRITE")),
    };
    if file.write_all(drup_text(&proof.lemmas).as_bytes()).is_err() {
        drop(file);
        let _ = fs::remove_file(&path);
        return (label, Err("WRITE"));
    }
    let summary = format!(
        "lemmas={} units={} binaries={} longer={}",
        proof.lemmas.len(),
        proof.units,
        proof.binaries,
        proof.longer
    );
    (label, Ok(summary))
}

/// `batch <boards.tsv> <out-dir>`
fn cmd_batch(args: &[String], out: &mut dyn Write) -> io::Result<u8> {
    if args.len() != 2 {
        return usage();
    }
    let tsv = match fs::read_to_string(&args[0]) {
        Ok(text) => text,
        Err(_) => {
            eprintln!("FAILED READ {}", args[0]);
            return Ok(EXIT_FAILED);
        }
    };
    let dir = Path::new(&args[1]);
    if !dir.is_dir() {
        eprintln!("FAILED OUT_DIR {}", args[1]);
        return Ok(EXIT_FAILED);
    }
    let mut boards = 0usize;
    let mut proved = 0usize;
    let mut failed = 0usize;
    for (index, line) in tsv.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        boards += 1;
        let (label, result) = batch_one(line, index + 1, dir);
        match result {
            Ok(summary) => {
                proved += 1;
                writeln!(out, "{label}\tPROVED\t{summary}")?;
            }
            Err(reason) => {
                failed += 1;
                writeln!(out, "{label}\tFAILED\t{reason}")?;
            }
        }
        // A board can take a while: show each line as soon as it is known.
        out.flush()?;
    }
    writeln!(out, "TOTAL boards={boards} proved={proved} failed={failed}")?;
    Ok(if failed == 0 {
        EXIT_PROVED
    } else {
        EXIT_FAILED
    })
}

fn run(args: &[String], out: &mut dyn Write) -> io::Result<u8> {
    let Some((command, rest)) = args.split_first() else {
        return usage();
    };
    match command.as_str() {
        "prove" => cmd_prove(rest, out),
        "batch" => cmd_batch(rest, out),
        _ => usage(),
    }
}

fn main() -> ExitCode {
    let mut args: Vec<String> = Vec::new();
    for arg in std::env::args_os().skip(1) {
        match arg.into_string() {
            Ok(text) => args.push(text),
            Err(_) => {
                eprintln!("{USAGE}");
                return ExitCode::from(EXIT_USAGE);
            }
        }
    }
    let stdout = io::stdout();
    let mut out = BufWriter::new(stdout.lock());
    let result = run(&args, &mut out);
    let flushed = out.flush();
    match (result, flushed) {
        (Ok(code), Ok(())) => ExitCode::from(code),
        _ => {
            eprintln!("FAILED WRITE stdout");
            ExitCode::from(EXIT_FAILED)
        }
    }
}
