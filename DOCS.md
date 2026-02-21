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

Keywords:
- `import`, `from`, `as`, `export`
- `struct`, `impl`, `fn`, `let`
- `if`, `else`, `while`, `for`, `break`, `continue`, `return`
- `true`, `false`

Primitive types:
- `Int`, `Float`, `Bool`, `String`, `Void`

Comments:
- line: `// ...`
- block: `/* ... */`

String escapes:
- `\n`, `\t`, `\r`, `\"`, `\\`

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
                 | fn_type ;

primitive_type   = "Int" | "Float" | "Bool" | "String" | "Void" ;
named_type       = ident { "." ident } ;
array_type       = "[" type ";" int_lit "]" ;
fn_type          = "Fn" "(" [ type_list ] ")" "->" type ;
type_list        = type { "," type } ;

block            = "{" { stmt } "}" ;

stmt             = let_stmt
                 | assign_stmt
                 | expr_stmt
                 | if_stmt
                 | while_stmt
                 | for_stmt
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
- Builtin package names (`io`, `str`, `arr`, `datetime`, `random`) are reserved package roots.
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

## 6. Type System Notes

- No implicit numeric promotion.
- `%` is `Int % Int` only.
- Arrays are static-size in type syntax (`[T; N]`, `N` literal).
- Struct methods: first parameter must be `self: StructName`.
- Function literals are non-capturing.

## 7. Builtin Packages (Current)

- `io`: print/read and formatting helpers
- `str`: string utilities (`len`, `contains`, `startsWith`, `endsWith`, `trim`, `toLower`, `toUpper`, `indexOf`, `lastIndexOf`, `slice`, `replace`, `repeat`, `isEmpty`)
- `arr`: static-array helpers (`len`, `isEmpty`, `contains`, `indexOf`, `count`, `first`, `last`, `join`)
- `datetime`: unix timestamp/time component helpers
- `random`: deterministic seed + random int/float

## 8. Diagnostics (Module/Import/Export)

Stable error codes:
- `E-MOD-NOT-FOUND`
- `E-MOD-CYCLE`
- `E-MOD-AMBIG`
- `E-EXPORT-UNKNOWN`
- `E-IMPORT-NOT-EXPORTED`
- `E-IMPORT-CONFLICT`

Resolver messages include module/path context and may include `did you mean ...` suggestions.

## 9. CLI Quick Reference

- `skepac check <entry.sk>`
- `skepac build <entry.sk> <out.skbc>`
- `skepac disasm <entry.sk | out.skbc>`
- `skeparun run <entry.sk>`
- `skeparun run-bc <out.skbc>`

Runtime env config:
- `SKEPA_MAX_CALL_DEPTH`: maximum VM call depth, required to be an integer `>= 1` when set.
- `--trace` on `skeparun` enables instruction trace output.
