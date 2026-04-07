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
- `struct`, `impl`, `fn`, `extern`, `let`

Control flow:
- `if`, `else`, `while`, `for`, `match`, `break`, `continue`, `return`

Literals:
- `true`, `false`

Primitive types:
- `Int`, `Float`, `Bool`, `String`, `Bytes`, `Void`

Comments:
- line: `// ...`
- block: `/* ... */`

String escapes:
- `\n`, `\t`, `\r`, `\"`, `\\`

Operators and delimiters (selected):
- arithmetic: `+`, `-`, `*`, `/`, `%`
- comparison: `==`, `!=`, `<`, `<=`, `>`, `>=`
- logical: `&&`, `||`, `!`
- bitwise: `~`, `&`, `|`, `^`, `<<`, `>>`
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
                 | extern_fn_decl
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

extern_fn_decl   = "extern" [ "(" string_lit ")" ] "fn" ident "(" [ param_list ] ")" [ "->" type ] ";" ;
fn_decl          = "fn" ident "(" [ param_list ] ")" [ "->" type ] block ;
param_list       = param { "," param } [","] ;
param            = ident ":" type ;

type             = primitive_type
                 | option_type
                 | named_type
                 | array_type
                 | vec_type
                 | map_type
                 | fn_type ;

primitive_type   = "Int" | "Float" | "Bool" | "String" | "Bytes" | "Void" ;
option_type      = "Option" "[" type "]" ;
named_type       = ident { "." ident } ;
array_type       = "[" type ";" int_lit "]" ;
vec_type         = "Vec" "[" type "]" ;
map_type         = "Map" "[" "String" "," type "]" ;
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
match_pattern    = "_"
                 | match_lit
                 | match_variant
                 | ( match_simple_pattern { "|" match_simple_pattern } ) ;
match_simple_pattern = match_lit | match_variant ;
match_lit        = int_lit | float_lit | bool_lit | string_lit ;
match_variant    = ident [ "(" ident ")" ] ;
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
postfix          = primary { call_suffix | field_suffix | index_suffix | try_suffix } ;
call_suffix      = "(" [ expr { "," expr } [","] ] ")" ;
field_suffix     = "." ident ;
index_suffix     = "[" expr "]" ;
try_suffix       = "?" ;

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
- Builtin package names (`io`, `str`, `option`, `result`, `bytes`, `map`, `arr`, `datetime`, `random`, `os`, `fs`, `net`, `vec`, `task`, `ffi`) are reserved package roots.
- `import ns; ns.f(...)` works only when `f` is exported exactly under that namespace level. Example: `import string; string.toUpper(...)` is invalid if only `string.case.toUpper` exists.

## 5. Operators

### 5.1 Built-in Operators

Arithmetic:
- `+`, `-`, `*`, `/`, `%`

Comparison:
- `==`, `!=`, `<`, `<=`, `>`, `>=`

Logical:
- `!`, `&&`, `||`

Bitwise:
- `~`, `&`, `|`, `^`, `<<`, `>>`

Current bitwise rules:
- bitwise operators are `Int`-only
- shifts require an `Int` right-hand side
- bitwise assignment operators like `&=` and `<<=` are not implemented

### 5.2 User-Defined Infix Operators

Declaration:

```sk
opr xoxo(lhs: Int, rhs: Int) -> Int precedence 9 {
  return lhs * 10 + rhs;
}
```

Use:

```sk
let v = 4 `xoxo` 2;
```

Rules:
- user-defined operators are binary only
- they are used only in backtick infix form
- they lower to ordinary function calls
- precedence only competes with binary operators
- unary and postfix operators always bind tighter than any custom infix operator
- same-module operators may be used before or after their declaration
- direct imports, wildcard imports, and simple re-export chains are supported for infix use
- if the parser cannot know an operator's precedence, it reports a parse error and suggests importing it directly with `from ... import ...`

### 5.3 Operator Precedence

Highest to lowest:
1. postfix: call `()`, field `.x`, index `[i]`
2. unary: `+`, `-`, `!`, `~`
3. multiplicative: `*`, `/`, `%`
4. additive: `+`, `-`
5. shift: `<<`, `>>`
6. bitwise AND: `&`
7. bitwise XOR: `^`
8. bitwise OR: `|`
9. comparison: `<`, `<=`, `>`, `>=`
10. equality: `==`, `!=`
11. logical AND: `&&`
12. logical OR: `||`

User-defined operator precedence:
- user-defined operators participate only in the binary precedence lattice
- higher numeric precedence binds tighter among binary operators
- postfix and unary operators remain structurally above all binary operators

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

Syntax:
- statement form: `match (expr) { pattern => { ... } ... }`
- expression form: `match (expr) { pattern => expr, ... }`

Pattern forms:
- wildcard: `_`
- literals: `Int`, `Float`, `Bool`, `String`
- builtin sum-type variants:
  - `Some(x)`
  - `Some(_)`
  - `None`
  - `Ok(v)`
  - `Ok(_)`
  - `Err(e)`
  - `Err(_)`
- OR-patterns with literals or non-binding variants:
  - `1 | 2`
  - `"y" | "Y"`
  - `None | Some`

Behavior:
- Match target is evaluated exactly once.
- Arms are checked top-to-bottom.
- First matching arm executes.
- No fallthrough.
- Match expressions require every arm expression to have a compatible result type.
- `Option[T]` matches are exhaustive only when they contain:
  - both `Some(...)` and `None`
  - or a wildcard arm `_`
- `Result[T, E]` matches are exhaustive only when they contain:
  - both `Ok(...)` and `Err(...)`
  - or a wildcard arm `_`

