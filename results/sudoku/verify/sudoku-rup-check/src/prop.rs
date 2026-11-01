//! Clause database with unit propagation at decision level 0.
//!
//! The database keeps the assignment `base` that results from propagating all
//! of its clauses without assumptions. A RUP test copies `base`, assumes the
//! lemma false and propagates again.

use crate::{CheckError, VARS};

/// `true` if `l` is a literal over the variables `1..=VARS`.
pub(crate) fn literal_ok(l: i32) -> bool {
    l != 0 && (l.unsigned_abs() as usize) <= VARS
}

/// Slot of literal `l` in the occurrence table, in `0..2 * VARS`.
/// `l` must satisfy [`literal_ok`].
fn lit_index(l: i32) -> usize {
    2 * (l.unsigned_abs() as usize - 1) + usize::from(l < 0)
}

/// Value of `l` under `assign`: 1 true, -1 false, 0 unassigned.
fn lit_value(assign: &[i8], l: i32) -> i8 {
    let a = assign[l.unsigned_abs() as usize];
    if l > 0 { a } else { -a }
}

/// Makes `l` true in `assign`.
fn set_true(assign: &mut [i8], l: i32) {
    assign[l.unsigned_abs() as usize] = if l > 0 { 1 } else { -1 };
}

/// Unit propagation. `queue` holds literals that were just made true in
/// `assign`; every clause containing the negation of such a literal lost a
/// literal and is inspected. Returns `false` on conflict (a clause with all
/// literals false). The queue is empty on return.
fn propagate_into(
    clauses: &[Vec<i32>],
    occurs: &[Vec<usize>],
    assign: &mut [i8],
    queue: &mut Vec<i32>,
) -> bool {
    while let Some(l) = queue.pop() {
        for &ci in &occurs[lit_index(-l)] {
            let mut satisfied = false;
            let mut unassigned = 0usize;
            let mut last = 0i32;
            for &x in &clauses[ci] {
                match lit_value(assign, x) {
                    1 => {
                        satisfied = true;
                        break;
                    }
                    0 => {
                        unassigned += 1;
                        last = x;
                    }
                    _ => {}
                }
            }
            if satisfied {
                continue;
            }
            if unassigned == 0 {
                queue.clear();
                return false;
            }
            if unassigned == 1 {
                set_true(assign, last);
                queue.push(last);
            }
        }
    }
    true
}

/// A clause database over variables `1..=729` with unit propagation.
///
/// Clauses are stored with their literals sorted and de-duplicated. Literals
/// are non-zero integers with absolute value at most [`VARS`]; a negative
/// literal is the negation of the variable.
#[derive(Debug, Clone)]
pub struct Db {
    /// All clauses, including accepted lemmas.
    clauses: Vec<Vec<i32>>,
    /// `occurs[lit_index(l)]` lists the clauses that contain literal `l`.
    occurs: Vec<Vec<usize>>,
    /// Assignment after propagating every clause at level 0, indexed by
    /// variable (index 0 unused): 1 true, -1 false, 0 unassigned.
    base: Vec<i8>,
    /// `true` once propagation at level 0 has produced a conflict.
    base_conflict: bool,
}

impl Db {
    /// Builds a database from `clauses` and propagates it at level 0.
    ///
    /// Returns an error naming the first clause that has a literal outside
    /// `+-1..=+-729` (including 0). An empty clause, or units that conflict,
    /// are accepted and set [`Db::base_conflict`].
    pub fn new(clauses: Vec<Vec<i32>>) -> Result<Db, CheckError> {
        for (i, clause) in clauses.iter().enumerate() {
            if let Some(&bad) = clause.iter().find(|&&l| !literal_ok(l)) {
                return Err(CheckError::new(format!(
                    "clause {}: invalid literal {} (absolute value must be in 1..={})",
                    i, bad, VARS
                )));
            }
        }
        let mut db = Db {
            clauses: Vec::with_capacity(clauses.len()),
            occurs: vec![Vec::new(); 2 * VARS],
            base: vec![0; VARS + 1],
            base_conflict: false,
        };
        for clause in clauses {
            db.store(clause);
        }
        db.init_base();
        Ok(db)
    }

    /// Stores a clause and registers it in the occurrence lists. Returns its index.
    fn store(&mut self, mut clause: Vec<i32>) -> usize {
        clause.sort_unstable();
        clause.dedup();
        let idx = self.clauses.len();
        for &l in &clause {
            self.occurs[lit_index(l)].push(idx);
        }
        self.clauses.push(clause);
        idx
    }

