# Skepa Language Docs

## 1. Overview

Skepa is a statically typed compiled language with:
- first-class functions (`Fn(...) -> ...`)
- static arrays (`[T; N]`)
- structs and impl methods
- multi-file modules with import/export

Source files use `.sk`.

## 2. Lexical Structure

Identifiers:
- start: `[A-Za-z_]`
- continue: `[A-Za-z0-9_]`

Keywords (reserved):

Module / namespace:
- `import`, `from`, `as`, `export`

Declarations:
- `struct`, `impl`, `fn`, `let`

Control flow:
- `if`, `else`, `while`, `for`, `match`, `break`, `continue`, `return`

Literals:
- `true`, `false`

Primitive types:
- `Int`, `Float`, `Bool`, `String`, `Void`

Comments:
- line: `// ...`
- block: `/* ... */`

String escapes:
- `\n`, `\t`, `\r`, `\"`, `\\`

Operators and delimiters (selected):
- arithmetic: `+`, `-`, `*`, `/`, `%`
- comparison: `==`, `!=`, `<`, `<=`, `>`, `>=`
- logical: `&&`, `||`, `!`
- assignment / arrows: `=`, `->`, `=>`
- grouping / separators: `()`, `[]`, `{}`, `.`, `,`, `:`, `;`

## 3. Formal Grammar (EBNF)

```ebnf
program         = { top_decl } ;

top_decl         = import_decl
                 | export_decl
                 | global_let
                 | struct_decl
                 | impl_decl
                 | fn_decl ;

import_decl      = "import" dotted_path [ "as" ident ] ";"
                 | "from" dotted_path "import" ( "*" | import_item { "," import_item } ) ";" ;

import_item      = ident [ "as" ident ] ;

export_decl      = "export" "{" export_item { "," export_item } "}" [ "from" dotted_path ] ";"
                 | "export" "*" "from" dotted_path ";" ;

export_item      = ident [ "as" ident ] ;

global_let       = "let" ident [ ":" type ] "=" expr ";" ;

struct_decl      = "struct" ident "{" [ field_decl { "," field_decl } [","] ] "}" ;
field_decl       = ident ":" type ;

impl_decl        = "impl" ident "{" { method_decl } "}" ;
method_decl      = "fn" ident "(" [ param_list ] ")" [ "->" type ] block ;

fn_decl          = "fn" ident "(" [ param_list ] ")" [ "->" type ] block ;
param_list       = param { "," param } [","] ;
param            = ident ":" type ;

type             = primitive_type
                 | named_type
                 | array_type
                 | vec_type
                 | fn_type ;

primitive_type   = "Int" | "Float" | "Bool" | "String" | "Void" ;
named_type       = ident { "." ident } ;
array_type       = "[" type ";" int_lit "]" ;
vec_type         = "Vec" "[" type "]" ;
fn_type          = "Fn" "(" [ type_list ] ")" "->" type ;
type_list        = type { "," type } ;

block            = "{" { stmt } "}" ;

stmt             = let_stmt
                 | assign_stmt
                 | expr_stmt
                 | if_stmt
                 | while_stmt
                 | for_stmt
                 | match_stmt
                 | break_stmt
                 | continue_stmt
                 | return_stmt ;

let_stmt         = "let" ident [ ":" type ] "=" expr ";" ;
assign_stmt      = assign_target "=" expr ";" ;
assign_target    = ident
                 | expr "." ident
                 | expr "[" expr "]" { "[" expr "]" } ;
expr_stmt        = expr ";" ;

if_stmt          = "if" "(" expr ")" block [ "else" ( if_stmt | block ) ] ;
while_stmt       = "while" "(" expr ")" block ;
for_stmt         = "for" "(" [ for_init ] ";" [ expr ] ";" [ for_step ] ")" block ;
match_stmt       = "match" "(" expr ")" "{" match_arm { match_arm } "}" ;
match_arm        = match_pattern "=>" block ;
match_pattern    = "_" | match_lit | ( match_lit { "|" match_lit } ) ;
match_lit        = int_lit | float_lit | bool_lit | string_lit ;
for_init         = for_let | for_assign | expr ;
for_step         = for_assign | expr ;
for_let          = "let" ident [ ":" type ] "=" expr ;
for_assign       = assign_target "=" expr ;

break_stmt       = "break" ";" ;
continue_stmt    = "continue" ";" ;
return_stmt      = "return" [ expr ] ";" ;

expr             = logical_or ;
logical_or       = logical_and { "||" logical_and } ;
logical_and      = equality { "&&" equality } ;
equality         = comparison { ("==" | "!=") comparison } ;
comparison       = additive { ("<" | "<=" | ">" | ">=") additive } ;
additive         = multiplicative { ("+" | "-") multiplicative } ;
multiplicative   = unary { ("*" | "/" | "%") unary } ;
unary            = ("+" | "-" | "!") unary | postfix ;
postfix          = primary { call_suffix | field_suffix | index_suffix } ;
call_suffix      = "(" [ expr { "," expr } [","] ] ")" ;
field_suffix     = "." ident ;
index_suffix     = "[" expr "]" ;

primary          = int_lit | float_lit | bool_lit | string_lit
                 | ident
                 | "(" expr ")"
                 | array_lit
                 | array_repeat
                 | struct_lit
                 | fn_lit ;

array_lit        = "[" [ expr { "," expr } ] "]" ;
array_repeat     = "[" expr ";" int_lit "]" ;
struct_lit       = named_type "{" [ struct_field { "," struct_field } [","] ] "}" ;
struct_field     = ident ":" expr ;
fn_lit           = "fn" "(" [ param_list ] ")" "->" type block ;
```