Notes:
- Variant bindings destructure the payload into a local visible only inside that arm.
- `Some(_)`, `Ok(_)`, and `Err(_)` match the variant while explicitly discarding the payload.
- `None` is written without payload binding in patterns.
- `Some()`, `Ok()`, `Err()`, and `None()` are invalid pattern forms.
- OR-pattern variant alternatives cannot bind payload names. Use separate arms when binding is needed.

Examples:

```sk
let x: Int = match (Some(1)) {
  Some(v) => v,
  None => 0,
};

let y: Int = match (Ok(7)) {
  Ok(_) => 1,
  Err(_) => 0,
};
```

```sk
match (x) {
  0 => { return 10; }
  _ => { return 20; }
}
```

## 7. Type System Notes

- No implicit numeric promotion.
- `%` is `Int % Int` only.
- Arrays are static-size in type syntax (`[T; N]`, `N` literal).
- Vectors are runtime-sized in type syntax (`Vec[T]`).
- Maps are runtime-sized in type syntax (`Map[String, T]`).
- Struct methods: first parameter must be `self: StructName`.
- Function literals are non-capturing.

## 7.0 Core Semantics

### Value Categories

The language has two important runtime categories:

- value-like types
- shared-reference types

Value-like types behave like ordinary values when assigned, passed, or returned.

Examples:
- `Int`
- `Float`
- `Bool`
- `String`
- `Bytes`
- fixed arrays
- structs made only of value-like fields
- `Option[T]`
- `Result[T, E]`

Shared-reference types alias shared underlying runtime state when assigned, passed, or returned.

Examples:
- `Vec[T]`
- `Map[String, T]`
- `task.Channel[T]`
- `task.Task[T]`
- `net.Socket`
- `net.Listener`
- `ffi.Library`
- `ffi.Symbol`

For shared-reference types:
- assignment does not duplicate the underlying object
- passing to a function does not duplicate the underlying object
- returning from a function does not duplicate the underlying object
- mutation through one alias is visible through every alias
- closing one resource alias closes the shared underlying resource for every alias

### Mutation Visibility

Mutation follows the runtime category of the value:

- mutating a value-like aggregate affects only that value
- mutating a shared-reference value affects the shared underlying object

Examples:
- `vec.push`, `vec.set`, and `vec.delete` mutate the shared vector object
- `map.insert` and `map.remove` mutate the shared map object
- `net.close` closes the shared socket resource, not only one local variable name

### Shadowing And Scope

Lexical scope is block-based.

- function parameters and local `let` bindings occupy the same function-local scope level
- same-scope duplicate local bindings are rejected
- duplicate parameter names are rejected
- inner blocks may shadow outer bindings
- loop bodies may shadow outer bindings
- function parameters introduce bindings in the function-local scope
- match-arm variant bindings exist only inside that arm
- import/export alias collisions are rejected at module scope

Shadowing is allowed only across nested scopes, not within the same scope.

Examples:
- this is rejected:
  - `fn f(x: Int) -> Int { let x: Int = 1; return x; }`
- this is allowed:
  - `fn f(x: Int) -> Int { if (true) { let x: Int = 1; return x; } return x; }`

### Function Values

Function values are first-class, but function literals are non-capturing.

- named functions may be used as `Fn(...) -> ...` values
- function literals may be used as `Fn(...) -> ...` values
- function literals cannot capture outer locals or parameters

This means the language supports first-class function values, not general capturing closures.

Practical consequences:
- `let f: Fn(Int) -> Int = add;` is valid
- `let f: Fn(Int) -> Int = fn(x: Int) -> Int { return x + 1; };` is valid
- `let y = 1; let f: Fn(Int) -> Int = fn(x: Int) -> Int { return x + y; };` is rejected
- passing function values, storing them in arrays/vectors, and returning them is supported
- closure-style environment capture is not part of the language model

### Strict Vs Typed Failure

The core rule is:

- use runtime errors for misuse, invariant violations, and deliberate strict operations
- use `Option[T]` for ordinary absence
- use `Result[T, E]` for ordinary recoverable failure

Package docs may still call out specific strict operations, but the split above is the language-wide rule.

### Equality And Comparison

The comparison operators `<`, `<=`, `>`, and `>=` are numeric/string comparisons only where the operand types support them.

The equality operators `==` and `!=` require compatible operand types.

Structural equality is supported for ordinary value-like types, including:
- `Int`
- `Float`
- `Bool`
- `String`
- `Bytes`
- fixed arrays
- structs
- `Option[T]`
- `Result[T, E]`

Equality is intentionally not supported for:
- `Fn(...) -> ...` values
- `Vec[T]`
- `Map[String, T]`

Reason:
- function values do not have a meaningful language-level equality notion
- vectors and maps are mutable shared-reference types, so structural equality is intentionally not part of the current language model

## 7.1 Error Model

The language error model has three categories:

1. fatal runtime errors
2. typed absence
3. typed recoverable failure

### Fatal Runtime Errors

They are the correct mechanism for:
- programmer bugs
- violated builtin preconditions
- invalid internal states
- misuse of opaque handles
- impossible states at runtime
- unrecoverable host/runtime failures where no typed surface exists yet

Examples that use runtime-error behavior:
- out-of-bounds indexing
- negative sleep duration
- using the wrong `net` handle kind
- use-after-close on `net`, `task`, or `ffi` handles
- invalid UTF-8 passed to helpers that explicitly require valid UTF-8
- calling foreign symbols with the wrong ABI/signature

Runtime errors are process-level failures unless explicitly handled by the embedding/testing runtime. They are not typed language values.

### Typed Absence

`Option[T]` is the language mechanism for absence.

It is the correct mechanism when:
- a value may legitimately be missing
- missing data is expected and common
- callers should branch explicitly instead of crashing

Examples of `Option`-style APIs:
- safe map lookup
- safe environment lookup
- safe collection access helpers

Absence is not an error by itself. It should not use fatal runtime failure when the missing case is part of ordinary control flow.

