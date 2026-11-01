//! A public, deterministic generator of proofs that a Sudoku answer is the only solution.
//!
//! [`prove`] takes a puzzle and its answer, and nothing else. It writes a list of lemma clauses
//! (a DRUP proof) that the checker crate `sudoku-rup-check` accepts as a proof that the answer is
//! the only solution of the puzzle. The lemmas come from one fixed rule that looks only at the
//! clause database of the puzzle and the lemmas written so far; no order, table or hint comes from
//! anywhere else, so equal input always gives equal output.
//!
//! Variables and literals are those of the checker: `var(r, c, d) = 81 * r + 9 * c + d`, true when
//! cell `(r, c)` holds digit `d`. A "wrong" literal is `var(r, c, d)` for a digit `d` that differs
//! from the answer in that cell. To refute it is to make the unit clause `[-x]` hold.
//!
//! The rule, with every order ascending by variable number:
//!
//! 1. Refuse the input unless the answer is a full valid grid that keeps every given.
//! 2. Repeat until unit propagation decides every cell to the answer:
//!    * **depth 1**: a wrong literal whose unit clause is RUP gets that unit clause at once;
//!    * **depth 2**: otherwise, a wrong literal `x` is refuted by binary clauses `[-x, -y]`, each
//!      RUP on its own, trimmed to a set that is still enough, followed by `[-x]`;
//!    * **split**: otherwise the first open wrong literal is refuted by nested case splits
//!      (at most 16 deep) on the unassigned literal of the cell with the fewest open digits.
//! 3. Run the checker on the result; a proof it does not verify is never returned.
//!
//! Reason codes of [`ProveError::reason`]: `ANSWER_FORM`, `ANSWER_VS_GIVENS`, `ANSWER_RULES`,
//! `PUZZLE_CONTRADICTS`, `NOT_UNIQUE`, `SPLIT_DEPTH`, `SPLIT_NOT_CLOSED`, `LEMMA_CAP`,
//! `SELF_CHECK`, `INTERNAL_ERROR`.
//!
//! The crate uses only the standard library and the checker, and contains no `unsafe` code.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

use std::fmt;

use sudoku_rup_check::{Db, VARS, Verdict, check_board, rules_hold, sudoku_cnf, var};

/// Most lemmas a proof may hold.
const MAX_LEMMAS: usize = 200_000;

/// Most nested case splits on the way to one lemma.
const MAX_SPLIT_DEPTH: usize = 16;

const ANSWER_FORM: &str = "ANSWER_FORM";
const ANSWER_VS_GIVENS: &str = "ANSWER_VS_GIVENS";
const ANSWER_RULES: &str = "ANSWER_RULES";
const PUZZLE_CONTRADICTS: &str = "PUZZLE_CONTRADICTS";
const NOT_UNIQUE: &str = "NOT_UNIQUE";
const SPLIT_DEPTH: &str = "SPLIT_DEPTH";
const SPLIT_NOT_CLOSED: &str = "SPLIT_NOT_CLOSED";
const LEMMA_CAP: &str = "LEMMA_CAP";
const SELF_CHECK: &str = "SELF_CHECK";
const INTERNAL_ERROR: &str = "INTERNAL_ERROR";

/// A proof that an answer is the only solution of a puzzle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Proof {
    /// The lemmas in the order they were written. Each is sorted ascending and has no repeated
    /// literal, and is RUP with respect to the puzzle's clauses and the lemmas before it.
    pub lemmas: Vec<Vec<i32>>,
    /// How many lemmas have one literal.
    pub units: usize,
    /// How many lemmas have two literals.
    pub binaries: usize,
    /// How many lemmas have more than two literals.
    pub longer: usize,
    /// How many times the depth 2 step wrote lemmas.
    pub depth2_rounds: usize,
    /// How many times the split step wrote lemmas.
    pub split_rounds: usize,
}

/// Why no proof was produced.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProveError {
    /// Short upper-case reason code, for example `ANSWER_RULES`.
    pub reason: &'static str,
    /// One line of explanation; free text, not meant to be parsed.
    pub detail: String,
}

