//! Integration tests. Each test is one "cell" and prints a single
//! `CELL <name> GREEN ...` line when it passes (run with `--nocapture` to see them).
//!
//! Run them with `cargo test --release -- --nocapture`: the proofs are computed once in the test
//! process and again by the binary, and a release build does that in a fraction of the time.
//!
//! The puzzles are published ones; the source of each is next to its constant. No answer is typed
//! in: a short exhaustive fill (test-only code below, ahead of the cells) computes every answer
//! and checks that the puzzle has exactly one solution. `fixed_rule_is_pinned` pins, for every hard
//! puzzle, the lemma counts, the step counts and an FNV-1a hash of the DRUP text; they change only
//! when the fixed rule changes.
//!
//! Each MUTANT line is a change of `src/lib.rs` (a `sed` expression, basic regular expressions,
//! that matches exactly one line) and a cell that must turn red when it is applied. A lemma that is
//! not RUP where it is written stops a debug build at a `debug_assert!` and a release build at the
//! final self-check; either way the cell is red.
//!
//! Depth 1 and depth 2 close every published hard puzzle below, so `prove` does not reach its split
//! step on them. The split step is covered by the unit test `depth_one_and_split_prove_the_hard_puzzles`
//! in `src/lib.rs` (the loop of `prove` without depth 2, on three of these puzzles) and by the two
//! cells that refuse a puzzle with a second solution: that refusal is raised inside the split step.
//! So the split mutants below are killed by those cells, and by that unit test (run `cargo test`).
//!
//! MUTANT depth1_rup_test_dropped: src/lib.rs: 's/if st.db.rup(&unit) {/if true {/'   (hard_puzzles_are_proved_and_checked)
//! MUTANT refuted_skip_dropped: src/lib.rs: 's/if st.db.value(x) == -1 {/if false {/'   (no_trivial_unit)
//! MUTANT trim_removes_every_y: src/lib.rs: 's/if trial.rup(&unit) {/if true {/'   (hard_puzzles_are_proved_and_checked)
//! MUTANT split_base_case_false: src/lib.rs: 's/if st.db.rup(clause) {/if false {/'   (another_solution_is_found_after_splitting)
//! MUTANT split_depth_cap_zero: src/lib.rs: 's/if depth == MAX_SPLIT_DEPTH {/if depth == 0 {/'   (another_solution_is_refused_by_name)
//! MUTANT right_literal_counted_wrong: src/lib.rs: 's/if d != usize/if true || d != usize/'   (another_solution_is_refused_by_name)

use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::sync::OnceLock;

use sudoku_rup_check::{
    Db, Verdict, check_board, dimacs_with_block, format_lemma, grid_to_string, parse_grid,
    parse_proof, parse_puzzle, rules_hold, sudoku_cnf, var,
};
use sudoku_rup_prover::{Proof, prove};

/// The example puzzle of the Wikipedia article on Sudoku. Unit propagation alone decides it.
const EASY: &str =
    "53..7....6..195....98....6.8...6...34..8.3..17...2...6.6....28....419..5....8..79";

/// A published hard puzzle and what the fixed rule writes for it. The counts and the hash come
/// from a second, independent implementation of the rule.
#[derive(Clone, Copy)]
struct Hard {
    name: &'static str,
    text: &'static str,
    lemmas: usize,
    units: usize,
    binaries: usize,
    longer: usize,
    depth2_rounds: usize,
    split_rounds: usize,
    fnv: u64,
}