### Typed Recoverable Failure

`Result[T, E]` is the language mechanism for recoverable failure.

It is the correct mechanism when:
- an operation can fail during normal execution
- the caller is expected to decide how to recover
- the failure should be visible in the type system

Examples of `Result`-style APIs:
- parsing
- filesystem I/O
- network requests
- database operations
- structured FFI boundaries

Recoverable failure should not use fatal runtime failure when the caller can reasonably inspect, transform, retry, or propagate the error.

### Design Rule

When a feature can fail, choose the surface by intent:

- use runtime error for bugs, invariant violations, and misuse
- use `Option[T]` for ordinary absence
- use `Result[T, E]` for ordinary recoverable failure

### Standard Library Error Conventions

The standard library follows these conventions:

- use runtime error for programmer bugs, invariant violations, misuse, and strict operations that cannot sensibly continue
- use `Option[T]` for ordinary absence
- use `Result[T, E]` for ordinary recoverable failure

Builtins that stay runtime-error style:
- indexing operations with invalid bounds
- wrong-type builtin calls
- wrong opaque handle kind passed to a builtin
- use of closed handles
- double-close on handle resources
- invalid ABI use in FFI
- explicit process termination APIs like `os.exit`

Builtins that use `Option[T]` for absence:
- `os.arg`
- `os.envGet`
- `bytes.get`
- `arr.first`
- `arr.last`
- `vec.get`
- `map.get`
- `map.remove`

Builtins that use `Result[T, E]` for recoverable failure:
- `datetime.parseUnix`
- `fs.exists`
- `fs.readText`
- `fs.writeText`
- `fs.appendText`
- `fs.mkdirAll`
- `fs.removeFile`
- `fs.removeDirAll`
- `os.exec`
- `os.execOut`
- `net.connect`
- `net.tlsConnect`
- `net.resolve`
- `net.parseUrl`
- `net.fetch`
- `ffi.open`
- `ffi.bind`
- `net.listen`
- `net.accept`
- `net.read`
- `net.write`
- `net.readBytes`
- `net.writeBytes`
- `net.readN`
- `net.localAddr`
- `net.peerAddr`
- `net.flush`
- `net.setReadTimeout`
- `net.setWriteTimeout`
- `bytes.toString`
- `str.slice`

The language and standard library use a mixed model:
- runtime errors for misuse and strict operations
- `Option[T]` for ordinary absence
- `Result[T, E]` for recoverable operational failure

### `Option[T]`

`Option[T]` is the language mechanism for ordinary absence.

Construction:
- `Some(value)`
- `None()`

Behavior:
- `Option[T]` is a builtin generic enum-like type
- `Some(value)` constructs an option containing a value
- `None()` constructs an empty option
- `Option[T]` values can be assigned, passed, returned, and compared with `==` / `!=`
- `option.isSome(value)` and `option.isNone(value)` inspect option values
- `match` supports `Some(x)` and `None` patterns with exhaustiveness checking
- `None()` relies on surrounding typed context when the inner type cannot be inferred directly

Example:

```sk
import option;

fn wrap(x: Int) -> Option[Int] {
  return Some(x);
}

fn missing() -> Option[Int] {
  return None();
}

fn main() -> Int {
  let a: Option[Int] = wrap(7);
  let b: Option[Int] = Some(7);
  let c: Option[Int] = missing();
  if (a == b && a != c && c == None() && option.isSome(a) && option.isNone(c)) {
    return 0;
  }
  return 1;
}
```

Pattern matching:

```sk
fn unwrapOrZero(x: Option[Int]) -> Int {
  match (x) {
    Some(v) => { return v; }
    None => { return 0; }
  }
}
```

Propagation:

- `expr?` on `Option[T]` unwraps `Some(value)` and yields `value`
- if the value is `None`, the enclosing function-like body returns `None`
- `?` on `Option[...]` is only valid inside a function-like body returning `Option[...]`

Example:

```sk
fn doubleOrNone(x: Option[Int]) -> Option[Int] {
  let value = x?;
  return Some(value + value);
}
```

### `Result[T, E]`

`Result[T, E]` is the language mechanism for ordinary recoverable failure.

Construction:
- `Ok(value)`
- `Err(error)`

Behavior:
- `Result[T, E]` is a builtin generic enum-like type
- `Ok(value)` constructs a success value
- `Err(error)` constructs a failure value
- `Result[T, E]` values can be assigned, passed, returned, and compared with `==` / `!=`
- `result.isOk(value)` and `result.isErr(value)` inspect result values
- `match` supports `Ok(v)` and `Err(e)` patterns with exhaustiveness checking
- `Ok(...)` and `Err(...)` rely on surrounding typed context when the opposite side of the result cannot be inferred directly
- `E` may be a builtin type or a user-defined named type such as a struct

Example:

```sk
import result;

fn wrap(x: Int) -> Result[Int, String] {
  return Ok(x);
}

fn fail() -> Result[Int, String] {
  return Err("bad");
}

fn main() -> Int {
  let a: Result[Int, String] = wrap(7);
  let b: Result[Int, String] = Ok(7);
  let c: Result[Int, String] = fail();
  let d: Result[Int, String] = Err("bad");
  if (a == b && c == d && a != c && result.isOk(a) && result.isErr(c)) {
    return 0;
  }
  return 1;
}
```

Pattern matching:

```sk
fn intoCode(x: Result[Int, String]) -> Int {
  match (x) {
    Ok(v) => { return v; }
    Err(msg) => {
      if (msg == "bad") {
        return 1;
      }
      return 2;
    }
  }
}
```

