//! Command line front end of the Sudoku RUP checker.
//!
//! ```text
//! sudoku-rup-check check <puzzle81> <proof-file> [<expected-grid81>]
//! sudoku-rup-check batch <boards.tsv> <proof-dir>
//! sudoku-rup-check dataset <test.csv> <proof-dir>
//! sudoku-rup-check dimacs <puzzle81> <grid81>
//! sudoku-rup-check drat <proof-file>
//! ```
//!
//! Exit status: 0 verified (or no failed board in `batch`), 1 open, 2 failed
//! or unreadable input, 64 wrong usage.

#![forbid(unsafe_code)]

use std::fs;
use std::io::{self, BufWriter, Write};
use std::path::Path;
use std::process::ExitCode;

use sudoku_rup_check::{
    CheckError, Verdict, check_board, dimacs_with_block, drat_text, grid_to_string, parse_grid,
    parse_proof, parse_puzzle,
};

const USAGE: &str = "usage: sudoku-rup-check check <puzzle81> <proof-file> [<expected-grid81>] \
| batch <boards.tsv> <proof-dir> | dataset <test.csv> <proof-dir> | dimacs <puzzle81> <grid81> | drat <proof-file>";

const EXIT_VERIFIED: u8 = 0;
const EXIT_OPEN: u8 = 1;
const EXIT_FAILED: u8 = 2;
const EXIT_USAGE: u8 = 64;

/// Result of checking one board, ready to be printed.
enum Outcome {
    Verified { grid: String, lemmas: usize },
    Open { decided: usize, lemmas: usize },
    /// `detail` is `<REASON> lemma=<index or ->`.
    Failed { detail: String },
}

impl Outcome {
    fn exit_code(&self) -> u8 {
        match self {
            Outcome::Verified { .. } => EXIT_VERIFIED,
            Outcome::Open { .. } => EXIT_OPEN,
            Outcome::Failed { .. } => EXIT_FAILED,
        }
    }
}

fn failed(detail: impl Into<String>) -> Outcome {
    Outcome::Failed {
        detail: detail.into(),
    }
}

fn parse_failure(err: &CheckError) -> Outcome {
    failed(format!("PARSE {} lemma=-", err))
}

fn lemma_text(lemma: Option<usize>) -> String {
    match lemma {
        Some(i) => i.to_string(),
        None => "-".to_string(),
    }
}

/// Parses the three texts and runs the checker.
fn evaluate(puzzle_text: &str, proof_text: &str, expected_text: Option<&str>) -> Outcome {
    let puzzle = match parse_puzzle(puzzle_text) {
        Ok(p) => p,
        Err(e) => return parse_failure(&e),
    };
    let lemmas = match parse_proof(proof_text) {
        Ok(l) => l,
        Err(e) => return parse_failure(&e),
    };
    let expected = match expected_text {
        Some(text) => match parse_grid(text) {
            Ok(g) => Some(g),
            Err(e) => return parse_failure(&e),
        },
        None => None,
    };
    match check_board(&puzzle, &lemmas, expected.as_ref()) {
        Verdict::Verified { grid, lemmas: n } => Outcome::Verified {
            grid: grid_to_string(&grid),
            lemmas: n,
        },
        Verdict::Open {
            decided_cells,
            lemmas: n,
        } => Outcome::Open {
            decided: decided_cells,
            lemmas: n,
        },
        Verdict::Failed { reason, lemma } => {
            failed(format!("{} lemma={}", reason, lemma_text(lemma)))
        }
    }
}

fn render_check(outcome: &Outcome) -> String {
    match outcome {
        Outcome::Verified { grid, lemmas } => format!("VERIFIED {} lemmas={}", grid, lemmas),
        Outcome::Open { decided, lemmas } => format!("OPEN decided={} lemmas={}", decided, lemmas),
        Outcome::Failed { detail } => format!("FAILED {}", detail),
    }
}

fn render_batch(id: &str, outcome: &Outcome) -> String {
    match outcome {
        Outcome::Verified { grid, .. } => format!("{}\tVERIFIED\t{}", id, grid),
        Outcome::Open { decided, .. } => format!("{}\tOPEN\tdecided={}", id, decided),
        Outcome::Failed { detail } => format!("{}\tFAILED\t{}", id, detail),
    }
}

fn usage() -> io::Result<u8> {
    eprintln!("{}", USAGE);
    Ok(EXIT_USAGE)
}

