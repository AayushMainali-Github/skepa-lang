# Skepa Language Docs (v0.4.0)

## 1. Source Model

- File extension: `.sk`
- Top-level declarations:
  - `import <module>;`
  - `struct <Name> { ... }`
  - `impl <Name> { ... }`
  - `fn <name>(...) -> <Type> { ... }`

## 2. Lexical Rules

- Whitespace is ignored between tokens.
- Comments:
  - `// ...`
  - `/* ... */`
- Identifiers: start with letter or `_`, continue with letter/digit/`_`.

### Keywords

`import`, `struct`, `impl`, `fn`, `let`, `if`, `else`, `while`, `for`, `break`, `continue`, `return`, `true`, `false`

### Built-in Type Names

`Int`, `Float`, `Bool`, `String`, `Void`

### Literals

- Int: `42`
- Float: `3.14`
- Bool: `true`, `false`
- String: `"hello"`

### String Escapes

Supported escapes: `\n`, `\t`, `\r`, `\"`, `\\`

## 3. Builtin Packages

Imports are explicit:

```sk
import io;
import str;
import arr;
import datetime;
import random;
```

### `io` package

- `io.print(x: String) -> Void`
- `io.println(x: String) -> Void`
- `io.printInt(x: Int) -> Void`
- `io.printFloat(x: Float) -> Void`
- `io.printBool(x: Bool) -> Void`
- `io.printString(x: String) -> Void`
- `io.format(fmt: String, ...args) -> String`
- `io.printf(fmt: String, ...args) -> Void`
- `io.readLine() -> String`

`io.format/io.printf` specifiers:
- `%d` Int
- `%f` Float
- `%s` String
- `%b` Bool
- `%%` literal `%`

### `str` package

- `str.len(s: String) -> Int`
- `str.contains(s: String, needle: String) -> Bool`
- `str.startsWith(s: String, prefix: String) -> Bool`
- `str.endsWith(s: String, suffix: String) -> Bool`
- `str.trim(s: String) -> String`
- `str.toLower(s: String) -> String`
- `str.toUpper(s: String) -> String`
- `str.indexOf(s: String, needle: String) -> Int` (`-1` if not found)
- `str.lastIndexOf(s: String, needle: String) -> Int` (`-1` if not found)
- `str.slice(s: String, start: Int, end: Int) -> String` (end-exclusive, char-indexed)
- `str.replace(s: String, from: String, to: String) -> String`
- `str.repeat(s: String, count: Int) -> String`
- `str.isEmpty(s: String) -> Bool`

### `arr` package

- `arr.len(a: [T; N]) -> Int`
- `arr.isEmpty(a: [T; N]) -> Bool`
- `arr.contains(a: [T; N], x: T) -> Bool`
- `arr.indexOf(a: [T; N], x: T) -> Int` (`-1` if not found)
- `arr.count(a: [T; N], x: T) -> Int`
- `arr.first(a: [T; N]) -> T`
- `arr.last(a: [T; N]) -> T`
- `arr.join(a: [String; N], sep: String) -> String`

### `datetime` package

- `datetime.nowUnix() -> Int`
- `datetime.nowMillis() -> Int`
- `datetime.fromUnix(ts: Int) -> String` (UTC, ISO-like)
- `datetime.fromMillis(ms: Int) -> String` (UTC, ISO-like with milliseconds)
- `datetime.parseUnix(s: String) -> Int` (expects `YYYY-MM-DDTHH:MM:SSZ`)
- `datetime.year(ts: Int) -> Int` (UTC year)
- `datetime.month(ts: Int) -> Int` (UTC month `1..12`)
- `datetime.day(ts: Int) -> Int` (UTC day `1..31`)
- `datetime.hour(ts: Int) -> Int` (UTC hour `0..23`)
- `datetime.minute(ts: Int) -> Int` (UTC minute `0..59`)
- `datetime.second(ts: Int) -> Int` (UTC second `0..59`)

### `random` package

- `random.seed(seed: Int) -> Void`

Example:

```sk
import arr;
let xs: [Int; 5] = [7, 2, 9, 2, 5];
let c = arr.count(xs, 2);         // 2
let first = arr.first(xs);        // 7
let last = arr.last(xs);          // 5
```

Static-size constraints:
- Array concat infers exact size: `[T; N] + [T; M] -> [T; N+M]`.