Structured error payload example:
```sk
struct ParseError {
  code: Int,
  message: String,
}

fn fail() -> Result[Int, ParseError] {
  return Err(ParseError { code: 7, message: "bad" });
}

fn main() -> Int {
  match (fail()) {
    Ok(v) => { return v; }
    Err(err) => {
      if ((err.code == 7) && (err.message == "bad")) {
        return 0;
      }
      return 1;
    }
  }
}
```

Propagation:

- `expr?` on `Result[T, E]` unwraps `Ok(value)` and yields `value`
- if the value is `Err(error)`, the enclosing function-like body returns `Err(error)`
- `?` on `Result[...]` is only valid inside a function-like body returning a compatible `Result[..., E]`

Example:

```sk
fn addBoth(a: Result[Int, String], b: Result[Int, String]) -> Result[Int, String] {
  let left = a?;
  let right = b?;
  return Ok(left + right);
}
```

## 8. Builtin Packages (Current)

- `io`: print/read and formatting helpers
- `str`: string utilities (`len`, `contains`, `startsWith`, `endsWith`, `trim`, `toLower`, `toUpper`, `indexOf`, `lastIndexOf`, `slice`, `replace`, `repeat`, `isEmpty`)
- `option`: option helpers (`isSome`, `isNone`, `unwrapSome`) for `Option[T]`
- `result`: result helpers (`isOk`, `isErr`, `unwrapOk`, `unwrapErr`) for `Result[T, E]`
- `bytes`: byte-string helpers (`fromString`, `toString`, `len`, `get`, `slice`, `concat`, `push`, `append`)
- `map`: string-keyed map helpers (`new`, `len`, `has`, `get`, `insert`, `remove`)
- `arr`: static-array helpers (`len`, `isEmpty`, `contains`, `indexOf`, `count`, `first`, `last`, `join`)
- `datetime`: unix timestamp/time component helpers
- `random`: deterministic seed + random int/float
- `os`: host/process helpers (`platform`, `arch`, `arg`, `envHas`, `envGet`, `envSet`, `envRemove`, `sleep`, `exit`, `exec`, `execOut`)
- `fs`: basic filesystem helpers (`exists`, `readText`, `writeText`, `appendText`, `mkdirAll`, `removeFile`, `removeDirAll`, `join`)
- `net`: blocking TCP/TLS helpers with opaque handle types (`net.Socket`, `net.Listener`)
- `task`: typed task/channel helpers with opaque handle types (`task.Task[T]`, `task.Channel[T]`)
- `ffi`: native-library helpers with opaque handle types (`ffi.Library`, `ffi.Symbol`)
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
- `str.slice(s: String, start: Int, end: Int) -> Result[String, String]`
- `str.replace(s: String, from: String, to: String) -> String`
- `str.repeat(s: String, count: Int) -> String`
- `str.isEmpty(s: String) -> Bool`

Behavior:
- String helpers are non-mutating (they return derived values).
- String indexing is not exposed directly; use helper functions.

Notes:
- `str.repeat` validates repeat count at runtime.
- `str.slice` returns `Ok(String)` on valid bounds and `Err(String)` on invalid bounds.
- Exact `str.len` semantics follow runtime string helper behavior used by the implementation/tests.

### 8.4 `option`

Signatures:
- `option.isSome(value: Option[T]) -> Bool`
- `option.isNone(value: Option[T]) -> Bool`
- `option.unwrapSome(value: Option[T]) -> T`

Behavior:
- `Option[T]` is the language mechanism for ordinary absence.
- `Some(value)` constructs present option values.
- `None()` constructs absent option values.
- `option.isSome` returns `true` when the value is `Some(...)`.
- `option.isNone` returns `true` when the value is `None()`.
- `option.unwrapSome` returns the inner value for `Some(...)`.

Notes:
- `option.unwrapSome` is a strict helper and raises a runtime error when given `None()`.
- `Option[T]` values support language-level `==` / `!=`.
- `match` supports `Some(x)` and `None` patterns directly.
- `expr?` on `Option[T]` unwraps `Some(value)` or returns `None()` from a compatible function-like body.

Example:

```sk
import option;

fn main() -> Int {
  let value: Option[Int] = Some(7);
  if (option.isSome(value) && option.unwrapSome(value) == 7) {
    return 0;
  }
  return 1;
}
```

### 8.5 `result`

Signatures:
- `result.isOk(value: Result[T, E]) -> Bool`
- `result.isErr(value: Result[T, E]) -> Bool`
- `result.unwrapOk(value: Result[T, E]) -> T`
- `result.unwrapErr(value: Result[T, E]) -> E`

Behavior:
- `Result[T, E]` is the language mechanism for ordinary recoverable failure.
- `Ok(value)` constructs success values.
- `Err(error)` constructs failure values.
- `result.isOk` returns `true` for `Ok(...)`.
- `result.isErr` returns `true` for `Err(...)`.
- `result.unwrapOk` returns the success payload.
- `result.unwrapErr` returns the error payload.

Notes:
- `result.unwrapOk` and `result.unwrapErr` are strict helpers and raise runtime errors when used on the wrong variant.
- `Result[T, E]` values support language-level `==` / `!=`.
- `match` supports `Ok(v)` and `Err(e)` patterns directly.
- `expr?` on `Result[T, E]` unwraps `Ok(value)` or returns `Err(error)` from a compatible function-like body.
- `E` may be a builtin type or a user-defined named type such as a struct.

Example:

```sk
import result;

fn parse(flag: Bool) -> Result[Int, String] {
  if (flag) {
    return Ok(7);
  }
  return Err("bad");
}

fn main() -> Int {
  let value: Result[Int, String] = parse(true);
  if (result.isOk(value) && result.unwrapOk(value) == 7) {
    return 0;
  }
  return 1;
}
```

### 8.6 `arr`