## 4. Module System

### 4.1 Import Forms

- `import a.b;`
- `import a.b as x;`
- `from a.b import f, g as h;`
- `from a.b import *;`

Notes:
- Imports are file-local. Importing `str` in one module does not make `str` visible in other modules.
- `from x import ...` must target a concrete file module. If `x` resolves to a folder namespace root, it is an ambiguity error.

### 4.2 Export Forms

- `export { f, g as h, User, version };`
- `export { f } from a.b;`
- `export * from a.b;`
- multiple export blocks per file are allowed and merged

### 4.3 Path Mapping

For import path `a.b`:
- file candidate: `a/b.sk`
- folder candidate: `a/b/`

For `import a;`:
- if only `a.sk` exists: import that file module
- if only `a/` exists: folder import (recursive)
- if both exist: ambiguity error (`E-MOD-AMBIG`)

### 4.4 Folder Import Recursive Semantics

`import string;` where `string/` is a folder recursively loads all `.sk` files:
- `string/case.sk` -> `string.case`
- `string/nested/trim.sk` -> `string.nested.trim`

These are available through namespace paths (`string.case.up(...)`).

### 4.5 Resolution Algorithm (High-level)

1. Start from entry file (`main.sk`) and BFS/queue parse reachable imports.
2. Build module graph with canonical module ids from relative file paths.
3. Resolve file/folder targets per import path.
4. Detect module graph cycles.
5. Build per-module local symbols: top-level `fn`, `struct`, top-level `let`.
6. Build export maps:
   - merge local export blocks
   - apply re-exports (`export {...} from`, `export * from`)
   - detect duplicate export targets
   - detect re-export cycles
7. Validate imports:
   - imported symbol must be exported
   - wildcard and alias binding conflicts are errors
8. Run sema using module-qualified symbol context.

### 4.6 Conflict and Precedence Rules

- Local names/aliases in `from ... import ...` cannot collide in same module scope.
- Wildcard imports can conflict with prior bindings; conflict is an error.
- Export target names collide after aliasing, not before.
- If same target name appears from multiple export blocks, it is an error.
- Builtin package names (`io`, `str`, `arr`, `datetime`, `random`, `os`, `fs`, `vec`) are reserved package roots.
- `import ns; ns.f(...)` works only when `f` is exported exactly under that namespace level. Example: `import string; string.toUpper(...)` is invalid if only `string.case.toUpper` exists.

## 5. Operator Precedence

Highest to lowest:
1. postfix: call `()`, field `.x`, index `[i]`
2. unary: `+`, `-`, `!`
3. multiplicative: `*`, `/`, `%`
4. additive: `+`, `-`
5. comparison: `<`, `<=`, `>`, `>=`
6. equality: `==`, `!=`
7. logical AND: `&&`
8. logical OR: `||`

Associativity:
- binary operators: left-associative
- unary operators: right-associative

Short-circuit:
- `false && rhs` skips `rhs`
- `true || rhs` skips `rhs`

## 6. Statement Semantics

### 6.1 `if` / `else`

- Conditions must be `Bool`.
- `else if` chains are supported (`else if (...) { ... }`).

### 6.2 `while` / `for`

- Loop conditions must be `Bool` when present.
- `break` and `continue` are only valid inside loops.
- `for` supports omitted clauses: `for (;;) { ... }`.

### 6.3 `match` 

Status:
- Statement only (not a match-expression yet).

Syntax:
- `match (expr) { pattern => { ... } ... }`

Pattern forms:
- wildcard: `_`
- literals: `Int`, `Float`, `Bool`, `String`
- OR-patterns with literals: `1 | 2`, `"y" | "Y"`

Behavior:
- Match target is evaluated exactly once.
- Arms are checked top-to-bottom.
- First matching arm executes.
- No fallthrough.

## 7. Type System Notes

