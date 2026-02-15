# Skepa Language Docs (v0.3.x)

## 1. Source Model

- File extension: `.sk`
- Top-level declarations:
  - `import <module>;`
  - `fn <name>(...) -> <Type> { ... }`

## 2. Lexical Rules

- Whitespace is ignored between tokens.
- Comments:
  - `// ...`
  - `/* ... */`
- Identifiers: start with letter or `_`, continue with letter/digit/`_`.

### Keywords

`import`, `fn`, `let`, `if`, `else`, `while`, `for`, `break`, `continue`, `return`, `true`, `false`

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
- `arr.reverse(a: [T; N]) -> [T; N]`
- `arr.slice(a: [T; N], start: Int, end: Int) -> [T; M]` (end-exclusive)
- `arr.sum(a: [T; N]) -> T`
- `arr.min(a: [Int|Float; N]) -> Int|Float`
- `arr.max(a: [Int|Float; N]) -> Int|Float`
- `arr.sort(a: [Int|Float|String|Bool; N]) -> [Int|Float|String|Bool; N]`
- `arr.distinct(a: [T; N]) -> [T; M]` (stable first-occurrence dedup)
- `arr.join(a: [String; N], sep: String) -> String`

Example:

```sk
import arr;
let xs: [Int; 5] = [7, 2, 9, 2, 5];
let mid = arr.slice(xs, 1, 4);   // [2, 9, 2]
let lo = arr.min(xs);             // 2
let hi = arr.max(xs);             // 9
let sorted = arr.sort(xs);        // [2, 2, 5, 7, 9]
let flags: [Bool; 4] = [true, false, true, false];
let sf = arr.sort(flags);         // [false, false, true, true]
let d = arr.distinct([3, 1, 3, 2]); // [3, 1, 2]
```

`arr.sum` element support:
- `Int` -> numeric sum
- `Float` -> numeric sum
- `String` -> concatenation
- `Array` -> concatenation/flatten one level

Empty-array `arr.sum` behavior (deterministic):
- Returns `0` at runtime (current uniform identity).
- Future typed identities may be introduced (`0.0`, `""`, `[]`) once runtime identity typing is added.

Runtime edge semantics:
- `arr.first` / `arr.last` on empty arrays: runtime `E-VM-INDEX-OOB`.
- `arr.min` / `arr.max` on empty arrays: runtime `E-VM-INDEX-OOB`.
- `arr.slice` out-of-range bounds: runtime `E-VM-INDEX-OOB`.
- `str.repeat` with negative `count`: runtime `E-VM-INDEX-OOB`.
- `str.repeat` output is capped to `1,000,000` bytes; larger outputs fail with runtime `E-VM-INDEX-OOB`.

## 4. Types

- `Int`
- `Float`
- `Bool`
- `String`
- `Void`
- Static arrays: `[T; N]`

Rules:
- `N` is compile-time fixed.
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
- calls: `f(x)`, `io.println("ok")`, `arr.sum(xs)`
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

- User-defined structs/enums
- Dynamic collections (`Vec`, map, set)
- Dynamic array mutation (`push`, `pop`, etc.)
- Casting / implicit numeric promotion
- String interpolation
- Package manager / dependency resolver