Signatures:
- `arr.len(a: [T; N]) -> Int`
- `arr.isEmpty(a: [T; N]) -> Bool`
- `arr.contains(a: [T; N], x: T) -> Bool`
- `arr.indexOf(a: [T; N], x: T) -> Int`
- `arr.count(a: [T; N], x: T) -> Int`
- `arr.first(a: [T; N]) -> Option[T]`
- `arr.last(a: [T; N]) -> Option[T]`
- `arr.join(a: [String; N], sep: String) -> String`

Behavior:
- Array helpers are non-mutating and return values/copies.
- Arrays remain statically-sized in the language type system.

Notes:
- `arr.first` / `arr.last` return `Some(value)` on non-empty arrays and `None()` on empty arrays.
- `arr.join` is defined for `Array[String]`.

### 8.7 `datetime`

Signatures:
- `datetime.nowUnix() -> Int`
- `datetime.nowMillis() -> Int`
- `datetime.fromUnix(ts: Int) -> String`
- `datetime.fromMillis(ms: Int) -> String`
- `datetime.parseUnix(s: String) -> Result[Int, String]`
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
- `datetime.parseUnix` expects `YYYY-MM-DDTHH:MM:SSZ`.
- `datetime.parseUnix` returns `Ok(Int)` on valid input and `Err(String)` on invalid input.

### 8.8 `random`

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

### 8.9 `os`

Signatures:
- `os.platform() -> String`
- `os.arch() -> String`
- `os.arg(index: Int) -> Option[String]`
- `os.envHas(name: String) -> Bool`
- `os.envGet(name: String) -> Option[String]`
- `os.envSet(name: String, value: String) -> Void`
- `os.envRemove(name: String) -> Void`
- `os.sleep(ms: Int) -> Void`
- `os.exit(code: Int) -> Void`
- `os.exec(program: String, args: Vec[String]) -> Result[Int, String]`
- `os.execOut(program: String, args: Vec[String]) -> Result[String, String]`

Behavior:
- All `os` functions are synchronous/blocking.
- `os.platform()` returns one of `windows`, `linux`, `macos`.
- `os.arch()` returns the host architecture string from the runtime environment.
- `os.arg(index)` returns `Some(value)` when the process argument exists and `None()` when it does not.
- `os.envHas(name)` returns whether an environment variable is present.
- `os.envGet(name)` returns `Some(value)` when the variable exists and `None()` when it does not.
- `os.envSet(name, value)` sets an environment variable for the current process.
- `os.envRemove(name)` removes an environment variable from the current process.
- `os.sleep(ms)` requires non-negative milliseconds; negative values raise a runtime error.
- `os.exit(code)` terminates the current process with the provided exit code.
- `os.exec(program, args)` runs the program directly with argv arguments and returns `Ok(exitCode)` on success or `Err(String)` if the process cannot be spawned.
- `os.execOut(program, args)` runs the program directly with argv arguments and returns `Ok(stdout)` on success or `Err(String)` if the process cannot be spawned.

Notes:
- `os.arg(index)` returns `None()` for negative or out-of-range indices.
- `os.envGet(name)` raises a runtime error only for invalid non-UTF-8 environment data.
- `os.execOut(program, args)` uses lossy UTF-8 decoding for stdout and trims trailing line endings.
- If a process exits without a normal exit code, `os.exec(program, args)` returns `Ok(-1)`.

### 8.10 `fs`

Signatures:
- `fs.exists(path: String) -> Result[Bool, String]`
- `fs.readText(path: String) -> Result[String, String]`
- `fs.writeText(path: String, data: String) -> Result[Void, String]`
- `fs.appendText(path: String, data: String) -> Result[Void, String]`
- `fs.mkdirAll(path: String) -> Result[Void, String]`
- `fs.removeFile(path: String) -> Result[Void, String]`
- `fs.removeDirAll(path: String) -> Result[Void, String]`
- `fs.join(a: String, b: String) -> String`

Behavior:
- All `fs` functions are synchronous/blocking.
- `fs.exists` returns `true` for existing files/directories and `false` for missing paths.
- `fs.exists` returns `Ok(Bool)` when path existence can be checked.
- `fs.readText` reads the full file as UTF-8 text and returns `Ok(String)` on success.
- `fs.writeText` creates or overwrites a file and returns `Ok(())` on success.
- `fs.appendText` appends to a file and creates it if missing, returning `Ok(())` on success.
- `fs.mkdirAll` recursively creates directories, is safe on an existing directory, and returns `Ok(())` on success.
- `fs.removeFile` removes a file path and returns `Ok(())` on success.
- `fs.removeDirAll` recursively removes a directory tree and returns `Ok(())` on success.
- `fs.join` joins path segments using host path semantics and does not check existence.

Notes:
- `fs.exists` returns `Err(String)` if path existence cannot be checked due to a host filesystem error.
- `fs.readText` returns `Err(String)` on read failure or invalid UTF-8.
- `fs.writeText`, `fs.appendText`, `fs.mkdirAll`, `fs.removeFile`, and `fs.removeDirAll` return `Err(String)` on filesystem failure.

### 8.11 `bytes`

Signatures:
- `bytes.fromString(s: String) -> Bytes`
- `bytes.toString(b: Bytes) -> Result[String, String]`
- `bytes.len(b: Bytes) -> Int`
- `bytes.get(b: Bytes, i: Int) -> Option[Int]`
- `bytes.slice(b: Bytes, start: Int, end: Int) -> Bytes`
- `bytes.concat(a: Bytes, b: Bytes) -> Bytes`
- `bytes.push(b: Bytes, x: Int) -> Bytes`
- `bytes.append(a: Bytes, b: Bytes) -> Bytes`

Behavior:
- `Bytes` is an immutable runtime-managed byte container.
- All `bytes` helpers are non-mutating and return derived values.
- `bytes.fromString` encodes UTF-8 bytes from a `String`.
- `bytes.toString` decodes UTF-8 and returns `Ok(String)` on valid data.

