//! DIMACS and DRAT text for cross-checking with an external proof checker.

use crate::cnf::{VARS, sudoku_cnf, var};
use crate::proof::format_lemma;

/// DIMACS CNF of `puzzle` plus one blocking clause that forbids `grid`.
///
/// The text starts with `p cnf 729 <count>` and lists the clauses of
/// [`sudoku_cnf`] in order, then the blocking clause: the negation of the 81
/// literals `var(r, c, grid[r][c])`. The formula is unsatisfiable exactly when
/// `grid` is the only solution of the puzzle.
///
/// # Panics
///
/// Panics if a digit of `grid` is outside `1..=9`: a blocking clause built
/// from such a grid would silently express something else.
pub fn dimacs_with_block(puzzle: &[u8; 81], grid: &[u8; 81]) -> String {
    assert!(
        grid.iter().all(|d| (1..=9).contains(d)),
        "grid digits must be in 1..=9"
    );
    let cnf = sudoku_cnf(puzzle);
    let mut out = String::with_capacity(cnf.len() * 16 + 1024);
    out.push_str(&format!("p cnf {} {}\n", VARS, cnf.len() + 1));
    for clause in &cnf {
        out.push_str(&format_lemma(clause));
        out.push('\n');
    }
    let block: Vec<i32> = (0..81usize)
        .map(|i| -var(i / 9, i % 9, usize::from(grid[i])))
        .collect();
    out.push_str(&format_lemma(&block));
    out.push('\n');
    out
}

/// DRAT text for a proof: each lemma as a line ending in `0`, then a final
/// line `0` (the empty clause). Empty lemmas in `lemmas` are skipped, since
/// the final line already is the empty clause.
///
/// Together with the formula from [`dimacs_with_block`], unit propagation over
/// the formula and the lemmas forces the grid and falsifies the blocking
/// clause, so the empty clause is RUP and a DRAT checker accepts the proof.
pub fn drat_text(lemmas: &[Vec<i32>]) -> String {
    let mut out = String::new();
    for lemma in lemmas {
        if lemma.is_empty() {
            continue;
        }
        out.push_str(&format_lemma(lemma));
        out.push('\n');
    }
    out.push_str("0\n");
    out
}