/// `check <puzzle81> <proof-file> [<expected-grid81>]`
fn cmd_check(args: &[String], out: &mut dyn Write) -> io::Result<u8> {
    if args.len() != 2 && args.len() != 3 {
        return usage();
    }
    let proof_text = match fs::read_to_string(&args[1]) {
        Ok(text) => text,
        Err(_) => {
            writeln!(out, "FAILED READ {}", args[1])?;
            return Ok(EXIT_FAILED);
        }
    };
    let outcome = evaluate(&args[0], &proof_text, args.get(2).map(String::as_str));
    writeln!(out, "{}", render_check(&outcome))?;
    Ok(outcome.exit_code())
}

/// Checks one line of the boards file: `<id>\t<puzzle81>\t<grid81 or ->`.
/// Returns the label to print (the id, or `line<N>` if the id is unusable)
/// and the outcome.
fn batch_one(line: &str, line_no: usize, dir: &Path) -> (String, Outcome) {
    let fields: Vec<&str> = line.split('\t').collect();
    if fields.len() != 3 {
        return (format!("line{}", line_no), failed("BAD_LINE lemma=-"));
    }
    let id = fields[0];
    if id.is_empty() || id.len() > 12 || !id.bytes().all(|b| b.is_ascii_digit()) {
        return (format!("line{}", line_no), failed("BAD_ID lemma=-"));
    }
    let proof_path = dir.join(format!("{}.drup", id));
    let proof_text = match fs::read_to_string(&proof_path) {
        Ok(text) => text,
        Err(e) if e.kind() == io::ErrorKind::NotFound => {
            return (id.to_string(), failed("NO_PROOF_FILE lemma=-"));
        }
        Err(_) => return (id.to_string(), failed("READ lemma=-")),
    };
    let expected = if fields[2].trim() == "-" {
        None
    } else {
        Some(fields[2])
    };
    (id.to_string(), evaluate(fields[1], &proof_text, expected))
}

/// `batch <boards.tsv> <proof-dir>`
fn cmd_batch(args: &[String], out: &mut dyn Write) -> io::Result<u8> {
    if args.len() != 2 {
        return usage();
    }
    let tsv = match fs::read_to_string(&args[0]) {
        Ok(text) => text,
        Err(_) => {
            writeln!(out, "FAILED READ {}", args[0])?;
            return Ok(EXIT_FAILED);
        }
    };
    let dir = Path::new(&args[1]);
    let mut boards = 0usize;
    let mut n_verified = 0usize;
    let mut n_open = 0usize;
    let mut n_failed = 0usize;
    for (index, line) in tsv.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        boards += 1;
        let (label, outcome) = batch_one(line, index + 1, dir);
        match &outcome {
            Outcome::Verified { .. } => n_verified += 1,
            Outcome::Open { .. } => n_open += 1,
            Outcome::Failed { .. } => n_failed += 1,
        }
        writeln!(out, "{}", render_batch(&label, &outcome))?;
    }
    writeln!(
        out,
        "TOTAL boards={} verified={} open={} failed={}",
        boards, n_verified, n_open, n_failed
    )?;
    Ok(if n_failed == 0 { EXIT_VERIFIED } else { EXIT_FAILED })
}

/// `dataset <test.csv> <proof-dir>`: every `<id>.drup` in the directory is the proof for data line `<id>` of the
/// dataset file (the header is line 0, ids have no leading zero). The puzzle is that line's `question` column and the
/// derived grid must equal its `answer` column, so `VERIFIED` means the proof holds and its unique solution is the
/// dataset's answer. Lines are split on commas; a line holding a quote character is refused.
fn cmd_dataset(args: &[String], out: &mut dyn Write) -> io::Result<u8> {
    if args.len() != 2 {
        return usage();
    }
    let csv = match fs::read_to_string(&args[0]) {
        Ok(text) => text,
        Err(_) => {
            writeln!(out, "FAILED READ {}", args[0])?;
            return Ok(EXIT_FAILED);
        }
    };
    let mut lines = csv.lines();
    let header: Vec<&str> = lines.next().unwrap_or("").split(',').map(str::trim).collect();
    let column = |name: &str| header.iter().position(|field| *field == name);
    let (Some(question), Some(answer)) = (column("question"), column("answer")) else {
        writeln!(out, "FAILED DATASET_COLUMNS {}", args[0])?;
        return Ok(EXIT_FAILED);
    };
    let data: Vec<&str> = lines.collect();
    let entries = match fs::read_dir(&args[1]) {
        Ok(entries) => entries,
        Err(_) => {
            writeln!(out, "FAILED READ {}", args[1])?;
            return Ok(EXIT_FAILED);
        }
    };
    let mut ids: Vec<u64> = Vec::new();
    for entry in entries.flatten() {
        let name = entry.file_name();
        let Some(stem) = name.to_str().and_then(|text| text.strip_suffix(".drup")) else {
            continue;
        };
        if stem.is_empty() || stem.len() > 12 || stem.starts_with('0') || !stem.bytes().all(|b| b.is_ascii_digit()) {
            continue;
        }
        if let Ok(id) = stem.parse::<u64>() {
            ids.push(id);
        }
    }
    ids.sort_unstable();
    let dir = Path::new(&args[1]);
    let (mut n_verified, mut n_open, mut n_failed) = (0usize, 0usize, 0usize);
    for &id in &ids {
        let outcome = dataset_one(id, &data, question, answer, header.len(), dir);
        match &outcome {
            Outcome::Verified { .. } => n_verified += 1,
            Outcome::Open { .. } => n_open += 1,
            Outcome::Failed { .. } => n_failed += 1,
        }
        writeln!(out, "{}", render_dataset(id, &outcome))?;
    }
    writeln!(
        out,
        "TOTAL boards={} verified={} open={} failed={}",
        ids.len(),
        n_verified,
        n_open,
        n_failed
    )?;
    Ok(if n_failed == 0 { EXIT_VERIFIED } else { EXIT_FAILED })
}