impl ProveError {
    /// Creates an error from a reason code and a one-line detail.
    pub fn new(reason: &'static str, detail: impl Into<String>) -> ProveError {
        ProveError {
            reason,
            detail: detail.into(),
        }
    }
}

impl fmt::Display for ProveError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.reason, self.detail)
    }
}

impl std::error::Error for ProveError {}

/// Wraps an error that cannot happen for CNF built by the checker.
fn internal(error: impl fmt::Display) -> ProveError {
    ProveError::new(INTERNAL_ERROR, error.to_string())
}

/// The growing clause database and the lemmas written so far.
struct State {
    db: Db,
    lemmas: Vec<Vec<i32>>,
}

impl State {
    /// Writes one lemma: sorts it ascending, drops repeats, adds it to the database and to the
    /// lemma list. The lemma must be RUP at this point (checked in debug builds, and again by the
    /// final self-check).
    fn emit(&mut self, mut lemma: Vec<i32>) -> Result<(), ProveError> {
        lemma.sort_unstable();
        lemma.dedup();
        if self.lemmas.len() >= MAX_LEMMAS {
            return Err(ProveError::new(
                LEMMA_CAP,
                format!("more than {MAX_LEMMAS} lemmas"),
            ));
        }
        debug_assert!(
            self.db.rup(&lemma),
            "lemma {lemma:?} is not RUP where it is added"
        );
        self.db.add(lemma.clone()).map_err(internal)?;
        if self.db.base_conflict() {
            return Err(ProveError::new(
                SELF_CHECK,
                "the clause database became contradictory",
            ));
        }
        self.lemmas.push(lemma);
        Ok(())
    }
}

/// `true` when unit propagation over the database already falsifies the positive literal `x`.
fn refuted(db: &Db, x: i32) -> bool {
    db.value(x) == -1
}

/// Step 1 of the rule: the answer must be a valid grid that keeps every given.
fn check_input(puzzle: &[u8; 81], answer: &[u8; 81]) -> Result<(), ProveError> {
    for (i, &digit) in answer.iter().enumerate() {
        if !(1..=9).contains(&digit) {
            return Err(ProveError::new(
                ANSWER_FORM,
                format!("cell {i}: the digit {digit} is not in 1..=9"),
            ));
        }
    }
    for (i, (&given, &digit)) in puzzle.iter().zip(answer.iter()).enumerate() {
        if given != 0 && given != digit {
            return Err(ProveError::new(
                ANSWER_VS_GIVENS,
                format!("cell {i}: the puzzle has {given}, the answer has {digit}"),
            ));
        }
    }
    if !rules_hold(puzzle, answer) {
        return Err(ProveError::new(
            ANSWER_RULES,
            "the answer repeats a digit in a row, a column or a box",
        ));
    }
    Ok(())
}

/// Every positive literal `var(r, c, d)` whose digit differs from the answer, ascending.
fn wrong_literals(answer: &[u8; 81]) -> Vec<i32> {
    let mut wrong = Vec::with_capacity(81 * 8);
    for r in 0..9usize {
        for c in 0..9usize {
            for d in 1..=9usize {
                if d != usize::from(answer[9 * r + c]) {
                    wrong.push(var(r, c, d));
                }
            }
        }
    }
    wrong
}

/// The answer's literal of every cell, in cell order.
fn answer_literals(answer: &[u8; 81]) -> [i32; 81] {
    std::array::from_fn(|i| var(i / 9, i % 9, usize::from(answer[i])))
}

/// Depth 1: every open wrong literal whose unit clause is RUP gets that unit clause, at once, so
/// later literals of the same pass see it. Returns how many lemmas the pass wrote.
fn depth1_pass(st: &mut State, wrong: &[i32]) -> Result<usize, ProveError> {
    let mut emitted = 0usize;
    for &x in wrong {
        // A literal that unit propagation already falsifies never gets a lemma.
        if st.db.value(x) == -1 {
            continue;
        }
        let unit = [-x];
        if st.db.rup(&unit) {
            st.emit(unit.to_vec())?;
            emitted += 1;
        }
    }
    Ok(emitted)
}

