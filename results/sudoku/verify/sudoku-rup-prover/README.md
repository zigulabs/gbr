# sudoku-rup-prover

Given a 9x9 Sudoku puzzle and its answer, writes a proof that the answer is the only solution.
The proof is a list of lemma clauses in DRUP form (reverse unit propagation, additions only). It is
accepted by the sibling crate `sudoku-rup-check`, and through that crate's DIMACS and DRAT export
also by `drat-trim`.

The prover reads exactly two strings: the puzzle and the answer. It has no other input. No table,
hint or order comes from anywhere else, so the same puzzle and answer always give the same bytes.

Standard library and the checker crate only, no `unsafe`.

## The fixed rule

Variables are the checker's: `var(r, c, d) = 81*r + 9*c + d` with `r`, `c` in `0..9` and `d` in
`1..=9`, so `1..=729`. A positive literal says "cell `(r, c)` holds digit `d`". The clauses are the
checker's CNF of the puzzle (see its README). Every order below is ascending variable number.

A *wrong* literal is `var(r, c, d)` with a digit `d` that differs from the answer in that cell:
648 of them. To *refute* a wrong literal `x` is to make the unit clause `[-x]` hold. A literal is
*already refuted* when unit propagation over the clauses so far falsifies it; such a literal never
gets a lemma, so there is no trivial unit clause in a proof.

1. **Check the input.** Every answer digit must be in `1..=9`, the answer must agree with every
   given, and it must satisfy the Sudoku rules. If not, nothing is written
   (`ANSWER_FORM`, `ANSWER_VS_GIVENS`, `ANSWER_RULES`). If unit propagation over the puzzle's own
   clauses already ends in a conflict, nothing is written (`PUZZLE_CONTRADICTS`).
2. **Repeat until unit propagation decides every cell to the answer's digit:**
   * **Depth 1.** Go through the wrong literals that are not already refuted. If `[-x]` is RUP,
     write it at once, so that later literals of the same pass see it. If the pass wrote anything,
     start step 2 again.
   * **Depth 2.** Otherwise go through the wrong literals that are not already refuted. For `x`,
     collect every literal `y` (ascending) that is still open after assuming `x` and for which
     `[-x, -y]` is RUP. If there is none, take the next `x`. Add all these binary clauses to a copy
     of the clause database; if `[-x]` is not RUP there, take the next `x`. Otherwise trim: go
     through the `y` in ascending order and drop a `y` when `[-x]` is still RUP with the binary
     clauses of the `y` kept so far, without that one. Write `[-x, -y]` for each `y` kept
     (ascending) and then `[-x]`. Start step 2 again.
   * **Split.** Otherwise take the first wrong literal `x` that is not already refuted and make
     `[-x]` RUP by case splits, then write it. To make a clause RUP: if it is RUP already, stop;
     if it is 16 splits deep, give up (`SPLIT_DEPTH`); otherwise look at the assignment under the
     negation of the clause and pick the unassigned positive literal `b` from the cell with the
     fewest unassigned digits, ties to the smallest literal. Make `clause + [-b]` RUP (one level
     deeper) and write it, then make `clause + [b]` RUP and write it. After that the clause is RUP
     (`SPLIT_NOT_CLOSED` if it is not, which does not happen). If every variable is assigned and
     no conflict arises, the puzzle has another solution (`NOT_UNIQUE`; the one-line detail holds
     that grid). Start step 2 again.
   * More than 200,000 lemmas give `LEMMA_CAP`.
3. **Check the result.** The checker (`check_board`) must verify the lemmas against the puzzle
   and give back exactly the answer. A proof it does not verify is never returned (`SELF_CHECK`).

Every lemma is written sorted ascending (by integer value, so `-536 -11 0`) with no repeated
literal, and every lemma is RUP at the moment it is written.

An easy puzzle that unit propagation decides needs no lemma: the proof is empty.

## Command line

```
sudoku-rup-prover prove <puzzle81> <answer81>
sudoku-rup-prover batch <boards.tsv> <out-dir>
```

