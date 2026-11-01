//! Integration tests. Each test is one "cell" and prints a single
//! `CELL <name> GREEN ...` line when it passes (run with `--nocapture` to see them).
//!
//! The only puzzle data used are two widely published puzzles and their solutions.

use sudoku_rup_check::{
    Db, VARS, Verdict, check_board, dimacs_with_block, drat_text, format_lemma, grid_to_string,
    parse_grid, parse_proof, parse_puzzle, rules_hold, sudoku_cnf, var,
};

/// The example puzzle of the Wikipedia article on Sudoku.
const EASY: &str =
    "53..7....6..195....98....6.8...6...34..8.3..17...2...6.6....28....419..5....8..79";
const EASY_SOLUTION: &str =
    "534678912672195348198342567859761423426853791713924856961537284287419635345286179";

/// Arto Inkala's widely published puzzle.
const HARD: &str =
    "8..........36......7..9.2...5...7.......457.....1...3...1....68..85...1..9....4..";
const HARD_SOLUTION: &str =
    "812753649943682175675491283154237896369845721287169534521974368438526917796318452";

fn puz(text: &str) -> [u8; 81] {
    parse_puzzle(text).expect("valid puzzle text")
}

fn grd(text: &str) -> [u8; 81] {
    parse_grid(text).expect("valid grid text")
}

fn count_givens(text: &str) -> usize {
    text.chars().filter(|c| ('1'..='9').contains(c)).count()
}