/// Depth 2: the first open wrong literal `x` that binary clauses `[-x, -y]` refute. Writes the
/// trimmed binary clauses and then `[-x]`; returns `true` if it wrote anything.
fn depth2_round(st: &mut State, wrong: &[i32]) -> Result<bool, ProveError> {
    for &x in wrong {
        if refuted(&st.db, x) {
            continue;
        }
        let Some(under_x) = st.db.assignment_under(&[x]) else {
            continue;
        };
        // Every literal that `x` leaves open and that cannot hold together with `x`.
        let mut ys: Vec<i32> = Vec::new();
        for y in 1..=(VARS as i32) {
            if under_x[y as usize] == 0 && st.db.rup(&[-x, -y]) {
                ys.push(y);
            }
        }
        if ys.is_empty() {
            continue;
        }
        let unit = [-x];
        let mut all = st.db.clone();
        for &y in &ys {
            all.add(vec![-x, -y]).map_err(internal)?;
        }
        if !all.rup(&unit) {
            continue;
        }
        // Trim: drop each y, in ascending order, whose binary clause the others make unnecessary.
        let mut keep = ys.clone();
        for &y in &ys {
            let mut trial = st.db.clone();
            for &k in &keep {
                if k != y {
                    trial.add(vec![-x, -k]).map_err(internal)?;
                }
            }
            if trial.rup(&unit) {
                keep.retain(|&k| k != y);
            }
        }
        for &y in &keep {
            st.emit(vec![-x, -y])?;
        }
        st.emit(unit.to_vec())?;
        return Ok(true);
    }
    Ok(false)
}

/// The positive literal to split on: unassigned in `assign`, from the cell with the fewest
/// unassigned digits, ties to the smallest literal. `None` when every variable is assigned.
fn branch_literal(assign: &[i8]) -> Option<i32> {
    let mut best: Option<(usize, i32)> = None;
    for cell in 0..81usize {
        let mut open = 0usize;
        let mut first = 0i32;
        for d in 1..=9usize {
            let v = var(cell / 9, cell % 9, d);
            if assign[v as usize] == 0 {
                if open == 0 {
                    first = v;
                }
                open += 1;
            }
        }
        if open == 0 {
            continue;
        }
        let better = match best {
            None => true,
            Some((fewest, _)) => open < fewest,
        };
        if better {
            best = Some((open, first));
        }
    }
    best.map(|(_, literal)| literal)
}

/// The 81 characters of the grid that `assign` decides, `?` for a cell with no true digit.
fn grid_text(assign: &[i8]) -> String {
    (0..81usize)
        .map(|i| {
            (1..=9usize)
                .find(|&d| assign[var(i / 9, i % 9, d) as usize] == 1)
                .map_or('?', |d| char::from(b'0' + d as u8))
        })
        .collect()
}

/// Makes `clause` RUP by case splits, writing the lemmas the splits need. If `clause` is already
/// RUP nothing is written. Otherwise one literal `b` is chosen, `clause + [-b]` and `clause + [b]`
/// are each made RUP one level deeper and written, and then `clause` itself is RUP.
fn split(st: &mut State, clause: &[i32], depth: usize) -> Result<(), ProveError> {
    if st.db.rup(clause) {
        return Ok(());
    }
    if depth == MAX_SPLIT_DEPTH {
        return Err(ProveError::new(
            SPLIT_DEPTH,
            format!(
                "a clause of {} literals is not closed within {} splits",
                clause.len(),
                MAX_SPLIT_DEPTH
            ),
        ));
    }
    let negation: Vec<i32> = clause.iter().map(|&l| -l).collect();
    let Some(assign) = st.db.assignment_under(&negation) else {
        return Err(internal(
            "the negation of a clause that is not RUP is contradictory",
        ));
    };
    let Some(b) = branch_literal(&assign) else {
        return Err(ProveError::new(
            NOT_UNIQUE,
            format!("another solution: {}", grid_text(&assign)),
        ));
    };
    let mut without = clause.to_vec();
    without.push(-b);
    split(st, &without, depth + 1)?;
    st.emit(without)?;
    let mut with = clause.to_vec();
    with.push(b);
    split(st, &with, depth + 1)?;
    st.emit(with)?;
    if !st.db.rup(clause) {
        return Err(ProveError::new(
            SPLIT_NOT_CLOSED,
            "a clause is not RUP after both halves of its split",
        ));
    }
    Ok(())
}