- No implicit numeric promotion.
- `%` is `Int % Int` only.
- Arrays are static-size in type syntax (`[T; N]`, `N` literal).
- Vectors are runtime-sized in type syntax (`Vec[T]`).
- Struct methods: first parameter must be `self: StructName`.
- Function literals are non-capturing.

## 8. Builtin Packages (Current)

- `io`: print/read and formatting helpers
- `str`: string utilities (`len`, `contains`, `startsWith`, `endsWith`, `trim`, `toLower`, `toUpper`, `indexOf`, `lastIndexOf`, `slice`, `replace`, `repeat`, `isEmpty`)
- `arr`: static-array helpers (`len`, `isEmpty`, `contains`, `indexOf`, `count`, `first`, `last`, `join`)
- `datetime`: unix timestamp/time component helpers
- `random`: deterministic seed + random int/float
- `os`: basic host/process helpers (`cwd`, `platform`, `sleep`, `execShell`, `execShellOut`)
- `fs`: basic filesystem helpers (`exists`, `readText`, `writeText`, `appendText`, `mkdirAll`, `removeFile`, `removeDirAll`, `join`)
- `vec`: runtime-sized vector helpers (`new`, `len`, `push`, `get`, `set`, `delete`)

### 8.1 General Rules

- Builtins are accessed through imported package roots (for example, `import str; str.len("x");`).
- Builtin package roots are reserved and cannot be resolved as project modules.
- Builtin calls are type-checked in sema (arity and argument types).
- Builtin runtime behavior may still raise runtime errors (for example invalid values like negative sleep duration).

### 8.2 `io`

Signatures:
- `io.print(s: String) -> Void`
- `io.println(s: String) -> Void`
- `io.printInt(x: Int) -> Void`
- `io.printFloat(x: Float) -> Void`
- `io.printBool(x: Bool) -> Void`
- `io.printString(x: String) -> Void`
- `io.readLine() -> String`
- `io.format(fmt: String, ...) -> String`
- `io.printf(fmt: String, ...) -> Void`

Behavior:
- Printing functions are side-effecting and synchronous.
- `io.format` returns a formatted string; `io.printf` prints formatted output directly.
- Format strings use `%d`, `%f`, `%s`, `%b`, `%%`.

Notes:
- Format strings support basic escapes (`\n`, `\t`, `\\`, `\"`).
- Variadic arguments are type-checked when the format string is a literal.

### 8.3 `str`

Signatures:
- `str.len(s: String) -> Int`
- `str.contains(s: String, needle: String) -> Bool`
- `str.startsWith(s: String, prefix: String) -> Bool`
- `str.endsWith(s: String, suffix: String) -> Bool`
- `str.trim(s: String) -> String`
- `str.toLower(s: String) -> String`
- `str.toUpper(s: String) -> String`
- `str.indexOf(s: String, needle: String) -> Int`
- `str.lastIndexOf(s: String, needle: String) -> Int`
- `str.slice(s: String, start: Int, end: Int) -> String`
- `str.replace(s: String, from: String, to: String) -> String`
- `str.repeat(s: String, count: Int) -> String`
- `str.isEmpty(s: String) -> Bool`

Behavior:
- String helpers are non-mutating (they return derived values).
- String indexing is not exposed directly; use helper functions.

Notes:
- `str.repeat` validates repeat count at runtime.
- Exact `str.len` semantics follow runtime string helper behavior used by the implementation/tests.

### 8.4 `arr`

Signatures:
- `arr.len(a: [T; N]) -> Int`
- `arr.isEmpty(a: [T; N]) -> Bool`
- `arr.contains(a: [T; N], x: T) -> Bool`
- `arr.indexOf(a: [T; N], x: T) -> Int`
- `arr.count(a: [T; N], x: T) -> Int`
- `arr.first(a: [T; N]) -> T`
- `arr.last(a: [T; N]) -> T`
- `arr.join(a: [String; N], sep: String) -> String`

Behavior:
- Array helpers are non-mutating and return values/copies.
- Arrays remain statically-sized in the language type system.

Notes:
- `arr.first` / `arr.last` on empty arrays raise runtime errors.
- `arr.join` is defined for `Array[String]`.

### 8.5 `datetime`

Signatures:
- `datetime.nowUnix() -> Int`
- `datetime.nowMillis() -> Int`
- `datetime.fromUnix(ts: Int) -> String`
- `datetime.fromMillis(ms: Int) -> String`
- `datetime.parseUnix(s: String) -> Int`
- `datetime.year(ts: Int) -> Int`
- `datetime.month(ts: Int) -> Int`
- `datetime.day(ts: Int) -> Int`
- `datetime.hour(ts: Int) -> Int`
- `datetime.minute(ts: Int) -> Int`
- `datetime.second(ts: Int) -> Int`

Behavior:
- `datetime` functions operate on Unix timestamps and UTC-based components.
- `datetime.nowUnix` / `nowMillis` read the host system clock.