Notes:
- `bytes.get` returns `Some(byte)` in `0..=255` for in-range access and `None()` otherwise.
- `bytes.toString` returns `Err(String)` on invalid UTF-8.
- `bytes.push` requires the appended byte value to be in `0..=255`.
- `bytes.slice` requires valid non-negative bounds with `start <= end`.
- `bytes.slice` and `bytes.push` are deliberate strict operations. Invalid bounds or invalid byte values raise runtime errors instead of returning `Result`.
- `Bytes` supports language-level `==` / `!=` by content.

### 8.12 `map`

Signatures:
- `map.new() -> Map[String, T]` (typed context required)
- `map.len(m: Map[String, T]) -> Int`
- `map.has(m: Map[String, T], key: String) -> Bool`
- `map.get(m: Map[String, T], key: String) -> Option[T]`
- `map.insert(m: Map[String, T], key: String, value: T) -> Void`
- `map.remove(m: Map[String, T], key: String) -> Option[T]`

Behavior:
- Maps are runtime-sized, mutable, and keyed by `String`.
- `map.insert` mutates the map in place, replacing any existing value for the key.
- `map.remove` removes the key and returns the removed value when present.

Notes:
- `map.new()` currently requires typed context (for example `let headers: Map[String, Int] = map.new();`).
- `Map[String, T]` follows the shared-reference rules from the core semantics section.
- `map.get` returns `Some(value)` when the key exists and `None()` when it does not.
- `map.remove` returns `Some(value)` when the key existed and `None()` when it did not.
- `Map` values cannot currently be compared with `==` or `!=`.

### 8.13 `net`

Opaque types:
- `net.Socket`
- `net.Listener`

Signatures:
- `net.connect(address: String) -> Result[net.Socket, String]`
- `net.tlsConnect(host: String, port: Int) -> Result[net.Socket, String]`
- `net.resolve(host: String) -> Result[String, String]`
- `net.parseUrl(url: String) -> Result[Map[String, String], String]`
- `net.fetch(url: String, options: Map[String, String]) -> Result[Map[String, String], String]`
- `net.listen(address: String) -> Result[net.Listener, String]`
- `net.accept(listener: net.Listener) -> Result[net.Socket, String]`
- `net.read(socket: net.Socket) -> Result[String, String]`
- `net.write(socket: net.Socket, data: String) -> Result[Void, String]`
- `net.readBytes(socket: net.Socket) -> Result[Bytes, String]`
- `net.writeBytes(socket: net.Socket, data: Bytes) -> Result[Void, String]`
- `net.readN(socket: net.Socket, count: Int) -> Result[Bytes, String]`
- `net.localAddr(socket: net.Socket) -> Result[String, String]`
- `net.peerAddr(socket: net.Socket) -> Result[String, String]`
- `net.flush(socket: net.Socket) -> Result[Void, String]`
- `net.setReadTimeout(socket: net.Socket, ms: Int) -> Result[Void, String]`
- `net.setWriteTimeout(socket: net.Socket, ms: Int) -> Result[Void, String]`
- `net.close(socket: net.Socket) -> Void`
- `net.closeListener(listener: net.Listener) -> Void`

Behavior:
- All `net` functions are synchronous/blocking.
- `net.connect(address)` opens a blocking TCP client connection and returns `Ok(net.Socket)` on success or `Err(String)` on connection failure.
- `net.tlsConnect(host, port)` opens a blocking TLS client connection with certificate and hostname verification, returning `Ok(net.Socket)` on success or `Err(String)` on failure.
- `net.resolve(host)` resolves the host name and returns `Ok(String)` with the first resolved IP address as text, or `Err(String)` if resolution fails.
- `net.parseUrl(url)` parses a URL and returns `Ok(Map[String, String])` with keys: `scheme`, `host`, `port`, `path`, `query`, and `fragment`, or `Err(String)` if parsing fails.
- `net.fetch(url, options)` performs a blocking HTTP request and returns `Ok(Map[String, String])` on success or `Err(String)` on request failure.
- `net.listen(address)` binds a blocking TCP listener, returning `Ok(net.Listener)` on success or `Err(String)` on bind failure. Using port `0` lets the OS choose an ephemeral port.
- `net.accept(listener)` blocks until a client connects, then returns `Ok(net.Socket)` on success or `Err(String)` on accept failure.
- `net.read(socket)` performs a single blocking read of up to 4096 bytes and returns `Ok(String)` on success or `Err(String)` on I/O failure.
- `net.write(socket, data)` writes the UTF-8 bytes of `data` to the socket and returns `Ok(())` on success or `Err(String)` on I/O failure.
- `net.readBytes(socket)` performs a single blocking read of up to 4096 bytes and returns `Ok(Bytes)` on success or `Err(String)` on I/O failure.
- `net.writeBytes(socket, data)` writes raw bytes to the socket and returns `Ok(())` on success or `Err(String)` on I/O failure.
- `net.readN(socket, count)` performs a blocking exact read of `count` bytes and returns `Ok(Bytes)` on success or `Err(String)` on I/O failure.
- `net.localAddr(socket)` returns `Ok(String)` with the socket's local address as `host:port`, or `Err(String)` on failure.
- `net.peerAddr(socket)` returns `Ok(String)` with the socket's peer address as `host:port`, or `Err(String)` on failure.
- `net.flush(socket)` flushes pending buffered socket/TLS writes and returns `Ok(())` on success or `Err(String)` on failure.
- `net.setReadTimeout(socket, ms)` sets the read timeout in milliseconds, returning `Ok(())` on success or `Err(String)` on failure. `0` clears the timeout.
- `net.setWriteTimeout(socket, ms)` sets the write timeout in milliseconds, returning `Ok(())` on success or `Err(String)` on failure. `0` clears the timeout.
- `net.close(socket)` closes a socket handle.
- `net.closeListener(listener)` closes a listener handle.