/// Split: the first open wrong literal `x` is refuted by case splits, then `[-x]` is written.
fn split_round(st: &mut State, wrong: &[i32]) -> Result<(), ProveError> {
    let Some(&x) = wrong.iter().find(|&&w| !refuted(&st.db, w)) else {
        return Err(ProveError::new(
            SELF_CHECK,
            "no open wrong literal is left, yet a cell is undecided",
        ));
    };
    split(st, &[-x], 0)?;
    st.emit(vec![-x])
}

/// Proves that `answer` is the only solution of `puzzle`.
///
/// `puzzle` has `0` for a blank and `1..=9` for a given; `answer` has `1..=9` everywhere. The
/// input is refused with `ANSWER_FORM`, `ANSWER_VS_GIVENS` or `ANSWER_RULES` unless the answer is
/// a full valid grid that keeps every given, and with `PUZZLE_CONTRADICTS` if unit propagation
/// over the puzzle's clauses already ends in a conflict.
///
/// The returned [`Proof`] has been verified by `sudoku_rup_check::check_board` against `answer`
/// before it is returned (`SELF_CHECK` otherwise). A puzzle that has another solution gives
/// `NOT_UNIQUE`; a puzzle the fixed rule cannot close gives `SPLIT_DEPTH` or `LEMMA_CAP`.
pub fn prove(puzzle: &[u8; 81], answer: &[u8; 81]) -> Result<Proof, ProveError> {
    prove_with(puzzle, answer, true)
}