const HARD: [Hard; 4] = [
    // Peter Norvig's essay on Sudoku (norvig.com/sudoku.html): the hard grid of the essay, also
    // the first line of his list top95.txt (norvig.com/top95.txt). Seventeen givens.
    Hard {
        name: "norvig_top95_1",
        text: "4.....8.5.3..........7......2.....6.....8.4......1.......6.3.7.5..2.....1.4......",
        lemmas: 16,
        units: 16,
        binaries: 0,
        longer: 0,
        depth2_rounds: 0,
        split_rounds: 0,
        fnv: 0xff1a_bf54_6a4c_6ffd,
    },
    // "AI Escargot", Arto Inkala, 2006; widely published.
    Hard {
        name: "ai_escargot",
        text: "1....7.9..3..2...8..96..5....53..9...1..8...26....4...3......1..4......7..7...3..",
        lemmas: 35,
        units: 19,
        binaries: 16,
        longer: 0,
        depth2_rounds: 3,
        split_rounds: 0,
        fnv: 0xa45a_84c7_d3e6_a31e,
    },
    // Arto Inkala's puzzle of 2012 ("the world's hardest sudoku"); widely published.
    Hard {
        name: "inkala_2012",
        text: "8..........36......7..9.2...5...7.......457.....1...3...1....68..85...1..9....4..",
        lemmas: 85,
        units: 52,
        binaries: 33,
        longer: 0,
        depth2_rounds: 5,
        split_rounds: 0,
        fnv: 0xc082_89cc_aaf0_a913,
    },
    // "Easter Monster", Sudoku Players' Forum; widely published (for example it is listed
    // under that name in sudoku/sudoku.txt of the repository attractivechaos/plb).
    Hard {
        name: "easter_monster",
        text: "1.......2.9.4...5...6...7...5.9.3.......7.......85..4.7.....6...3...9.8...2.....1",
        lemmas: 102,
        units: 37,
        binaries: 65,
        longer: 0,
        depth2_rounds: 11,
        split_rounds: 0,
        fnv: 0xb128_ae6b_3a04_a5f1,
    },
];

const HARD_COUNT: usize = HARD.len();

/// A hard puzzle with its answer and the proof the library wrote for it.
struct Proved {
    name: &'static str,
    puzzle: [u8; 81],
    answer: [u8; 81],
    proof: Proof,
}

static PROVED: [OnceLock<Proved>; HARD_COUNT] = [const { OnceLock::new() }; HARD_COUNT];

/// The proof of hard puzzle `i`, computed once per test process.
fn proved(i: usize) -> &'static Proved {
    PROVED[i].get_or_init(|| {
        let spec: Hard = HARD[i];
        let (puzzle, answer) = board(spec.text);
        let proof = prove(&puzzle, &answer)
            .unwrap_or_else(|error| panic!("{}: no proof: {error}", spec.name));
        Proved {
            name: spec.name,
            puzzle,
            answer,
            proof,
        }
    })
}

/// The hard puzzle with the shortest proof among those that use the depth 2 step, so that the
/// cells that prove again (and run the binary) stay short.
fn cheapest() -> usize {
    (0..HARD_COUNT)
        .min_by_key(|&i| {
            let proof = &proved(i).proof;
            (proof.depth2_rounds == 0, proof.lemmas.len())
        })
        .expect("the hard list is not empty")
}

/// The proof as DRUP text: one lemma per line, closed by `0`.
fn drup_text(lemmas: &[Vec<i32>]) -> String {
    lemmas
        .iter()
        .map(|lemma| format!("{}\n", format_lemma(lemma)))
        .collect()
}

