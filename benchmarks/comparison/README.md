# Cross-Language Benchmark Harness

This harness compares Skepa against Python, C, C++, Java, Rust, and Node.js on the same small CPU-bound programs.

It measures:

- build time for compiled languages
- run time for each benchmark case
- checksum output for correctness comparison
- min, median, max, and average runtime per language/case
- normalized scorecard and Skepa head-to-head index

Run from the repository root:

```powershell
.\benchmarks\comparison\run.ps1
```

Useful options:

```powershell
.\benchmarks\comparison\run.ps1 -Runs 10 -Warmups 2
.\benchmarks\comparison\run.ps1 -Languages skepa,cpp,rust,node
.\benchmarks\comparison\run.ps1 -Skepac .\target\release\skepac.exe
```

Generated files:

```text
benchmarks/comparison/.work/      generated source and binaries
benchmarks/comparison/results/    timestamped CSV, JSON, and Markdown reports
```

Each result directory contains:

```text
builds.csv                 build status and build time by language
runs.csv                   raw runtime rows by language and case
scores.csv                 normalized language scorecard
skepa-head-to-head.csv      Skepa-vs-language factors
results.json               all rows in one machine-readable file
summary.md                 human-readable report
```

The current cases are:

```text
arith_loop
bitmix
nested_loops
fib_iter
gcd_chain
prime_count
collatz
branch_mix
function_calls
vec_push_sum
matrix_walk
fib_rec
string_scan
bytes_scan
map_count
option_result
file_read
```

Scoring model:

- Runtime index is the geometric mean of per-case scores where the fastest language for a case is 100.
- Build index is normalized to the fastest nonzero build time. Interpreted/JIT languages with no explicit build step are shown as 100 for build index.
- Composite score is `85% runtime index + 15% build index`.
- Skepa head-to-head runtime factor above `1.0` means Skepa ran faster than that language on the shared successful cases.
- Skepa power index above `100` means Skepa is ahead of that language under the model.

Notes:

- Missing language toolchains are skipped and reported.
- Cases that a language/backend template cannot currently run are reported as `unsupported` instead of being folded into the score.
- The Skepa benchmark uses `skepac build-native`, then runs the produced executable.
- The Criterion benchmarks under `skeplib/benches` measure compiler internals. This harness measures user-visible cross-language program build/run behavior.