/// Proof by failed-literal saturation: repeatedly add the unit lemma `[-v]`
/// for every undecided variable `v` whose assumption leads to a conflict.
/// Each lemma is RUP at the moment it is added.
fn saturate(puzzle: &[u8; 81]) -> Vec<Vec<i32>> {
    let mut db = Db::new(sudoku_cnf(puzzle)).expect("generated CNF is in range");
    let mut lemmas: Vec<Vec<i32>> = Vec::new();
    loop {
        let mut added = false;
        for v in 1..=(VARS as i32) {
            if db.value(v) == 0 && db.rup(&[-v]) {
                lemmas.push(vec![-v]);
                db.add(vec![-v]).expect("literal in range");
                added = true;
            }
        }
        if !added {
            break;
        }
    }
    lemmas
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

#[test]
fn cnf_shape() {
    let p = puz(EASY);
    let givens = count_givens(EASY);
    assert_eq!(givens, 30);
    let cnf = sudoku_cnf(&p);
    assert_eq!(cnf.len(), 11988 + givens);
    assert_eq!(var(0, 0, 1), 1);
    assert_eq!(var(8, 8, 9), 729);
    assert!(
        cnf.iter()
            .all(|cl| cl.iter().all(|&l| l != 0 && (l.unsigned_abs() as usize) <= VARS))
    );
    // Fixed order of the families: first clause of each, by index.
    assert_eq!(cnf[0], (1..=9).collect::<Vec<i32>>()); // cell at-least-one
    assert_eq!(cnf[81], vec![-1, -2]); // cell at-most-one
    assert_eq!(cnf[81 + 2916], vec![1, 10, 19, 28, 37, 46, 55, 64, 73]); // row 0, digit 1
    assert_eq!(cnf[81 + 2916 + 1], vec![-1, -10]);
    assert_eq!(cnf[81 + 2916 + 2997], vec![1, 82, 163, 244, 325, 406, 487, 568, 649]); // column 0
    assert_eq!(
        cnf[81 + 2916 + 2 * 2997],
        vec![1, 10, 19, 82, 91, 100, 163, 172, 181]
    ); // box 0, row-major
    assert_eq!(cnf[11988], vec![5]); // first given: cell 0 holds 5
    assert_eq!(cnf[11989], vec![12]); // second given: cell 1 holds 3
    assert_eq!(cnf[cnf.len() - 1], vec![729]); // last given: cell 80 holds 9
    println!("CELL cnf_shape GREEN clauses={} givens={}", cnf.len(), givens);
}

#[test]
fn easy_verified() {
    let p = puz(EASY);
    let sol = grd(EASY_SOLUTION);
    let proof = saturate(&p);
    match check_board(&p, &proof, Some(&sol)) {
        Verdict::Verified { grid: g, lemmas } => {
            assert_eq!(g, sol);
            assert_eq!(lemmas, proof.len());
            println!("CELL easy_verified GREEN lemmas={}", lemmas);
        }
        other => panic!("expected Verified, got {:?}", other),
    }
}

#[test]
fn hard_open_without_proof() {
    let p = puz(HARD);
    let givens = count_givens(HARD);
    match check_board(&p, &[], None) {
        Verdict::Open {
            decided_cells,
            lemmas,
        } => {
            assert_eq!(lemmas, 0);
            assert!(decided_cells >= givens && decided_cells < 81);
            println!(
                "CELL hard_open_without_proof GREEN decided={} lemmas={}",
                decided_cells, lemmas
            );
        }
        other => panic!("expected Open, got {:?}", other),
    }
}

#[test]
fn hard_saturation_consistent() {
    let p = puz(HARD);
    let sol = grd(HARD_SOLUTION);
    let proof = saturate(&p);
    match check_board(&p, &proof, None) {
        Verdict::Failed { reason, lemma } => {
            panic!("saturation proof rejected: {} at {:?}", reason, lemma)
        }
        Verdict::Verified { grid: g, lemmas } => {
            assert_eq!(g, sol);
            println!(
                "CELL hard_saturation_consistent GREEN verified lemmas={}",
                lemmas
            );
        }
        Verdict::Open {
            decided_cells,
            lemmas,
        } => {
            println!(
                "CELL hard_saturation_consistent GREEN open decided={} lemmas={}",
                decided_cells, lemmas
            );
        }
    }
}

#[test]
fn mutant_wrong_lemma() {
    let p = puz(EASY);
    let mut proof = saturate(&p);
    let at = proof.len();
    // Cell (0, 0) is a given 5, so "cell (0, 0) is not 5" is false.
    proof.push(vec![-var(0, 0, 5)]);
    let verdict = check_board(&p, &proof, None);
    assert_eq!(
        verdict,
        Verdict::Failed {
            reason: "LEMMA_NOT_RUP",
            lemma: Some(at)
        }
    );
    println!("CELL mutant_wrong_lemma GREEN lemma={}", at);
}

#[test]
fn mutant_changed_given() {
    let proof = saturate(&puz(EASY));
    let sol = grd(EASY_SOLUTION);
    // Same proof and expected grid, but the first given 5 becomes 1.
    let mutated = format!("1{}", &EASY[1..]);
    assert_ne!(mutated, EASY);
    let verdict = check_board(&puz(&mutated), &proof, Some(&sol));
    assert!(
        !matches!(verdict, Verdict::Verified { .. }),
        "mutant accepted: {:?}",
        verdict
    );
    println!("CELL mutant_changed_given GREEN verdict={:?}", verdict);
}

#[test]
fn mutant_expected_mismatch() {
    let p = puz(EASY);
    let proof = saturate(&p);
    let mut wrong = grd(EASY_SOLUTION);
    wrong.swap(0, 1); // two digits swapped in row 0
    let verdict = check_board(&p, &proof, Some(&wrong));
    assert_eq!(
        verdict,
        Verdict::Failed {
            reason: "GRID_MISMATCH",
            lemma: None
        }
    );
    println!("CELL mutant_expected_mismatch GREEN");
}

#[test]
fn rules_hold_detects_duplicate() {
    let p = puz(EASY);
    let sol = grd(EASY_SOLUTION);
    assert!(rules_hold(&p, &sol));
    assert!(rules_hold(&puz(HARD), &grd(HARD_SOLUTION)));

    // Cells 0 and 1 swapped: row 0 is still a permutation, but columns 0 and 1
    // now repeat a digit, and the first two givens are not kept.
    let mut swapped = sol;
    swapped.swap(0, 1);
    assert!(!rules_hold(&p, &swapped));

    // Without givens only the units can object.
    let blank = [0u8; 81];
    assert!(rules_hold(&blank, &sol));
    assert!(!rules_hold(&blank, &swapped));

    // Rows and columns are permutations here, boxes are not.
    let mut latin = [0u8; 81];
    for r in 0..9 {
        for c in 0..9 {
            latin[9 * r + c] = ((r + c) % 9) as u8 + 1;
        }
    }
    assert!(!rules_hold(&blank, &latin));

    // Digits outside 1..=9 are rejected.
    let mut bad = sol;
    bad[40] = 0;
    assert!(!rules_hold(&blank, &bad));
    bad[40] = 10;
    assert!(!rules_hold(&blank, &bad));
    println!("CELL rules_hold_detects_duplicate GREEN");
}

#[test]
fn proof_parse() {
    let text = "c a comment\n\n  c indented comment\n1 -2 0\n\n3 4\n5 0\n";
    assert_eq!(
        parse_proof(text).expect("comments, blank lines, multi-line clause"),
        vec![vec![1, -2], vec![3, 4, 5]]
    );
    assert_eq!(parse_proof(""), Ok(vec![]));

    assert!(parse_proof("1 2").is_err()); // no closing 0
    assert!(parse_proof("730 0").is_err()); // variable out of range
    assert!(parse_proof("-730 0").is_err());
    assert!(parse_proof("d 5 0").is_err()); // deletion line
    assert!(parse_proof("0\n1 0\n").is_err()); // empty clause not last
    assert!(parse_proof("x 0").is_err()); // not an integer
    assert!(parse_proof("1.5 0").is_err());

    // The empty clause is allowed as the very last clause.
    assert_eq!(
        parse_proof("-5 0\n0\n"),
        Ok(vec![vec![-5], vec![]])
    );

    // Errors name the line.
    let err = parse_proof("1 0\n2 x 0\n").expect_err("bad token");
    assert!(err.to_string().contains("line 2"), "{}", err);
    println!("CELL proof_parse GREEN");
}

#[test]
fn dimacs_and_drat_text() {
    let p = puz(EASY);
    let sol = grd(EASY_SOLUTION);
    let cnf = sudoku_cnf(&p);
    let text = dimacs_with_block(&p, &sol);
    assert!(text.ends_with('\n'));
    let lines: Vec<&str> = text.lines().collect();

    let header = format!("p cnf 729 {}", cnf.len() + 1);
    assert_eq!(lines[0], header);
    assert_eq!(lines.len(), cnf.len() + 2); // header, clauses, blocking clause
    for (k, clause) in cnf.iter().enumerate() {
        assert_eq!(lines[1 + k], format_lemma(clause));
    }

    // The blocking clause negates the solution digit of every cell.
    let last: Vec<&str> = lines[lines.len() - 1].split_whitespace().collect();
    assert_eq!(last.len(), 82);
    assert_eq!(last[81], "0");
    for (i, tok) in last[..81].iter().enumerate() {
        assert!(tok.starts_with('-'));
        let want = -var(i / 9, i % 9, usize::from(sol[i]));
        assert_eq!(tok.parse::<i32>().expect("integer literal"), want);
    }

    // DRAT: one line per lemma, then the empty clause on its own line.
    assert_eq!(format_lemma(&[-123, 456]), "-123 456 0");
    assert_eq!(format_lemma(&[]), "0");
    let lemmas = vec![vec![1, -2], vec![3]];
    let drat = drat_text(&lemmas);
    assert_eq!(drat, "1 -2 0\n3 0\n0\n");
    assert!(drat.ends_with("\n0\n"));
    assert_eq!(drat.lines().last(), Some("0"));
    assert_eq!(drat_text(&[]), "0\n");
    assert_eq!(drat_text(&[vec![-5], vec![]]), "-5 0\n0\n");
    println!(
        "CELL dimacs_and_drat_text GREEN clauses={} header={:?}",
        cnf.len() + 1,
        header
    );
}

#[test]
fn rup_basics() {
    let p = puz(EASY);
    let db = Db::new(sudoku_cnf(&p)).expect("generated CNF is in range");
    assert!(!db.base_conflict());
    assert_eq!(db.clause_count(), 11988 + count_givens(EASY));

    // Cell (0, 0) is a given 5.
    assert_eq!(db.value(var(0, 0, 5)), 1);
    assert_eq!(db.value(-var(0, 0, 5)), -1);
    // A clause with a literal that is already true is RUP.
    assert!(db.rup(&[var(0, 0, 5), var(0, 1, 1)]));
    assert!(db.rup(&[var(0, 0, 5)]));
    // The empty clause is RUP only for a conflicting database.
    assert!(!db.rup(&[]));
    // A false unit is not RUP, and literals out of range never are.
    assert!(!db.rup(&[-var(0, 0, 5)]));
    assert!(!db.rup(&[0]));
    assert!(!db.rup(&[730]));
    assert_eq!(db.value(0), 0);
    assert_eq!(db.value(-730), 0);

    // Assumptions: the given is already true, its negation is already false.
    let ok = db.assignment_under(&[var(0, 0, 5)]).expect("consistent");
    assert_eq!(ok.len(), VARS + 1);
    assert_eq!(ok[var(0, 0, 5) as usize], 1);
    assert!(db.assignment_under(&[-var(0, 0, 5)]).is_none());
    println!("CELL rup_basics GREEN");
}

#[test]
fn board_text_parsing() {
    let p = parse_puzzle(&format!("  {}\n", EASY)).expect("surrounding whitespace is ignored");
    assert_eq!(p[0], 5);
    assert_eq!(p[2], 0);
    assert_eq!(parse_puzzle(&EASY.replace('.', "0")).expect("'0' is a blank"), p);
    assert!(parse_puzzle(&EASY[..80]).is_err());
    assert!(parse_puzzle(&format!("{}1", EASY)).is_err());
    assert!(parse_puzzle(&EASY.replace('5', "x")).is_err());
    assert!(parse_puzzle("").is_err());
    assert!(parse_grid(EASY).is_err()); // a grid has no blanks
    assert!(parse_grid(&EASY_SOLUTION.replace('5', "0")).is_err());
    let g = parse_grid(EASY_SOLUTION).expect("valid grid");
    assert_eq!(grid_to_string(&g), EASY_SOLUTION);
    assert_eq!(grid_to_string(&p), EASY);
    println!("CELL board_text_parsing GREEN");
}

#[test]
fn db_literal_checks_and_conflicts() {
    assert!(Db::new(vec![vec![0]]).is_err());
    assert!(Db::new(vec![vec![1, 730]]).is_err());
    assert!(Db::new(vec![vec![-730]]).is_err());
    let msg = Db::new(vec![vec![1], vec![2, 0]])
        .expect_err("literal 0")
        .to_string();
    assert!(msg.contains("clause 1"), "{}", msg);

    // Duplicate literals collapse; a tautology constrains nothing.
    let dup = Db::new(vec![vec![5, 5]]).expect("in range");
    assert_eq!(dup.value(5), 1);
    let taut = Db::new(vec![vec![7, -7]]).expect("in range");
    assert_eq!(taut.value(7), 0);
    assert!(!taut.base_conflict());

    // An empty clause, or contradicting units, give a conflicting database.
    assert!(Db::new(vec![vec![]]).expect("in range").base_conflict());
    assert!(
        Db::new(vec![vec![3], vec![-3]])
            .expect("in range")
            .base_conflict()
    );

    // Units propagate: (1) and (-1 or 2) force 2.
    let mut db = Db::new(vec![vec![1], vec![-1, 2]]).expect("in range");
    assert_eq!(db.value(2), 1);
    assert!(!db.base_conflict());

    // A clause that is false under the level-0 assignment sets the conflict flag.
    assert!(db.add(vec![0]).is_err());
    assert_eq!(db.clause_count(), 2);
    db.add(vec![-2]).expect("in range");
    assert!(db.base_conflict());
    assert!(db.rup(&[]));
    assert!(db.assignment_under(&[]).is_none());
    println!("CELL db_literal_checks_and_conflicts GREEN");
}

#[test]
fn rup_tautology_and_duplicates() {
    let db = Db::new(sudoku_cnf(&puz(HARD))).expect("generated CNF is in range");
    assert!(!db.base_conflict());
    let v = (1..=(VARS as i32))
        .find(|&v| db.value(v) == 0)
        .expect("HARD leaves variables undecided");
    // A tautology is always RUP; a repeated literal acts like one literal.
    assert!(db.rup(&[v, -v]));
    assert_eq!(db.rup(&[v, v]), db.rup(&[v]));
    println!("CELL rup_tautology_and_duplicates GREEN var={}", v);
}

#[test]
fn check_board_failure_reasons() {
    let p = puz(EASY);
    let sol = grd(EASY_SOLUTION);
    let fail = |reason: &'static str, lemma: Option<usize>| Verdict::Failed { reason, lemma };

    // A digit above 9 in the puzzle.
    let mut bad_puzzle = p;
    bad_puzzle[2] = 10;
    assert_eq!(check_board(&bad_puzzle, &[], None), fail("BAD_PUZZLE", None));

    // Two 5s in row 0.
    let clash = puz(&format!("55{}", &EASY[2..]));
    assert_eq!(
        check_board(&clash, &[], None),
        fail("GIVENS_CONTRADICT", None)
    );

    // Bad lemmas: empty, literal 0, literal out of range.
    assert_eq!(
        check_board(&p, &[vec![]], None),
        fail("EMPTY_CLAUSE_IN_PROOF", Some(0))
    );
    assert_eq!(
        check_board(&p, &[vec![1, 0]], None),
        fail("LITERAL_OUT_OF_RANGE", Some(0))
    );
    assert_eq!(
        check_board(&p, &[vec![730]], None),
        fail("LITERAL_OUT_OF_RANGE", Some(0))
    );

    // A redundant lemma (already true at level 0) is accepted.
    assert_eq!(
        check_board(&p, &[vec![var(0, 0, 5)]], Some(&sol)),
        Verdict::Verified {
            grid: sol,
            lemmas: 1
        }
    );
    println!("CELL check_board_failure_reasons GREEN");
}

#[test]
fn dimacs_formula_matches_propagation() {
    let p = puz(EASY);
    let sol = grd(EASY_SOLUTION);

    // Propagation forces the solution, so the blocking clause is falsified:
    // the formula is refuted before any lemma is needed.
    let formula = read_dimacs(&dimacs_with_block(&p, &sol));
    assert_eq!(formula.len(), sudoku_cnf(&p).len() + 1);
    assert!(Db::new(formula).expect("in range").base_conflict());

    // Blocking some other grid leaves the formula satisfiable: the solution
    // satisfies the blocking clause.
    let mut other = sol;
    other.swap(0, 1);
    let formula = read_dimacs(&dimacs_with_block(&p, &other));
    assert!(!Db::new(formula).expect("in range").base_conflict());
    println!("CELL dimacs_formula_matches_propagation GREEN");
}
