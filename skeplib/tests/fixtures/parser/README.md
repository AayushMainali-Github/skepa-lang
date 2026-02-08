# Parser Fixture Guide

- Put programs that must parse cleanly in `valid/`.
- Put programs that must produce diagnostics in `invalid/`.
- Keep fixtures small and focused.
- For every parser bug fix, add one new fixture that reproduces the old bug.
