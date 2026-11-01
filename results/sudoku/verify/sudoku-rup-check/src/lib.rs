//! Checker for Sudoku solutions that come with a reverse-unit-propagation (RUP) proof.
//!
//! A 9x9 puzzle is encoded as CNF with the extended encoding: 729 variables,
//! `var(r, c, d) = 81 * r + 9 * c + d`, true when cell `(r, c)` holds digit `d`.
//! A proof is a list of lemma clauses. Each lemma must follow from the CNF and
//! the lemmas before it by reverse unit propagation: assuming every literal of
//! the lemma false and propagating units reaches a conflict.
//!
//! After the last lemma, unit propagation over the whole database decides the
//! board when every cell ends with exactly one true digit. That grid is then
//! the unique solution, because every lemma is a logical consequence of the
//! rules and the givens. The grid is also checked directly against the Sudoku
//! rules, without using the CNF.
//!
//! The CNF and the proof can be exported as DIMACS and DRAT text, so that an
//! independent checker such as `drat-trim` can confirm the same claim.
//!
//! The crate uses only the standard library and contains no `unsafe` code.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

mod check;
mod cnf;
mod dimacs;
mod proof;
mod prop;

use std::fmt;

pub use check::{Verdict, check_board, rules_hold};
pub use cnf::{VARS, grid_to_string, parse_grid, parse_puzzle, sudoku_cnf, var};
pub use dimacs::{dimacs_with_block, drat_text};
pub use proof::{format_lemma, parse_proof};
pub use prop::Db;

/// Error for malformed input text or out-of-range clause data.
///
/// The message is one line and has no trailing newline.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheckError {
    message: String,
}

impl CheckError {
    /// Creates an error with the given one-line message.
    pub fn new(message: impl Into<String>) -> CheckError {
        CheckError {
            message: message.into(),
        }
    }

    /// The message text.
    pub fn message(&self) -> &str {
        &self.message
    }
}

impl fmt::Display for CheckError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for CheckError {}