/// The fixed rule. With `use_depth2` false the depth 2 step is left out, so that a test can drive
/// the split step on puzzles that depth 2 closes; [`prove`] always passes `true`.
fn prove_with(puzzle: &[u8; 81], answer: &[u8; 81], use_depth2: bool) -> Result<Proof, ProveError> {
    check_input(puzzle, answer)?;
    let db = Db::new(sudoku_cnf(puzzle)).map_err(internal)?;
    if db.base_conflict() {
        return Err(ProveError::new(
            PUZZLE_CONTRADICTS,
            "unit propagation over the puzzle's clauses already ends in a conflict",
        ));
    }
    let wrong = wrong_literals(answer);
    let right = answer_literals(answer);
    let mut st = State {
        db,
        lemmas: Vec::new(),
    };
    let mut depth2_rounds = 0usize;
    let mut split_rounds = 0usize;
    while !right.iter().all(|&a| st.db.value(a) == 1) {
        if depth1_pass(&mut st, &wrong)? > 0 {
            continue;
        }
        if use_depth2 && depth2_round(&mut st, &wrong)? {
            depth2_rounds += 1;
            continue;
        }
        split_round(&mut st, &wrong)?;
        split_rounds += 1;
    }
    match check_board(puzzle, &st.lemmas, Some(answer)) {
        Verdict::Verified { grid, lemmas } if grid == *answer && lemmas == st.lemmas.len() => {}
        other => {
            return Err(ProveError::new(
                SELF_CHECK,
                format!("the checker answered {other:?}"),
            ));
        }
    }
    let lemmas = st.lemmas;
    let units = lemmas.iter().filter(|l| l.len() == 1).count();
    let binaries = lemmas.iter().filter(|l| l.len() == 2).count();
    let longer = lemmas.iter().filter(|l| l.len() > 2).count();
    Ok(Proof {
        lemmas,
        units,
        binaries,
        longer,
        depth2_rounds,
        split_rounds,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use sudoku_rup_check::{format_lemma, parse_grid, parse_puzzle};

    /// A published hard puzzle with its answer, and what the rule without its depth 2 step writes
    /// for it: lemma counts, split rounds and the FNV-1a hash of the DRUP text.
    struct Case {
        name: &'static str,
        puzzle: &'static str,
        answer: &'static str,
        lemmas: usize,
        units: usize,
        binaries: usize,
        longer: usize,
        split_rounds: usize,
        fnv: u64,
    }

    const CASES: [Case; 3] = [
        // "AI Escargot", Arto Inkala, 2006.
        Case {
            name: "ai_escargot",
            puzzle: "1....7.9..3..2...8..96..5....53..9...1..8...26....4...3......1..4......7..7...3..",
            answer: "162857493534129678789643521475312986913586742628794135356478219241935867897261354",
            lemmas: 109,
            units: 37,
            binaries: 4,
            longer: 68,
            split_rounds: 2,
            fnv: 0xa51f_179e_4078_99dc,
        },
        // Arto Inkala, 2012.
        Case {
            name: "inkala_2012",
            puzzle: "8..........36......7..9.2...5...7.......457.....1...3...1....68..85...1..9....4..",
            answer: "812753649943682175675491283154237896369845721287169534521974368438526917796318452",
            lemmas: 345,
            units: 47,
            binaries: 6,
            longer: 292,
            split_rounds: 3,
            fnv: 0x2bb0_f55d_f49b_fb8b,
        },
        // "Easter Monster", Sudoku Players' Forum.
        Case {
            name: "easter_monster",
            puzzle: "1.......2.9.4...5...6...7...5.9.3.......7.......85..4.7.....6...3...9.8...2.....1",
            answer: "174385962293467158586192734451923876928674315367851249719548623635219487842736591",
            lemmas: 451,
            units: 33,
            binaries: 8,
            longer: 410,
            split_rounds: 4,
            fnv: 0x1854_f476_42e6_9e43,
        },
    ];

    /// FNV-1a over the bytes of `text`.
    fn fnv1a(text: &str) -> u64 {
        let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
        for &byte in text.as_bytes() {
            hash ^= u64::from(byte);
            hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
        }
        hash
    }

    /// The published hard puzzles are all closed by depth 1 and depth 2, so `prove` does not reach
    /// its split step on them. Here the rule runs without its depth 2 step (`prove_with` with
    /// `false`): depth 1, and the split step whenever depth 1 stalls. Real proofs come out of the
    /// split step, and the checker verifies them. The numbers (lemma counts, split rounds, FNV-1a
    /// of the DRUP text) come from an independent reference implementation of this prover's rule
    /// and change only when the rule changes.
    #[test]
    fn depth_one_and_split_prove_the_hard_puzzles() {
        let mut summary = String::new();
        for case in &CASES {
            let puzzle = parse_puzzle(case.puzzle).expect("valid puzzle text");
            let answer = parse_grid(case.answer).expect("valid grid text");
            let proof = prove_with(&puzzle, &answer, false)
                .unwrap_or_else(|error| panic!("{}: {error}", case.name));
            // The checker accepts every lemma, and the answer is the only solution.
            match check_board(&puzzle, &proof.lemmas, Some(&answer)) {
                Verdict::Verified { grid, lemmas } => {
                    assert_eq!(grid, answer, "{}: the decided grid", case.name);
                    assert_eq!(lemmas, proof.lemmas.len(), "{}: the lemma count", case.name);
                }
                other => panic!("{}: expected Verified, got {:?}", case.name, other),
            }
            assert_eq!(
                proof.depth2_rounds, 0,
                "{}: depth 2 was left out",
                case.name
            );
            assert!(
                proof.split_rounds > 0,
                "{}: the split step was not used",
                case.name
            );
            assert_eq!(
                (
                    proof.lemmas.len(),
                    proof.units,
                    proof.binaries,
                    proof.longer,
                    proof.split_rounds
                ),
                (
                    case.lemmas,
                    case.units,
                    case.binaries,
                    case.longer,
                    case.split_rounds
                ),
                "{}: counts of the fixed rule changed",
                case.name
            );
            let text: String = proof
                .lemmas
                .iter()
                .map(|lemma| format!("{}\n", format_lemma(lemma)))
                .collect();
            assert_eq!(
                fnv1a(&text),
                case.fnv,
                "{}: the lemma list of the fixed rule changed",
                case.name
            );
            summary.push_str(&format!(
                " {}:lemmas={},split_rounds={}",
                case.name,
                proof.lemmas.len(),
                proof.split_rounds
            ));
        }
        println!("CELL depth_one_and_split_prove_the_hard_puzzles GREEN{summary}");
    }
}
