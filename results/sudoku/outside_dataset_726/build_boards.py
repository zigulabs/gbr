"""Builds the checker's input for the puzzles outside the dataset from their public sources.

Usage: python3 build_boards.py results.tsv <sources dir> <package dir> > boards.tsv

<sources dir> holds the four public files named in results.tsv: sudoku.txt (github.com/attractivechaos/plb, file
sudoku/sudoku.txt), top95.txt and hardest.txt (norvig.com) and diabolical.txt (github.com/grantm/sudoku-exchange-puzzle-bank).
<package dir> is this package (one puzzle is taken from verify/sudoku-rup-prover/tests/cells.rs by its name).
Writes one line per puzzle, "<line>\t<puzzle81>\t-": the puzzle text with '.' for a blank and no expected grid, so the checker
decides the grid from the certificate alone. Every puzzle text must hash to its puzzle_sha256 in results.tsv, else it stops."""
import hashlib
import re
import sys

results, sources, package = sys.argv[1:4]
files = {}


def source_lines(name):
    if name not in files:
        files[name] = open(sources + "/" + name, encoding="utf-8").read().split("\n")
    return files[name]


def take(source, where):
    if source == "cells.rs":
        text_rs = open(package + "/verify/sudoku-rup-prover/tests/cells.rs", encoding="utf-8").read()
        return re.search(r'name: "%s",\s*text: "([0-9.]{81})"' % re.escape(where), text_rs).group(1)
    line = source_lines(source)[int(where) - 1]
    if source == "sudoku.txt":
        return line.split("\t")[0].strip()
    if source == "diabolical.txt":
        return line.split()[1]
    return line.strip()


rows = [r.split("\t") for r in open(results, encoding="ascii").read().split("\n") if r]
head = rows[0]
col = {name: head.index(name) for name in ("line", "source", "source_line", "puzzle_sha256")}
for r in rows[1:]:
    text = "".join("." if ch in ".0" else ch for ch in take(r[col["source"]], r[col["source_line"]]))
    if len(text) != 81 or hashlib.sha256(text.encode("ascii")).hexdigest() != r[col["puzzle_sha256"]]:
        sys.exit("PUZZLE_TEXT_MISMATCH line %s (%s %s)" % (r[col["line"]], r[col["source"]], r[col["source_line"]]))
    print("%s\t%s\t-" % (r[col["line"]], text))
