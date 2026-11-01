# sudoku-rup-check

A small, dependency-free checker for Sudoku solutions that come with a proof.
Given a 9x9 puzzle and a list of lemma clauses, it checks every lemma by reverse
unit propagation (RUP) and reports whether the lemmas force a complete grid.
Standard library only, no `unsafe`.

## What it checks

The puzzle is encoded as CNF (the "extended encoding"):

* Variable `var(r, c, d) = 81*r + 9*c + d` with `r`, `c` in `0..9` and `d` in
  `1..=9`, so variables are `1..=729`. A positive literal says "cell `(r, c)`
  holds digit `d`", a negative literal says it does not.
* Clauses, in this fixed order: cell at-least-one (81); cell at-most-one
  (81 * 36); then for rows, columns and boxes, for each unit and digit, one
  at-least-one clause over its nine cells followed by the 36 pairwise
  at-most-one clauses; then one unit clause per given. That is 11988 clauses
  plus the number of givens.

A proof is a list of lemmas. Lemma *i* is accepted if it is RUP with respect to
the CNF and lemmas `0..i`: assigning all its literals false and propagating
unit clauses reaches a conflict. Accepted lemmas join the clause database.

After the last lemma, unit propagation runs with no assumptions. If every cell
then has exactly one true digit, all lemmas being logical consequences of the
rules and givens means that grid is the unique solution. The grid is also
checked directly against the Sudoku rules (rows, columns, boxes, givens),
without using the CNF.

Results:

| Verdict    | Meaning                                                              |
|------------|----------------------------------------------------------------------|
| `VERIFIED` | all lemmas accepted, grid decided and valid (and equal to the expected grid, if one was given) |
| `OPEN`     | all lemmas accepted, but propagation does not decide every cell      |
| `FAILED`   | a lemma is not RUP, the givens contradict, the grid breaks a rule, ... |

## Proof format

Plain text, DIMACS style: integers separated by whitespace, each clause closed
by `0`.

```
c this is a comment
-123 456 0
-7 0
```

* Literals are `-729..=-1` and `1..=729`.
* A clause may span lines. A line whose first non-space character is `c` is a
  comment.
* Deletion lines (`d ...`) are rejected: only additions are supported.
* A bare `0` is the empty clause. The parser accepts it only as the last
  clause (DRAT proofs end with it); the board checker rejects a proof that
  contains it, since the empty clause is not part of a board proof.
* Malformed input is reported with the line number.

## Command line

```
sudoku-rup-check check <puzzle81> <proof-file> [<expected-grid81>]
sudoku-rup-check batch <boards.tsv> <proof-dir>
sudoku-rup-check dataset <test.csv> <proof-dir>
sudoku-rup-check dimacs <puzzle81> <grid81>
sudoku-rup-check drat <proof-file>
```

A puzzle is 81 characters in row-major order: `1`-`9` for a given, `.` or `0` for
a blank. A grid is 81 characters `1`-`9`.

`check` prints one line and sets the exit status:

| Output                              | Exit |
|-------------------------------------|------|
| `VERIFIED <grid81> lemmas=<n>`      | 0    |
| `OPEN decided=<k> lemmas=<n>`       | 1    |
| `FAILED <REASON> lemma=<i or ->`    | 2    |

Reasons: `GIVENS_CONTRADICT`, `EMPTY_CLAUSE_IN_PROOF`, `LITERAL_OUT_OF_RANGE`,
`LEMMA_NOT_RUP`, `PROOF_DERIVES_CONFLICT`, `GRID_RULES_BROKEN`, `GRID_MISMATCH`,
`BAD_PUZZLE`. An unreadable proof file prints `FAILED READ <path>` and a parse
error prints `FAILED PARSE <message> lemma=-`, both with exit status 2.

`batch` reads a tab-separated file with one board per line,
`<id>\t<puzzle81>\t<grid81 or ->`, where `<id>` is 1 to 12 digits and the proof
is `<proof-dir>/<id>.drup`. It prints `<id>\t<VERIFIED|OPEN|FAILED>\t<detail>`
for each board (detail: the grid, `decided=<k>`, or `<REASON> lemma=<i or ->`;
a missing proof file is `NO_PROOF_FILE`), then
`TOTAL boards=<n> verified=<v> open=<o> failed=<f>`. The exit status is 0 if and
only if no board failed.

`dataset` checks proofs against a dataset file directly: every `<id>.drup` in
`<proof-dir>` is the proof for data line `<id>` of the CSV file (the header is
line 0; ids have no leading zero). The puzzle is read from that line's
`question` column and the decided grid must equal its `answer` column, so
`VERIFIED` means the proof holds and its unique solution is the dataset's answer.
It prints `<id>\tVERIFIED\tmatches_answer lemmas=<n>` (or `OPEN` / `FAILED`) per
proof, ascending by id, then the same `TOTAL` line as `batch`; grids are not
printed. A line holding a quote character, or with another field count than the
header, is refused.

`dimacs` and `drat` write the texts described below to standard output and
report errors (`FAILED READ ...`, `FAILED PARSE ...`, exit status 2) on standard
error, so that standard output stays a clean file. Wrong usage prints a usage
line to standard error and exits with status 64.

## Cross-checking with drat-trim

The same claim can be checked by an independent tool:

```
sudoku-rup-check dimacs "$PUZZLE" "$GRID" > board.cnf
sudoku-rup-check drat board.drup          > board.drat
drat-trim board.cnf board.drat            # expect: s VERIFIED
```

`board.cnf` is the puzzle's CNF plus one blocking clause that forbids `$GRID`
(the negation of its 81 cell literals). It is unsatisfiable exactly when `$GRID`
is the only solution. `board.drat` lists the lemmas followed by the empty
clause. With the blocking clause present, propagation over the CNF and the
lemmas forces the grid and falsifies the blocking clause, so the empty clause is
RUP and `drat-trim` reports the refutation as verified.

## Library

The `sudoku_rup_check` library exposes the pieces used by the binary:
`sudoku_cnf`, `Db` (clause database with unit propagation: `rup`, `add`,
`assignment_under`, `value`), `parse_proof`, `check_board` with its `Verdict`,
`rules_hold`, `dimacs_with_block` and `drat_text`.

## Tests

`cargo test -- --nocapture` runs the integration tests in `tests/cells.rs`; each
passing test prints one `CELL <name> GREEN ...` line. They use two widely
published puzzles: the example from the Wikipedia article on Sudoku and Arto
Inkala's puzzle.