Handle semantics:
- `net.Socket` and `net.Listener` are opaque builtin handle types, not structs.
- Users cannot construct or inspect these types directly.
- `net.Socket` and `net.Listener` follow the shared-reference rules from the core semantics section.
- Closing any alias closes the shared underlying resource for all aliases.

Notes:
- `net` supports both text and byte-oriented I/O.
- `net.read` requires valid UTF-8. Non-UTF-8 payloads return `Err(String)`.
- `net.readBytes` / `net.readN` are the correct APIs for binary protocols and arbitrary payloads.
- `net.read` is not a read-to-EOF helper; it returns one chunk from a single read call.
- `net.tlsConnect` is client-side only in the current surface. There is no TLS listener/accept API yet.
- `net.resolve` returns the first resolved address only. It is a convenience helper, not a full DNS result-set API.
- `net.parseUrl` is a convenience parser for common URLs. Missing optional parts are returned as empty strings in the success map, and invalid URLs return `Err(String)`.
- `net.fetch` currently supports `http://` and `https://` URLs only.
- `net.fetch` reads these option keys when present:
  - `method`
  - `body`
  - `contentType`
- `net.fetch` returns a map with these response keys:
  - `status`
  - `body`
  - `contentType`
- `net.fetch` defaults to `GET` when `method` is missing.
- `net.tlsConnect` validates the peer certificate chain and hostname through the host TLS implementation.
- Timeout setters require non-negative millisecond values. `0` means no timeout.
- Passing the wrong handle kind to a builtin raises a runtime error.
- Using a closed handle raises a runtime error.
- Double-close raises a runtime error.
- Ordinary address, bind, connect, read, write, timeout, resolve, fetch, and TLS handshake failures return `Err(String)`.
- Resources are also cleaned up when the process/runtime exits, but explicit close is still the intended ownership model.

Examples:

Client:
```sk
import net;
import result;

fn main() -> Int {
  let socket: net.Socket = result.unwrapOk(net.connect("127.0.0.1:8080"));
  result.unwrapOk(net.write(socket, "ping"));
  net.close(socket);
  return 0;
}
```

TLS client:
```sk
import net;
import result;

fn main() -> Int {
  let socket: net.Socket = result.unwrapOk(net.tlsConnect("example.com", 443));
  result.unwrapOk(net.write(socket, "GET / HTTP/1.0\r\nHost: example.com\r\n\r\n"));
  let body = result.unwrapOk(net.read(socket));
  net.close(socket);
  if (body != "") {
    return 0;
  }
  return 1;
}
```

Resolve:
```sk
import net;
import result;

fn main() -> Int {
  let ip: String = result.unwrapOk(net.resolve("example.com"));
  if (ip != "") {
    return 0;
  }
  return 1;
}
```

Parse URL:
```sk
import map;
import net;
import option;
import result;

fn main() -> Int {
  let parts: Map[String, String] = result.unwrapOk(net.parseUrl("https://example.com:8443/api?q=1#frag"));
  let host: String = option.unwrapSome(map.get(parts, "host"));
  let path: String = option.unwrapSome(map.get(parts, "path"));
  if ((host == "example.com") && (path == "/api")) {
    return 0;
  }
  return 1;
}
```

Fetch GET:
```sk
import map;
import net;
import option;
import str;
import result;

fn main() -> Int {
  let options: Map[String, String] = map.new();
  let response: Map[String, String] = result.unwrapOk(net.fetch("https://example.com/", options));
  let body: String = option.unwrapSome(map.get(response, "body"));
  if (str.len(body) > 0) {
    return 0;
  }
  return 1;
}
```

Fetch POST:
```sk
import map;
import net;
import option;
import str;
import result;

fn main() -> Int {
  let options: Map[String, String] = map.new();
  map.insert(options, "method", "POST");
  map.insert(options, "body", "{\"ok\":true}");
  map.insert(options, "contentType", "application/json");
  let response: Map[String, String] = result.unwrapOk(net.fetch("https://example.com/api", options));
  let body: String = option.unwrapSome(map.get(response, "body"));
  if (str.len(body) >= 0) {
    return 0;
  }
  return 1;
}
```

Fetch:
```sk
import map;
import net;
import option;
import result;

fn main() -> Int {
  let options: Map[String, String] = map.new();
  map.insert(options, "method", "POST");
  map.insert(options, "body", "{\"ok\":true}");
  map.insert(options, "contentType", "application/json");
  let response: Map[String, String] = result.unwrapOk(net.fetch("https://example.com/api", options));
  let status: String = option.unwrapSome(map.get(response, "status"));
  let body: String = option.unwrapSome(map.get(response, "body"));

  if ((status == "200") && (body != "")) {
    return 0;
  }
  return 1;
}
```

Server:
```sk
import net;
import result;

fn main() -> Int {
  let listener: net.Listener = result.unwrapOk(net.listen("127.0.0.1:8080"));
  let socket: net.Socket = result.unwrapOk(net.accept(listener));
  let req = result.unwrapOk(net.read(socket));
  result.unwrapOk(net.write(socket, "ok"));
  net.close(socket);
  net.closeListener(listener);
  return 0;
}
```

### 8.14 `task`

Opaque types:
- `task.Task[T]`
- `task.Channel[T]`

Signatures:
- `task.channel() -> task.Channel[T]` (typed context required)
- `task.send(ch: task.Channel[T], value: T) -> Void`
- `task.recv(ch: task.Channel[T]) -> T`
- `task.spawn(f: Fn() -> T) -> task.Task[T]`
- `task.join(task: task.Task[T]) -> T`