/// Checks the proof of data line `id` against the dataset's puzzle and answer columns.
fn dataset_one(id: u64, data: &[&str], question: usize, answer: usize, width: usize, dir: &Path) -> Outcome {
    let line = match usize::try_from(id).ok().and_then(|k| k.checked_sub(1)).and_then(|k| data.get(k)) {
        Some(line) => *line,
        None => return failed("NOT_IN_DATASET lemma=-"),
    };
    if line.contains('"') {
        return failed("DATASET_QUOTE lemma=-");
    }
    let fields: Vec<&str> = line.split(',').collect();
    if fields.len() != width {
        return failed("DATASET_FIELDS lemma=-");
    }
    let proof_text = match fs::read_to_string(dir.join(format!("{}.drup", id))) {
        Ok(text) => text,
        Err(_) => return failed("READ lemma=-"),
    };
    evaluate(fields[question], &proof_text, Some(fields[answer]))
}

fn render_dataset(id: u64, outcome: &Outcome) -> String {
    match outcome {
        Outcome::Verified { lemmas, .. } => format!("{}\tVERIFIED\tmatches_answer lemmas={}", id, lemmas),
        Outcome::Open { decided, .. } => format!("{}\tOPEN\tdecided={}", id, decided),
        Outcome::Failed { detail } => format!("{}\tFAILED\t{}", id, detail),
    }
}

/// `dimacs <puzzle81> <grid81>`
fn cmd_dimacs(args: &[String], out: &mut dyn Write) -> io::Result<u8> {
    if args.len() != 2 {
        return usage();
    }
    let puzzle = match parse_puzzle(&args[0]) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("FAILED PARSE {} lemma=-", e);
            return Ok(EXIT_FAILED);
        }
    };
    let grid = match parse_grid(&args[1]) {
        Ok(g) => g,
        Err(e) => {
            eprintln!("FAILED PARSE {} lemma=-", e);
            return Ok(EXIT_FAILED);
        }
    };
    out.write_all(dimacs_with_block(&puzzle, &grid).as_bytes())?;
    Ok(EXIT_VERIFIED)
}

/// `drat <proof-file>`
fn cmd_drat(args: &[String], out: &mut dyn Write) -> io::Result<u8> {
    if args.len() != 1 {
        return usage();
    }
    let text = match fs::read_to_string(&args[0]) {
        Ok(text) => text,
        Err(_) => {
            eprintln!("FAILED READ {}", args[0]);
            return Ok(EXIT_FAILED);
        }
    };
    let lemmas = match parse_proof(&text) {
        Ok(l) => l,
        Err(e) => {
            eprintln!("FAILED PARSE {} lemma=-", e);
            return Ok(EXIT_FAILED);
        }
    };
    out.write_all(drat_text(&lemmas).as_bytes())?;
    Ok(EXIT_VERIFIED)
}

fn run(args: &[String], out: &mut dyn Write) -> io::Result<u8> {
    let Some((command, rest)) = args.split_first() else {
        return usage();
    };
    match command.as_str() {
        "check" => cmd_check(rest, out),
        "batch" => cmd_batch(rest, out),
        "dataset" => cmd_dataset(rest, out),
        "dimacs" => cmd_dimacs(rest, out),
        "drat" => cmd_drat(rest, out),
        _ => usage(),
    }
}

fn main() -> ExitCode {
    let mut args: Vec<String> = Vec::new();
    for arg in std::env::args_os().skip(1) {
        match arg.into_string() {
            Ok(text) => args.push(text),
            Err(_) => {
                eprintln!("{}", USAGE);
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