Notes:
- `datetime.parseUnix` expects `YYYY-MM-DDTHH:MM:SSZ` and raises runtime errors on invalid input.

### 8.6 `random`

Signatures:
- `random.seed(seed: Int) -> Void`
- `random.int(min: Int, max: Int) -> Int`
- `random.float() -> Float`

Behavior:
- `random.seed` sets deterministic PRNG state for the current runtime host.
- `random.int(min, max)` is inclusive and requires `min <= max`.
- `random.float()` returns a float in `[0.0, 1.0)`.

Notes:
- Random behavior is deterministic for a given seed within the same runtime implementation.

### 8.7 `os`

Signatures:
- `os.cwd() -> String`
- `os.platform() -> String`
- `os.sleep(ms: Int) -> Void`
- `os.execShell(cmd: String) -> Int`
- `os.execShellOut(cmd: String) -> String`

Behavior:
- All `os` functions are synchronous/blocking.
- `os.platform()` returns one of `windows`, `linux`, `macos`.
- `os.sleep(ms)` requires non-negative milliseconds; negative values raise a runtime error.
- `os.execShell(cmd)` runs through the platform shell and returns the process exit code.
- `os.execShellOut(cmd)` runs through the platform shell and returns stdout as `String`.

Notes:
- `os.execShellOut` requires stdout to be valid UTF-8.
- Shell wrapper: Windows uses `cmd /C`; Linux/macOS use `sh -c`.
- `os.execShell*` can be dangerous with untrusted input (shell injection risk).
- If a process exits without a normal exit code, `os.execShell` returns `-1`.

### 8.8 `fs`

Signatures:
- `fs.exists(path: String) -> Bool`
- `fs.readText(path: String) -> String`
- `fs.writeText(path: String, data: String) -> Void`
- `fs.appendText(path: String, data: String) -> Void`
- `fs.mkdirAll(path: String) -> Void`
- `fs.removeFile(path: String) -> Void`
- `fs.removeDirAll(path: String) -> Void`
- `fs.join(a: String, b: String) -> String`

Behavior:
- All `fs` functions are synchronous/blocking.
- `fs.exists` returns `true` for existing files/directories and `false` for missing paths.
- `fs.exists` raises a runtime error if path existence cannot be checked due to a host filesystem error.
- `fs.readText` reads the full file as UTF-8 text.
- `fs.writeText` creates or overwrites a file.
- `fs.appendText` appends to a file and creates it if missing.
- `fs.mkdirAll` recursively creates directories and is safe on an existing directory.
- `fs.removeFile` removes a file path.
- `fs.removeDirAll` recursively removes a directory tree.
- `fs.join` joins path segments using host path semantics and does not check existence.

Notes:
- `fs.readText` raises a runtime error on read failure or invalid UTF-8.
- `fs.removeFile` / `fs.removeDirAll` raise runtime errors for missing paths.

### 8.9 `vec`

Signatures:
- `vec.new() -> Vec[T]` (typed context required)
- `vec.len(v: Vec[T]) -> Int`
- `vec.push(v: Vec[T], x: T) -> Void`
- `vec.get(v: Vec[T], i: Int) -> T`
- `vec.set(v: Vec[T], i: Int, x: T) -> Void`
- `vec.delete(v: Vec[T], i: Int) -> T`

Behavior:
- Vectors are runtime-sized and mutable.
- `vec.push`, `vec.set`, and `vec.delete` mutate the vector in place.
- `vec.delete` removes the element at `i`, shifts later elements left, and returns the removed value.
- Index operations (`get`, `set`, `delete`) require `Int` indices.

Notes:
- `vec.new()` currently requires typed context (for example `let xs: Vec[Int] = vec.new();`).
- Vector values use shared handle semantics: assignment/pass/return aliases the same underlying vector.
- Negative or out-of-bounds indices raise runtime errors.

## 9. Diagnostics (Module/Import/Export)

Stable error codes:
- `E-MOD-NOT-FOUND`
- `E-MOD-CYCLE`
- `E-MOD-AMBIG`
- `E-EXPORT-UNKNOWN`
- `E-IMPORT-NOT-EXPORTED`
- `E-IMPORT-CONFLICT`

Resolver messages include module/path context and may include `did you mean ...` suggestions.

## 10. CLI Quick Reference

- `skepac check <entry.sk>`
- `skepac build <entry.sk> <out.skbc>`
- `skepac disasm <entry.sk | out.skbc>`
- `skeparun run <entry.sk>`
- `skeparun run-bc <out.skbc>`

Runtime env config:
- `SKEPA_MAX_CALL_DEPTH`: maximum VM call depth, required to be an integer `>= 1` when set.
- `--trace` on `skeparun` enables instruction trace output.