/// FNV-1a over the bytes of `text`.
fn fnv1a(text: &str) -> u64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for &byte in text.as_bytes() {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

/// Reads the clause lines of a DIMACS text (everything but `p` and `c` lines).
fn read_dimacs(text: &str) -> Vec<Vec<i32>> {
    let mut clauses: Vec<Vec<i32>> = Vec::new();
    for line in text.lines() {
        if line.starts_with('p') || line.starts_with('c') {
            continue;
        }
        let mut lits: Vec<i32> = line
            .split_whitespace()
            .map(|t| t.parse().expect("integer token"))
            .collect();
        assert_eq!(lits.pop(), Some(0), "a clause line ends with 0");
        clauses.push(lits);
    }
    clauses
}

/// What `drat-trim` is asked: the puzzle's clauses plus the clause that forbids `answer`, then
/// the lemmas. Unit propagation must reach a conflict, so that the empty clause is RUP.
fn drat_view_refutes(puzzle: &[u8; 81], answer: &[u8; 81], lemmas: &[Vec<i32>]) -> bool {
    let mut db = Db::new(read_dimacs(&dimacs_with_block(puzzle, answer))).expect("in range");
    for lemma in lemmas {
        if !db.rup(lemma) {
            return false;
        }
        db.add(lemma.clone()).expect("in range");
    }
    db.base_conflict()
}

/// A fresh empty directory for one cell.
fn temp_dir(tag: &str) -> PathBuf {
    let dir =
        std::env::temp_dir().join(format!("sudoku-rup-prover-{}-{}", tag, std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("create the temp dir");
    dir
}

/// The command line front end.
fn prover() -> Command {
    Command::new(env!("CARGO_BIN_EXE_sudoku-rup-prover"))
}

// ---------------------------------------------------------------------------------------------
// Test-only exhaustive fill: answers for the published puzzles, and a check that each puzzle has
// exactly one solution. Depth first over the blank cell with the fewest candidates.

struct Fill {
    grid: [u8; 81],
    rows: [u16; 9],
    cols: [u16; 9],
    boxes: [u16; 9],
    count: usize,
    limit: usize,
    first: Option<[u8; 81]>,
}

fn box_of(cell: usize) -> usize {
    3 * (cell / 27) + (cell % 9) / 3
}

impl Fill {
    fn place(&mut self, cell: usize, digit: u8) {
        let bit = 1u16 << digit;
        self.grid[cell] = digit;
        self.rows[cell / 9] |= bit;
        self.cols[cell % 9] |= bit;
        self.boxes[box_of(cell)] |= bit;
    }

    fn lift(&mut self, cell: usize, digit: u8) {
        let bit = 1u16 << digit;
        self.grid[cell] = 0;
        self.rows[cell / 9] &= !bit;
        self.cols[cell % 9] &= !bit;
        self.boxes[box_of(cell)] &= !bit;
    }

    fn run(&mut self) {
        if self.count >= self.limit {
            return;
        }
        // The blank cell with the fewest candidates.
        let mut best: Option<(usize, u16, u32)> = None;
        for cell in 0..81usize {
            if self.grid[cell] != 0 {
                continue;
            }
            let used = self.rows[cell / 9] | self.cols[cell % 9] | self.boxes[box_of(cell)];
            let free = !used & 0b11_1111_1110;
            let n = free.count_ones();
            let better = match best {
                None => true,
                Some((_, _, fewest)) => n < fewest,
            };
            if better {
                best = Some((cell, free, n));
            }
        }
        match best {
            None => {
                self.count += 1;
                if self.first.is_none() {
                    self.first = Some(self.grid);
                }
            }
            Some((cell, free, _)) => {
                for digit in 1..=9u8 {
                    if free & (1u16 << digit) == 0 {
                        continue;
                    }
                    self.place(cell, digit);
                    self.run();
                    self.lift(cell, digit);
                    if self.count >= self.limit {
                        return;
                    }
                }
            }
        }
    }
}

/// Counts the completions of `puzzle` up to `limit` and returns the first one found.
fn completions(puzzle: &[u8; 81], limit: usize) -> (usize, Option<[u8; 81]>) {
    let mut fill = Fill {
        grid: [0; 81],
        rows: [0; 9],
        cols: [0; 9],
        boxes: [0; 9],
        count: 0,
        limit,
        first: None,
    };
    for (cell, &digit) in puzzle.iter().enumerate() {
        if digit == 0 {
            continue;
        }
        let bit = 1u16 << digit;
        let clash = fill.rows[cell / 9] & bit != 0
            || fill.cols[cell % 9] & bit != 0
            || fill.boxes[box_of(cell)] & bit != 0;
        if clash {
            return (0, None);
        }
        fill.place(cell, digit);
    }
    fill.run();
    (fill.count, fill.first)
}

/// Parses `text`, computes its answer and checks that it has exactly one solution.
fn board(text: &str) -> ([u8; 81], [u8; 81]) {
    let puzzle = parse_puzzle(text).expect("valid puzzle text");
    let (count, first) = completions(&puzzle, 2);
    assert_eq!(
        count, 1,
        "the puzzle must have exactly one solution: {text}"
    );
    (puzzle, first.expect("a solution was found"))
}

// ---------------------------------------------------------------------------------------------

#[test]
fn easy_puzzle_needs_no_lemma() {
    let (puzzle, answer) = board(EASY);
    let proof = prove(&puzzle, &answer).expect("the Wikipedia puzzle is provable");
    assert!(
        proof.lemmas.is_empty(),
        "unit propagation over the CNF decides this puzzle, got {} lemmas",
        proof.lemmas.len()
    );
    assert_eq!((proof.units, proof.binaries, proof.longer), (0, 0, 0));
    assert_eq!((proof.depth2_rounds, proof.split_rounds), (0, 0));
    assert_eq!(
        check_board(&puzzle, &proof.lemmas, Some(&answer)),
        Verdict::Verified {
            grid: answer,
            lemmas: 0
        }
    );
    assert!(drat_view_refutes(&puzzle, &answer, &proof.lemmas));
    println!("CELL easy_puzzle_needs_no_lemma GREEN");
}

#[test]
fn hard_puzzles_are_proved_and_checked() {
    let mut summary = String::new();
    for i in 0..HARD_COUNT {
        let p = proved(i);
        let proof = &p.proof;
        assert!(
            !proof.lemmas.is_empty(),
            "{}: a hard puzzle needs lemmas",
            p.name
        );
        assert_eq!(
            proof.units + proof.binaries + proof.longer,
            proof.lemmas.len(),
            "{}: the counts add up",
            p.name
        );
        for lemma in &proof.lemmas {
            assert!(!lemma.is_empty(), "{}: an empty lemma", p.name);
            assert!(
                lemma.windows(2).all(|pair| pair[0] < pair[1]),
                "{}: lemma not strictly ascending: {:?}",
                p.name,
                lemma
            );
        }
        match check_board(&p.puzzle, &proof.lemmas, Some(&p.answer)) {
            Verdict::Verified { grid, lemmas } => {
                assert_eq!(grid, p.answer, "{}: the decided grid", p.name);
                assert_eq!(lemmas, proof.lemmas.len(), "{}: the lemma count", p.name);
            }
            other => panic!("{}: expected Verified, got {:?}", p.name, other),
        }
        // The proof text parses back to the same lemmas.
        let text = drup_text(&proof.lemmas);
        assert_eq!(
            parse_proof(&text).expect("the DRUP text parses"),
            proof.lemmas,
            "{}: text round trip",
            p.name
        );
        // The same claim in the form `drat-trim` is given.
        assert!(
            drat_view_refutes(&p.puzzle, &p.answer, &proof.lemmas),
            "{}: the DRAT view does not refute the blocking clause",
            p.name
        );
        summary.push_str(&format!(
            " {}:lemmas={},units={},binaries={},longer={},depth2={},split={}",
            p.name,
            proof.lemmas.len(),
            proof.units,
            proof.binaries,
            proof.longer,
            proof.depth2_rounds,
            proof.split_rounds
        ));
    }
    println!("CELL hard_puzzles_are_proved_and_checked GREEN{summary}");
}

#[test]
fn no_trivial_unit() {
    let mut units_seen = 0usize;
    for i in 0..HARD_COUNT {
        let p = proved(i);
        let mut db = Db::new(sudoku_cnf(&p.puzzle)).expect("generated CNF is in range");
        let mut units = 0usize;
        for (k, lemma) in p.proof.lemmas.iter().enumerate() {
            if let [literal] = lemma.as_slice() {
                assert!(*literal < 0, "{}: unit lemma {k} is {lemma:?}", p.name);
                assert_eq!(
                    db.value(*literal),
                    0,
                    "{}: unit lemma {k} ({lemma:?}) was already decided when it was added",
                    p.name
                );
                units += 1;
            }
            db.add(lemma.clone()).expect("literal in range");
        }
        assert_eq!(units, p.proof.units, "{}: unit count", p.name);
        units_seen += units;
    }
    println!("CELL no_trivial_unit GREEN units={units_seen}");
}

#[test]
fn every_lemma_is_rup_at_its_place() {
    let mut lemmas_seen = 0usize;
    for i in 0..HARD_COUNT {
        let p = proved(i);
        let mut db = Db::new(sudoku_cnf(&p.puzzle)).expect("generated CNF is in range");
        for (k, lemma) in p.proof.lemmas.iter().enumerate() {
            assert!(
                db.rup(lemma),
                "{}: lemma {k} is not RUP where it is added: {lemma:?}",
                p.name
            );
            db.add(lemma.clone()).expect("literal in range");
            lemmas_seen += 1;
        }
        assert!(
            !db.base_conflict(),
            "{}: the database stays consistent",
            p.name
        );
        // After the last lemma unit propagation decides every cell to the answer.
        for (cell, &digit) in p.answer.iter().enumerate() {
            assert_eq!(
                db.value(var(cell / 9, cell % 9, usize::from(digit))),
                1,
                "{}: cell {cell} is not decided",
                p.name
            );
        }
    }
    println!("CELL every_lemma_is_rup_at_its_place GREEN lemmas={lemmas_seen}");
}

#[test]
fn deterministic() {
    let i = cheapest();
    let p = proved(i);
    // Two calls give the same lemmas, and the same counts.
    let again = prove(&p.puzzle, &p.answer).expect("proved again");
    assert_eq!(
        again.lemmas, p.proof.lemmas,
        "{}: lemma lists differ",
        p.name
    );
    assert_eq!(again, p.proof, "{}: proofs differ", p.name);
    // The command line writes exactly the library's lemmas.
    let output = prover()
        .arg("prove")
        .arg(grid_to_string(&p.puzzle))
        .arg(grid_to_string(&p.answer))
        .output()
        .expect("the prover binary starts");
    assert_eq!(output.status.code(), Some(0), "{}: exit status", p.name);
    assert!(output.stderr.is_empty(), "{}: nothing on stderr", p.name);
    let stdout = String::from_utf8(output.stdout).expect("utf-8 output");
    assert_eq!(
        stdout,
        drup_text(&p.proof.lemmas),
        "{}: command line output",
        p.name
    );
    println!(
        "CELL deterministic GREEN puzzle={} lemmas={}",
        p.name,
        p.proof.lemmas.len()
    );
}

#[test]
fn fixed_rule_is_pinned() {
    for (i, spec) in HARD.iter().enumerate() {
        let proof = &proved(i).proof;
        let got = (
            proof.lemmas.len(),
            proof.units,
            proof.binaries,
            proof.longer,
            proof.depth2_rounds,
            proof.split_rounds,
        );
        let want = (
            spec.lemmas,
            spec.units,
            spec.binaries,
            spec.longer,
            spec.depth2_rounds,
            spec.split_rounds,
        );
        assert_eq!(got, want, "{}: counts of the fixed rule changed", spec.name);
        assert_eq!(
            fnv1a(&drup_text(&proof.lemmas)),
            spec.fnv,
            "{}: the lemma list of the fixed rule changed",
            spec.name
        );
    }
    println!("CELL fixed_rule_is_pinned GREEN puzzles={HARD_COUNT}");
}

#[test]
fn hard_set_uses_depth_one_and_two() {
    // The mutants of the depth 1 and depth 2 steps can only be killed if the hard set uses both.
    let units: usize = (0..HARD_COUNT).map(|i| proved(i).proof.units).sum();
    let binaries: usize = (0..HARD_COUNT).map(|i| proved(i).proof.binaries).sum();
    let depth2: usize = (0..HARD_COUNT).map(|i| proved(i).proof.depth2_rounds).sum();
    assert!(units > 0, "no hard puzzle gets a unit lemma");
    assert!(depth2 > 0, "no hard puzzle uses the depth 2 step");
    assert!(binaries > 0, "no hard puzzle gets a binary lemma");
    println!(
        "CELL hard_set_uses_depth_one_and_two GREEN units={units} binaries={binaries} depth2_rounds={depth2}"
    );
}

#[test]
fn bad_answers_are_refused_by_name() {
    let (puzzle, answer) = board(EASY);

    // Two digits swapped in row 0, in cells that are blank in the puzzle: the givens still hold,
    // but a column repeats a digit.
    assert_eq!((puzzle[2], puzzle[3]), (0, 0));
    let mut swapped = answer;
    swapped.swap(2, 3);
    assert_ne!(swapped, answer);
    let error = prove(&puzzle, &swapped).expect_err("swapped digits are no solution");
    assert_eq!(error.reason, "ANSWER_RULES", "{error}");

    // A given changed.
    let mut changed = answer;
    changed[0] = changed[0] % 9 + 1;
    assert_ne!(changed[0], puzzle[0]);
    let error = prove(&puzzle, &changed).expect_err("a changed given is no solution");
    assert_eq!(error.reason, "ANSWER_VS_GIVENS", "{error}");

    // A 0 digit, and a digit above 9.
    let mut zero = answer;
    zero[40] = 0;
    let error = prove(&puzzle, &zero).expect_err("a 0 digit is no answer");
    assert_eq!(error.reason, "ANSWER_FORM", "{error}");
    let mut big = answer;
    big[40] = 10;
    let error = prove(&puzzle, &big).expect_err("a digit above 9 is no answer");
    assert_eq!(error.reason, "ANSWER_FORM", "{error}");

    // Nothing is refused when the answer is right.
    assert!(rules_hold(&puzzle, &answer));
    assert!(prove(&puzzle, &answer).is_ok());
    println!("CELL bad_answers_are_refused_by_name GREEN");
}

#[test]
fn another_solution_is_refused_by_name() {
    // Cells (0,3), (0,4), (3,3), (3,4) of the Wikipedia solution hold 6 7 / 7 6: blank them and
    // the swapped grid is a second solution.
    let (full, answer) = board(EASY);
    assert!(rules_hold(&full, &answer));
    let cells = [3usize, 4, 30, 31];
    let mut puzzle = answer;
    for &cell in &cells {
        puzzle[cell] = 0;
    }
    let mut other = answer;
    other.swap(3, 4);
    other.swap(30, 31);
    assert!(rules_hold(&puzzle, &answer));
    assert!(rules_hold(&puzzle, &other));
    assert_eq!(completions(&puzzle, 3).0, 2, "exactly two solutions");

    let error = prove(&puzzle, &answer).expect_err("a puzzle with two solutions has no proof");
    assert_eq!(error.reason, "NOT_UNIQUE", "{error}");
    assert_eq!(
        error.detail,
        format!("another solution: {}", grid_to_string(&other))
    );
    println!("CELL another_solution_is_refused_by_name GREEN");
}

#[test]
fn another_solution_is_found_after_splitting() {
    // "AI Escargot" without its given 9 at row 2, column 2 (cell 20) has many solutions. The rule
    // refutes what it can with depth 1 and depth 2, then splits; the nested splits end in a
    // complete grid that is not the answer. Which grid that is depends on every branch choice of
    // the split step, so the grid is pinned.
    assert_eq!(HARD[1].name, "ai_escargot");
    let (full, answer) = board(HARD[1].text);
    assert_eq!(full[20], 9);
    let mut puzzle = full;
    puzzle[20] = 0;
    assert!(
        completions(&puzzle, 2).0 >= 2,
        "the puzzle has more than one solution"
    );

    let error = prove(&puzzle, &answer).expect_err("a puzzle with many solutions has no proof");
    assert_eq!(error.reason, "NOT_UNIQUE", "{error}");
    let text = error
        .detail
        .strip_prefix("another solution: ")
        .expect("the detail names the grid");
    let other = parse_grid(text).expect("the detail holds a grid of 81 digits");
    assert!(
        rules_hold(&puzzle, &other),
        "the grid in the detail is a solution"
    );
    assert_ne!(other, answer);
    assert_eq!(
        text,
        "128537496536429178794618523475362981913785642682194735359276814841953267267841359"
    );
    println!("CELL another_solution_is_found_after_splitting GREEN");
}

#[test]
fn batch_writes_one_file_per_board_and_a_total() {
    let dir = temp_dir("batch");
    let (easy_puzzle, easy_answer) = board(EASY);
    let p = proved(cheapest());
    let tsv = format!(
        "1\t{}\t{}\n2\t{}\t{}\n",
        grid_to_string(&easy_puzzle),
        grid_to_string(&easy_answer),
        grid_to_string(&p.puzzle),
        grid_to_string(&p.answer)
    );
    let tsv_path = dir.join("boards.tsv");
    fs::write(&tsv_path, tsv).expect("write the boards file");
    let out_dir = dir.join("proofs");
    fs::create_dir(&out_dir).expect("create the output directory");

    let output = prover()
        .arg("batch")
        .arg(&tsv_path)
        .arg(&out_dir)
        .output()
        .expect("the prover binary starts");
    assert_eq!(output.status.code(), Some(0));
    let stdout = String::from_utf8(output.stdout).expect("utf-8 output");
    let lines: Vec<&str> = stdout.lines().collect();
    assert_eq!(lines.len(), 3, "{stdout}");
    assert_eq!(lines[0], "1\tPROVED\tlemmas=0 units=0 binaries=0 longer=0");
    assert_eq!(
        lines[1],
        format!(
            "2\tPROVED\tlemmas={} units={} binaries={} longer={}",
            p.proof.lemmas.len(),
            p.proof.units,
            p.proof.binaries,
            p.proof.longer
        )
    );
    assert_eq!(lines[2], "TOTAL boards=2 proved=2 failed=0");

    // One file per board, and nothing else.
    let mut names: Vec<String> = fs::read_dir(&out_dir)
        .expect("list the output directory")
        .map(|entry| {
            entry
                .expect("directory entry")
                .file_name()
                .to_string_lossy()
                .into_owned()
        })
        .collect();
    names.sort();
    assert_eq!(names, vec!["1.drup".to_string(), "2.drup".to_string()]);
    assert_eq!(
        fs::read_to_string(out_dir.join("1.drup")).expect("read 1.drup"),
        ""
    );
    let text = fs::read_to_string(out_dir.join("2.drup")).expect("read 2.drup");
    assert_eq!(text, drup_text(&p.proof.lemmas));
    // The checker's own parser and board check accept the file.
    let lemmas = parse_proof(&text).expect("the file parses");
    assert_eq!(lemmas, p.proof.lemmas);
    assert!(matches!(
        check_board(&p.puzzle, &lemmas, Some(&p.answer)),
        Verdict::Verified { .. }
    ));
    let _ = fs::remove_dir_all(&dir);
    println!(
        "CELL batch_writes_one_file_per_board_and_a_total GREEN puzzle={} lemmas={}",
        p.name,
        p.proof.lemmas.len()
    );
}

#[test]
fn batch_reports_a_refusal_and_never_overwrites() {
    let dir = temp_dir("refusal");
    let (puzzle, answer) = board(EASY);
    let mut swapped = answer;
    swapped.swap(2, 3);
    let puzzle_text = grid_to_string(&puzzle);
    let answer_text = grid_to_string(&answer);
    let swapped_text = grid_to_string(&swapped);
    let tsv = format!(
        "7\t{puzzle_text}\t{answer_text}\n\
         8\t{puzzle_text}\t{swapped_text}\n\
         9\t{puzzle_text}\t{answer_text}\n\
         10\t{puzzle_text}\n\
         ab\t{puzzle_text}\t{answer_text}\n"
    );
    let tsv_path = dir.join("boards.tsv");
    fs::write(&tsv_path, tsv).expect("write the boards file");
    let out_dir = dir.join("proofs");
    fs::create_dir(&out_dir).expect("create the output directory");
    fs::write(out_dir.join("7.drup"), "keep me\n").expect("write the existing file");

    let output = prover()
        .arg("batch")
        .arg(&tsv_path)
        .arg(&out_dir)
        .output()
        .expect("the prover binary starts");
    assert_eq!(output.status.code(), Some(2));
    let stdout = String::from_utf8(output.stdout).expect("utf-8 output");
    let want = [
        "7\tFAILED\tOUT_EXISTS",
        "8\tFAILED\tANSWER_RULES",
        "9\tPROVED\tlemmas=0 units=0 binaries=0 longer=0",
        "line4\tFAILED\tBAD_LINE",
        "line5\tFAILED\tBAD_ID",
        "TOTAL boards=5 proved=1 failed=4",
    ];
    let lines: Vec<&str> = stdout.lines().collect();
    assert_eq!(lines, want, "{stdout}");

    // The existing file is untouched, a refused board leaves no file, a proved board has one.
    assert_eq!(
        fs::read_to_string(out_dir.join("7.drup")).expect("read 7.drup"),
        "keep me\n"
    );
    let mut names: Vec<String> = fs::read_dir(&out_dir)
        .expect("list the output directory")
        .map(|entry| {
            entry
                .expect("directory entry")
                .file_name()
                .to_string_lossy()
                .into_owned()
        })
        .collect();
    names.sort();
    assert_eq!(names, vec!["7.drup".to_string(), "9.drup".to_string()]);

    // A missing output directory is refused before any board is processed.
    let missing = dir.join("no-such-dir");
    let output = prover()
        .arg("batch")
        .arg(&tsv_path)
        .arg(&missing)
        .output()
        .expect("the prover binary starts");
    assert_eq!(output.status.code(), Some(2));
    assert!(output.stdout.is_empty());
    assert!(String::from_utf8_lossy(&output.stderr).starts_with("FAILED OUT_DIR "));

    // `prove` names its refusals on standard error and keeps standard output clean.
    let output = prover()
        .arg("prove")
        .arg(&puzzle_text)
        .arg(&swapped_text)
        .output()
        .expect("the prover binary starts");
    assert_eq!(output.status.code(), Some(2));
    assert!(output.stdout.is_empty());
    assert!(String::from_utf8_lossy(&output.stderr).starts_with("FAILED ANSWER_RULES "));
    let output = prover()
        .arg("prove")
        .arg(&puzzle_text)
        .arg(answer_text.replace('5', "0"))
        .output()
        .expect("the prover binary starts");
    assert_eq!(output.status.code(), Some(2));
    assert!(output.stdout.is_empty());
    assert!(String::from_utf8_lossy(&output.stderr).starts_with("FAILED ANSWER_FORM "));

    // Wrong usage exits with status 64.
    let output = prover().output().expect("the prover binary starts");
    assert_eq!(output.status.code(), Some(64));
    assert!(output.stdout.is_empty());
    assert!(String::from_utf8_lossy(&output.stderr).starts_with("usage: "));

    let _ = fs::remove_dir_all(&dir);
    println!("CELL batch_reports_a_refusal_and_never_overwrites GREEN");
}
