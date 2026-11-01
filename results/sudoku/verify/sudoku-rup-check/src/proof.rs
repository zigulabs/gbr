//! Text format of proofs: DIMACS-style clauses, one lemma per clause.

use crate::{CheckError, VARS};

/// Parses proof text into lemma clauses.
///
/// The text is a stream of integers separated by ASCII whitespace. A clause
/// is a run of non-zero literals closed by `0`, and may span several lines.
/// Literals are `+-1..=+-729` (see [`crate::var`]).
///
/// * A line whose first non-space character is `c` is a comment.
/// * A line whose first non-space character is `d` is a deletion line; this
///   format has additions only, so it is an error.
/// * A bare `0` is the empty clause. It is accepted only as the very last
///   clause, where DRAT proofs end with it.
/// * Integers after the last `0`, or any token that is not an integer, are
///   errors. Errors name the line number.
pub fn parse_proof(text: &str) -> Result<Vec<Vec<i32>>, CheckError> {
    let mut lemmas: Vec<Vec<i32>> = Vec::new();
    let mut current: Vec<i32> = Vec::new();
    let mut current_line = 0usize;
    let mut empty_clause_line: Option<usize> = None;

    for (index, raw) in text.lines().enumerate() {
        let line_no = index + 1;
        let line = raw.trim_start_matches(|c: char| c.is_ascii_whitespace());
        if line.starts_with('c') {
            continue;
        }
        if line.starts_with('d') {
            return Err(CheckError::new(format!(
                "line {line_no}: deletion lines are not supported (additions only)"
            )));
        }
        for token in line.split_ascii_whitespace() {
            if let Some(first) = empty_clause_line {
                return Err(CheckError::new(format!(
                    "line {line_no}: content after the empty clause on line {first}"
                )));
            }
            let value = match parse_int(token) {
                Some(v) => v,
                None => {
                    return Err(CheckError::new(format!(
                        "line {line_no}: invalid token {token:?}"
                    )));
                }
            };
            if value == 0 {
                if current.is_empty() {
                    empty_clause_line = Some(line_no);
                    lemmas.push(Vec::new());
                } else {
                    lemmas.push(std::mem::take(&mut current));
                }
            } else {
                if (value.unsigned_abs() as usize) > VARS {
                    return Err(CheckError::new(format!(
                        "line {}: literal {} is outside -{}..={}",
                        line_no, value, VARS, VARS
                    )));
                }
                if current.is_empty() {
                    current_line = line_no;
                }
                current.push(value);
            }
        }
    }
    if !current.is_empty() {
        return Err(CheckError::new(format!(
            "line {current_line}: clause is not terminated by 0"
        )));
    }
    Ok(lemmas)
}

/// Parses `-?[0-9]+` as an `i32`. A zero must be written exactly `0`.
fn parse_int(token: &str) -> Option<i32> {
    let digits = token.strip_prefix('-').unwrap_or(token);
    if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let value: i32 = token.parse().ok()?;
    if value == 0 && token != "0" {
        return None;
    }
    Some(value)
}

/// Formats a clause as space-separated literals closed by `0`, for example
/// `"-123 456 0"`. The empty clause is `"0"`.
pub fn format_lemma(l: &[i32]) -> String {
    let mut s = String::new();
    for lit in l {
        s.push_str(&lit.to_string());
        s.push(' ');
    }
    s.push('0');
    s
}
