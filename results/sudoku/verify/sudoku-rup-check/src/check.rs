//! Checking a proof for one board and the direct Sudoku rule check.

use crate::cnf::{sudoku_cnf, var};
use crate::prop::{Db, literal_ok};

/// Outcome of checking one board against a proof.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Verdict {
    /// Every lemma was accepted, unit propagation decided all 81 cells, and the
    /// decided grid satisfies the Sudoku rules and the givens (and equals the
    /// expected grid, if one was given).
    Verified {
        /// The decided grid, row-major, digits `1..=9`.
        grid: [u8; 81],
        /// Number of lemmas in the proof.
        lemmas: usize,
    },
    /// Every lemma was accepted but unit propagation does not decide all cells.
    Open {
        /// Cells with exactly one true digit and all nine of its variables decided.
        decided_cells: usize,
        /// Number of lemmas in the proof.
        lemmas: usize,
    },
    /// The proof or the board was rejected.
    Failed {
        /// Short upper-case reason code.
        reason: &'static str,
        /// Zero-based index of the offending lemma, if the reason concerns one.
        lemma: Option<usize>,
    },
}

fn failed(reason: &'static str, lemma: Option<usize>) -> Verdict {
    Verdict::Failed { reason, lemma }
}

/// Checks `lemmas` as a RUP proof for `puzzle` and extracts the solution.
///
/// The CNF of `puzzle` is built and propagated first: a digit above 9 in
/// `puzzle` gives `BAD_PUZZLE`, a conflict gives `GIVENS_CONTRADICT`.
///
/// Each lemma, in order, must be non-empty (`EMPTY_CLAUSE_IN_PROOF`), use
/// literals in `+-1..=+-729` (`LITERAL_OUT_OF_RANGE`) and be RUP with respect
/// to the CNF and the earlier lemmas (`LEMMA_NOT_RUP`). It is then added; if
/// that makes the database conflict, the puzzle has no solution
/// (`PROOF_DERIVES_CONFLICT`).
///
/// If unit propagation then gives every cell exactly one true digit, that grid
/// is checked directly against the rules and the givens (`GRID_RULES_BROKEN`)
/// and, if `expected` is given, compared with it (`GRID_MISMATCH`). Otherwise
/// the verdict is [`Verdict::Open`].
pub fn check_board(
    puzzle: &[u8; 81],
    lemmas: &[Vec<i32>],
    expected: Option<&[u8; 81]>,
) -> Verdict {
    if puzzle.iter().any(|&d| d > 9) {
        return failed("BAD_PUZZLE", None);
    }
    let mut db = match Db::new(sudoku_cnf(puzzle)) {
        Ok(db) => db,
        // The generated CNF only has literals in range, so this cannot happen.
        Err(_) => return failed("INTERNAL_ERROR", None),
    };
    if db.base_conflict() {
        return failed("GIVENS_CONTRADICT", None);
    }

    for (i, lemma) in lemmas.iter().enumerate() {
        if lemma.is_empty() {
            return failed("EMPTY_CLAUSE_IN_PROOF", Some(i));
        }
        if lemma.iter().any(|&l| !literal_ok(l)) {
            return failed("LITERAL_OUT_OF_RANGE", Some(i));
        }
        if !db.rup(lemma) {
            return failed("LEMMA_NOT_RUP", Some(i));
        }
        if db.add(lemma.clone()).is_err() {
            return failed("LITERAL_OUT_OF_RANGE", Some(i));
        }
        if db.base_conflict() {
            return failed("PROOF_DERIVES_CONFLICT", Some(i));
        }
    }

    let mut grid = [0u8; 81];
    let mut decided = 0usize;
    for r in 0..9usize {
        for c in 0..9usize {
            let mut true_count = 0usize;
            let mut open_count = 0usize;
            let mut digit = 0u8;
            for d in 1..=9usize {
                match db.value(var(r, c, d)) {
                    1 => {
                        true_count += 1;
                        digit = d as u8;
                    }
                    0 => open_count += 1,
                    _ => {}
                }
            }
            if true_count == 1 && open_count == 0 {
                decided += 1;
                grid[9 * r + c] = digit;
            }
        }
    }
    if decided < 81 {
        return Verdict::Open {
            decided_cells: decided,
            lemmas: lemmas.len(),
        };
    }

    if !rules_hold(puzzle, &grid) {
        return failed("GRID_RULES_BROKEN", None);
    }
    if let Some(want) = expected {
        if *want != grid {
            return failed("GRID_MISMATCH", None);
        }
    }
    Verdict::Verified {
        grid,
        lemmas: lemmas.len(),
    }
}

/// Checks `grid` directly against the Sudoku rules: every digit is in `1..=9`,
/// each row, column and 3x3 box holds each of the nine digits exactly once,
/// and every nonzero entry of `puzzle` is kept.
pub fn rules_hold(puzzle: &[u8; 81], grid: &[u8; 81]) -> bool {
    if grid.iter().any(|&d| !(1..=9).contains(&d)) {
        return false;
    }
    for i in 0..81usize {
        if puzzle[i] != 0 && puzzle[i] != grid[i] {
            return false;
        }
    }
    // Bits 1..=9 set.
    const FULL: u16 = 0b11_1111_1110;
    for u in 0..9usize {
        let mut row = 0u16;
        let mut col = 0u16;
        let mut boxed = 0u16;
        for k in 0..9usize {
            row |= 1u16 << grid[9 * u + k];
            col |= 1u16 << grid[9 * k + u];
            let r = 3 * (u / 3) + k / 3;
            let c = 3 * (u % 3) + k % 3;
            boxed |= 1u16 << grid[9 * r + c];
        }
        if row != FULL || col != FULL || boxed != FULL {
            return false;
        }
    }
    true
}