Runtime edge semantics:
- `arr.first` / `arr.last` on empty arrays: runtime `E-VM-INDEX-OOB`.
- `str.repeat` with negative `count`: runtime `E-VM-INDEX-OOB`.
- `str.repeat` output is capped to `1,000,000` bytes; larger outputs fail with runtime `E-VM-INDEX-OOB`.

## 4. Types

- `Int`
- `Float`
- `Bool`
- `String`
- `Void`
- Static arrays: `[T; N]`
- Named struct types: `User`, `Profile`, etc.

Rules:
- `N` is compile-time fixed.
- In explicit type syntax, `N` must be a compile-time integer literal (not a variable).
- No runtime resize.
- No dynamic/vector operations.
- Multidimensional arrays are supported at arbitrary depth.

## 5. Functions

Example:

```sk
fn add(a: Int, b: Int) -> Int {
  return a + b;
}
```

Entry point:

```sk
fn main() -> Int {
  return 0;
}
```

On successful execution, `main()` return value is used as process exit code (low 8 bits).

## 6. Statements

Supported:
- `let` (typed or inferred)
- assignment
- expression statement
- `if / else / else if`
- `while`
- `for`
- `break`
- `continue`
- `return`

`for` form:

```sk
for (init; condition; step) {
  // body
}
```

Each clause is optional.

`break` and `continue` are valid only inside loop bodies.

## 7. Expressions

Supported:
- literals, identifiers, dotted paths, grouping
- array literals: `[1, 2, 3]`, repeat `[0; 8]`
- indexing and index assignment: `a[i]`, `a[i] = v`
- multidimensional indexing/assignment: `m[i][j]`, `t[a][b][c] = v`
- struct literals: `User { id: 1, name: "sam" }`
- field access/assignment: `u.id`, `u.id = 3`, nested `u.profile.age = 42`
- calls: `f(x)`, `io.println("ok")`, `arr.count(xs, 2)`
- method calls: `u.bump(1)`, `makeUser(9).bump(4)`, `users[i].bump(7)`
- unary: `+`, `-`, `!`
- binary: `* / %`, `+ -`, comparisons, equality, `&&`, `||`

Short-circuit:
- `false && rhs` skips `rhs`
- `true || rhs` skips `rhs`

`+` supports:
- `Int + Int`
- `Float + Float`
- `String + String`
- `Array + Array` when array element types match

Array concat typing:
- `[T; N] + [T; M] => [T; N+M]`

## 8. Type Rules

- Variable type is fixed after declaration.
- Assignment value type must match target type.
- Array element type/size must match declared type.
- Array index type must be `Int`.
- `if` / `while` / `for` condition type must be `Bool`.
- Function arity and argument types must match signatures.
- Return type must match declared function return type.
- Non-`Void` functions must return on all paths.
- Unary `+` / `-` require numeric operands.
- Numeric operations are strict (no implicit promotion).
- `%` supports `Int % Int` only.
- Struct literal rules:
  - all fields are required
  - unknown/duplicate fields are rejected
- Method rules:
  - methods are declared in `impl <Struct>`
  - first parameter must be exactly `self: <Struct>`
  - duplicate method names on the same struct (including across multiple `impl` blocks) are rejected

## 9. Runtime and CLI Contract

### Exit Codes

- `0`: success
- `2`: usage/argument error
- `3`: file IO error
- `10`: parse error (`skepac check`)
- `11`: semantic error
- `12`: codegen/compile error
- `13`: bytecode decode error
- `14`: runtime VM error

### Runtime Error Labels

Examples:
- `E-VM-DIV-ZERO`
- `E-VM-TYPE`
- `E-VM-ARITY`
- `E-VM-STACK-OVERFLOW`
- `E-VM-INDEX-OOB`

Compiler phase labels:
- `E-PARSE`
- `E-SEMA`
- `E-CODEGEN`

## 10. Bytecode (`.skbc`) Format

- Magic header: `SKBC`
- Version: `1` (`u32`, little-endian)
- Function serialization is deterministic (name-sorted)

Value tags:
- `0`: `Int(i64)`
- `1`: `Float(f64)`
- `2`: `Bool(u8)`
- `3`: `String(len + bytes)`
- `4`: `Array(len + items...)`
- `5`: `Unit`

Decoder rejects:
- invalid magic
- unsupported version
- truncated payload

## 11. Out of Scope (Current)

- Enums
- Dynamic collections (`Vec`, map, set)
- Dynamic array mutation (`push`, `pop`, etc.)
- Casting / implicit numeric promotion
- String interpolation
- Package manager / dependency resolver
