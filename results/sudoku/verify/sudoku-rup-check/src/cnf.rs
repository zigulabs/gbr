//! Variable numbering, board text parsing and the CNF of the extended encoding.

use crate::CheckError;

/// Number of propositional variables: one per (row, column, digit).
pub const VARS: usize = 729;

/// Returns the variable that means "cell `(r, c)` holds digit `d`".
///
/// `r` and `c` are in `0..9` and `d` is in `1..=9`; the result is in `1..=729`.
/// Arguments outside these ranges are a caller error, caught by a debug
/// assertion.
pub fn var(r: usize, c: usize, d: usize) -> i32 {
    debug_assert!(r < 9 && c < 9 && (1..=9).contains(&d));
    (81 * r + 9 * c + d) as i32
}

/// Parses a puzzle: 81 characters in row-major order, `'1'..='9'` for a given
/// digit and `'.'` or `'0'` for a blank (stored as 0).
///
/// Surrounding whitespace is ignored. Any other character, or a length other
/// than 81, is an error.
pub fn parse_puzzle(text: &str) -> Result<[u8; 81], CheckError> {
    parse_cells(text, "puzzle", true)
}

/// Parses a complete grid: 81 characters `'1'..='9'` in row-major order.
///
/// Surrounding whitespace is ignored. Blanks are not allowed.
pub fn parse_grid(text: &str) -> Result<[u8; 81], CheckError> {
    parse_cells(text, "grid", false)
}

/// Shared parser for puzzles and grids.
fn parse_cells(text: &str, what: &str, blank_ok: bool) -> Result<[u8; 81], CheckError> {
    let mut cells = [0u8; 81];
    let mut n = 0usize;
    for ch in text.trim().chars() {
        if n == 81 {
            return Err(CheckError::new(format!("{what} has more than 81 characters")));
        }
        cells[n] = match ch {
            '1'..='9' => ch as u8 - b'0',
            '.' | '0' if blank_ok => 0,
            _ => {
                return Err(CheckError::new(format!(
                    "{what}: invalid character {ch:?} at position {n}"
                )));
            }
        };
        n += 1;
    }
    if n != 81 {
        return Err(CheckError::new(format!(
            "{what} has {n} characters, expected 81"
        )));
    }
    Ok(cells)
}

/// Renders a grid or puzzle as 81 characters: digits `1..=9`, `'.'` for 0 and
/// `'?'` for any other value.
pub fn grid_to_string(grid: &[u8; 81]) -> String {
    grid.iter()
        .map(|&d| match d {
            1..=9 => char::from(b'0' + d),
            0 => '.',
            _ => '?',
        })
        .collect()
}

/// Builds the CNF of the extended encoding for `puzzle` (0 = blank).
///
/// Clauses come in this fixed order: cell at-least-one (81 clauses); cell
/// at-most-one (81 * 36 binary clauses); then rows, columns and boxes, where
/// each unit and digit contributes one at-least-one clause over its nine cells
/// followed by the 36 pairwise at-most-one clauses (cells of a box in
/// row-major order); finally one unit clause per given, in cell order.
///
/// That is 11988 clauses plus the number of givens. Entries of `puzzle`
/// outside `1..=9` are treated as blank.
pub fn sudoku_cnf(puzzle: &[u8; 81]) -> Vec<Vec<i32>> {
    let mut cnf: Vec<Vec<i32>> = Vec::with_capacity(11988 + 81);

    // 1. cell at-least-one
    for r in 0..9usize {
        for c in 0..9usize {
            cnf.push((1..=9usize).map(|d| var(r, c, d)).collect());
        }
    }

    // 2. cell at-most-one
    for r in 0..9usize {
        for c in 0..9usize {
            for d1 in 1..=9usize {
                for d2 in (d1 + 1)..=9usize {
                    cnf.push(vec![-var(r, c, d1), -var(r, c, d2)]);
                }
            }
        }
    }

    // 3. rows
    for r in 0..9usize {
        let cells: [(usize, usize); 9] = std::array::from_fn(|c| (r, c));
        for d in 1..=9usize {
            exactly_once(&mut cnf, &cells, d);
        }
    }

    // 4. columns
    for c in 0..9usize {
        let cells: [(usize, usize); 9] = std::array::from_fn(|r| (r, c));
        for d in 1..=9usize {
            exactly_once(&mut cnf, &cells, d);
        }
    }

    // 5. boxes
    for b in 0..9usize {
        let cells: [(usize, usize); 9] =
            std::array::from_fn(|k| (3 * (b / 3) + k / 3, 3 * (b % 3) + k % 3));
        for d in 1..=9usize {
            exactly_once(&mut cnf, &cells, d);
        }
    }

    // 6. givens
    for r in 0..9usize {
        for c in 0..9usize {
            let g = puzzle[9 * r + c];
            if (1..=9).contains(&g) {
                cnf.push(vec![var(r, c, usize::from(g))]);
            }
        }
    }

    cnf
}

/// Appends the at-least-one clause for digit `d` over `cells`, then the
/// pairwise at-most-one clauses in index order.
fn exactly_once(cnf: &mut Vec<Vec<i32>>, cells: &[(usize, usize); 9], d: usize) {
    cnf.push(cells.iter().map(|&(r, c)| var(r, c, d)).collect());
    for i in 0..9usize {
        for j in (i + 1)..9usize {
            let (ri, ci) = cells[i];
            let (rj, cj) = cells[j];
            cnf.push(vec![-var(ri, ci, d), -var(rj, cj, d)]);
        }
    }
}