    /// Computes `base` from scratch: assigns unit clauses, flags an empty
    /// clause or a unit that is already false, then propagates.
    fn init_base(&mut self) {
        let mut queue: Vec<i32> = Vec::new();
        for clause in &self.clauses {
            match clause.as_slice() {
                [] => {
                    self.base_conflict = true;
                    return;
                }
                [l] => match lit_value(&self.base, *l) {
                    1 => {}
                    -1 => {
                        self.base_conflict = true;
                        return;
                    }
                    _ => {
                        set_true(&mut self.base, *l);
                        queue.push(*l);
                    }
                },
                _ => {}
            }
        }
        if !propagate_into(&self.clauses, &self.occurs, &mut self.base, &mut queue) {
            self.base_conflict = true;
        }
    }

    /// `true` if propagating the database at level 0 produced a conflict, that
    /// is, the clauses are unsatisfiable by unit propagation alone.
    pub fn base_conflict(&self) -> bool {
        self.base_conflict
    }

    /// Number of stored clauses.
    pub fn clause_count(&self) -> usize {
        self.clauses.len()
    }

    /// Value of `lit` in the level-0 assignment: 1 true, -1 false, 0 unassigned.
    /// A literal outside `+-1..=+-729` has value 0.
    pub fn value(&self, lit: i32) -> i8 {
        if !literal_ok(lit) {
            return 0;
        }
        lit_value(&self.base, lit)
    }

    /// Reverse-unit-propagation test: assigns every literal of `lemma` false,
    /// propagates, and reports whether that reaches a conflict.
    ///
    /// A lemma with a literal that is already true is RUP. An empty lemma is
    /// RUP exactly when [`Db::base_conflict`] holds, and so is every lemma of
    /// a conflicting database. A lemma with a literal outside `+-1..=+-729`
    /// is not RUP (unless the database is already in conflict).
    pub fn rup(&self, lemma: &[i32]) -> bool {
        if self.base_conflict {
            return true;
        }
        if lemma.iter().any(|&l| !literal_ok(l)) {
            return false;
        }
        let mut assign = self.base.clone();
        let mut queue: Vec<i32> = Vec::new();
        for &l in lemma {
            match lit_value(&assign, l) {
                1 => return true,
                -1 => {}
                _ => {
                    set_true(&mut assign, -l);
                    queue.push(-l);
                }
            }
        }
        !propagate_into(&self.clauses, &self.occurs, &mut assign, &mut queue)
    }

    /// Adds a clause (for example an accepted lemma) and updates the level-0
    /// assignment: a clause that is unit under it is propagated, a clause that
    /// is false under it sets [`Db::base_conflict`].
    ///
    /// Returns an error, and leaves the database unchanged, if a literal is
    /// outside `+-1..=+-729`.
    pub fn add(&mut self, clause: Vec<i32>) -> Result<(), CheckError> {
        if let Some(&bad) = clause.iter().find(|&&l| !literal_ok(l)) {
            return Err(CheckError::new(format!(
                "invalid literal {} (absolute value must be in 1..={})",
                bad, VARS
            )));
        }
        let idx = self.store(clause);
        if self.base_conflict {
            return Ok(());
        }
        let mut satisfied = false;
        let mut unassigned = 0usize;
        let mut last = 0i32;
        for &l in &self.clauses[idx] {
            match lit_value(&self.base, l) {
                1 => {
                    satisfied = true;
                    break;
                }
                0 => {
                    unassigned += 1;
                    last = l;
                }
                _ => {}
            }
        }
        if satisfied {
            return Ok(());
        }
        if unassigned == 0 {
            self.base_conflict = true;
        } else if unassigned == 1 {
            set_true(&mut self.base, last);
            let mut queue = vec![last];
            if !propagate_into(&self.clauses, &self.occurs, &mut self.base, &mut queue) {
                self.base_conflict = true;
            }
        }
        Ok(())
    }

    /// Assignment obtained by assuming the literals in `assumptions` on top of
    /// the level-0 assignment and propagating.
    ///
    /// The result is indexed by variable (index 0 is unused) with values
    /// 1 true, -1 false, 0 unassigned. Returns `None` if an assumption is
    /// already false, if propagation reaches a conflict, if an assumption is
    /// outside `+-1..=+-729`, or if the database is already in conflict.
    pub fn assignment_under(&self, assumptions: &[i32]) -> Option<Vec<i8>> {
        if self.base_conflict {
            return None;
        }
        if assumptions.iter().any(|&l| !literal_ok(l)) {
            return None;
        }
        let mut assign = self.base.clone();
        let mut queue: Vec<i32> = Vec::new();
        for &l in assumptions {
            match lit_value(&assign, l) {
                -1 => return None,
                1 => {}
                _ => {
                    set_true(&mut assign, l);
                    queue.push(l);
                }
            }
        }
        if propagate_into(&self.clauses, &self.occurs, &mut assign, &mut queue) {
            Some(assign)
        } else {
            None
        }
    }
}