A puzzle is 81 characters in row-major order: `1`-`9` for a given, `.` or `0` for a blank. An
answer is 81 characters `1`-`9`.

`prove` writes the proof to standard output, one lemma per line closed by `0` (`-536 -11 0`), and
exits 0. A refusal is one line on standard error, `FAILED <REASON> <detail>`, with exit status 2
(`PUZZLE_FORM` or `ANSWER_FORM` if a text does not parse). Wrong usage prints a usage line to
standard error and exits with status 64.

`batch` reads a tab-separated file with one board per line, `<id>\t<puzzle81>\t<answer81>`, where
`<id>` is 1 to 12 digits, and writes `<out-dir>/<id>.drup`. `<out-dir>` must exist. A file is
created only if it does not exist; an existing one is never overwritten (`OUT_EXISTS`). It prints
one line per board

```
<id>\tPROVED\tlemmas=<n> units=<u> binaries=<b> longer=<l>
<id>\tFAILED\t<reason>
```

(a line that cannot be read is labelled `line<N>` and fails with `BAD_LINE` or `BAD_ID`; a proof
that cannot be written fails with `WRITE`), then `TOTAL boards=<n> proved=<p> failed=<f>`. The exit
status is 0 if and only if every board was proved. Nothing is written for a board that failed. An
unreadable boards file (`FAILED READ <path>`) or a missing output directory
(`FAILED OUT_DIR <path>`) is reported on standard error with exit status 2.

## Checking the output

With the checker, one board:

```
sudoku-rup-prover prove "$PUZZLE" "$ANSWER" > board.drup
sudoku-rup-check check "$PUZZLE" board.drup "$ANSWER"     # expect: VERIFIED <answer> lemmas=<n>
```

With the checker, a whole file: both tools read the same three columns (id, puzzle, answer), so
the file given to `batch` can be given to the checker as it is:

```
sudoku-rup-prover batch boards.tsv proofs/
sudoku-rup-check batch boards.tsv proofs/                 # expect: failed=0
```

With `drat-trim`, an independent check of the same claim:

```
sudoku-rup-check dimacs "$PUZZLE" "$ANSWER" > board.cnf
sudoku-rup-check drat board.drup             > board.drat
drat-trim board.cnf board.drat               # expect: s VERIFIED
```

`board.cnf` is the puzzle's CNF plus one clause that forbids `$ANSWER`; it is unsatisfiable exactly
when `$ANSWER` is the only solution. `board.drat` lists the lemmas followed by the empty clause.

## Library

```rust
use sudoku_rup_prover::prove;

let proof = prove(&puzzle, &answer)?;   // puzzle: &[u8; 81] with 0 for a blank, answer: &[u8; 81]
for lemma in &proof.lemmas { /* sorted ascending, closed by 0 when printed */ }
```

`Proof` has `lemmas`, `units`, `binaries`, `longer`, `depth2_rounds` and `split_rounds`.
`ProveError` has a `reason` and a one-line `detail`. The reason codes are `ANSWER_FORM`,
`ANSWER_VS_GIVENS`, `ANSWER_RULES`, `PUZZLE_CONTRADICTS`, `NOT_UNIQUE`, `SPLIT_DEPTH`,
`SPLIT_NOT_CLOSED`, `LEMMA_CAP`, `SELF_CHECK` and `INTERNAL_ERROR`.

## Tests

`cargo test --release -- --nocapture` runs the unit test in `src/lib.rs` and the cells in
`tests/cells.rs`; each passing test prints one `CELL <name> GREEN ...` line. The puzzles are
published ones (the example of the Wikipedia article on Sudoku and some well-known hard ones). The
test file works out each answer itself and asserts that the puzzle has exactly one solution. It
also pins, for each hard puzzle, the number of lemmas of each length, the step counts and a hash of
the proof text, so that any change of the fixed rule shows up. The header of the test file lists
the mutations of `src/lib.rs` that each make a named cell fail. Run the tests in release mode: the
hard puzzles take noticeably longer in a debug build.