Behavior:
- `task.channel()` creates a typed runtime-managed channel.
- `task.send` appends a value to the channel queue.
- `task.recv` removes and returns the next queued value.
- `task.spawn` starts a background task in native/CLI execution paths.
- `task.join` waits for task completion and returns the task result exactly once.

Notes:
- `task.channel()` currently requires typed context (for example `let jobs: task.Channel[Int] = task.channel();`).
- `task.Channel[T]` and `task.Task[T]` follow the shared-reference rules from the core semantics section.
- `task.recv` on an empty channel raises a runtime error.
- `task.join` on the same task more than once raises a runtime error.
- The interpreter keeps a deterministic inline fallback for task execution; native and CLI paths use real background threads.

### 8.15 `ffi`

Opaque types:
- `ffi.Library`
- `ffi.Symbol`

Signatures:
- `ffi.open(path: String) -> Result[ffi.Library, String]`
- `ffi.bind(lib: ffi.Library, symbol: String) -> Result[ffi.Symbol, String]`
- `ffi.closeLibrary(lib: ffi.Library) -> Void`
- `ffi.closeSymbol(sym: ffi.Symbol) -> Void`

Preferred user-facing API:
- Linked extern declarations are the primary FFI surface.
- Example:
  - `extern("libc.so.6") fn strlen(s: String) -> Int;`
  - `extern("libc.so.6") fn srand(seed: Int) -> Void;`
- User code should call linked extern declarations directly

Behavior:
- `ffi.open` loads a shared library from the host OS and returns `Ok(ffi.Library)` on success or `Err(String)` on load failure.
- `ffi.bind` looks up a symbol within that library and returns `Ok(ffi.Symbol)` on success or `Err(String)` on lookup failure.
- `ffi.closeLibrary` and `ffi.closeSymbol` close the corresponding handles.
- Linked extern calls are lowered through the runtime FFI layer automatically.

Borrowing and ownership rules:
- `ffi.Library` and `ffi.Symbol` are opaque runtime-managed handles, not structs.
- `ffi.Library` and `ffi.Symbol` follow the shared-reference rules from the core semantics section.
- Closing a library or symbol invalidates that handle for all aliases.
- Supported linked extern ABI shapes currently lower to borrowed-only calls:
  - `extern("lib") fn foo() -> Int`
  - `extern("lib") fn foo() -> Bool`
  - `extern("lib") fn foo() -> Void`
  - `extern("lib") fn foo(Int) -> Int`
  - `extern("lib") fn foo(Int) -> Bool`
  - `extern("lib") fn foo(Int) -> Void`
  - `extern("lib") fn foo(String) -> Int`
  - `extern("lib") fn foo(String) -> Void`
  - `extern("lib") fn foo(Int, Int) -> Int`
  - `extern("lib") fn foo(Bytes, Int) -> Int`
  - `extern("lib") fn foo(String, String) -> Int`
  - `extern("lib") fn foo(String, Int) -> Int`
  - `extern("lib") fn foo(Bytes) -> Int`
- String arguments are passed as temporary borrowed NUL-terminated pointers valid only for the duration of the call.
- `Bytes` arguments are passed as temporary borrowed `(ptr, len)` views valid only for the duration of the call.
- `Int` arguments are passed by value.
- Foreign code must not retain borrowed string or byte pointers after the call returns.
- No ownership transfer APIs exist yet for strings, bytes, or raw pointers.

String and buffer rules:
- Any linked extern call that borrows `String` rejects embedded NUL bytes at runtime.
- Linked extern calls that borrow `Bytes` accept arbitrary byte content, including zero bytes.
- The public FFI surface intentionally stays narrow and borrowed-only. General pointer-based FFI is not exposed yet.

Notes:
- Symbol ABI/signature correctness is the caller's responsibility.
- Calling a symbol with the wrong ABI or signature is undefined behavior at the foreign boundary.
- The runtime still uses low-level `ffi.call...` helpers internally when lowering linked extern calls, but they are not the intended user-facing API.

### 8.16 `vec`

Signatures:
- `vec.new() -> Vec[T]` (typed context required)
- `vec.len(v: Vec[T]) -> Int`
- `vec.push(v: Vec[T], x: T) -> Void`
- `vec.get(v: Vec[T], i: Int) -> Option[T]`
- `vec.set(v: Vec[T], i: Int, x: T) -> Void`
- `vec.delete(v: Vec[T], i: Int) -> T`

Behavior:
- Vectors are runtime-sized and mutable.
- `vec.push`, `vec.set`, and `vec.delete` mutate the vector in place.
- `vec.delete` removes the element at `i`, shifts later elements left, and returns the removed value.
- Index operations (`get`, `set`, `delete`) require `Int` indices.
- `vec.get` returns `Some(value)` for in-range access and `None()` otherwise.

Notes:
- `vec.new()` currently requires typed context (for example `let xs: Vec[Int] = vec.new();`).
- `Vec[T]` follows the shared-reference rules from the core semantics section.
- `vec.get` returns `None()` for negative or out-of-bounds indices.
- `vec.set` and `vec.delete` remain strict and raise runtime errors for invalid indices.
- This split is intentional: `vec.get` models ordinary absence with `Option`, while mutating invalid indices is treated as strict misuse.

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
- `skepac run <entry.sk>`
- `skepac build-native <entry.sk> <out.exe>`
- `skepac build-obj <entry.sk> <out.obj>`
- `skepac build-llvm-ir <entry.sk> <out.ll>`

## 11. Native Workflow

Recommended day-to-day flow:
- `skepac check app.sk`
- `skepac run app.sk`
- `skepac build-native app.sk app.exe`
- `skepac build-llvm-ir app.sk app.ll`

Migration note:
- old backend-specific commands were removed
- the old standalone runner was removed
- native artifacts and LLVM IR are now the supported build/debug outputs
