# Skepa Language Docs (v0.1.x)

## 1. Source Files

- Extension: `.sk`
- Top-level declarations:
  - `import <module>;`
  - `fn <name>(...) -> <Type> { ... }`

## 2. Lexical Rules

- Whitespace is ignored between tokens.
- Comments:
  - `// ...`
  - `/* ... */`
- Identifiers start with letter or `_`, and continue with letter, digit, or `_`.

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

## 3. Imports and Builtins

```sk
import io;
```

Built-in `io` API:

- `io.print(x: String) -> Void`
- `io.println(x: String) -> Void`
- `io.readLine() -> String`

## 4. Types

- `Int`
- `Float`
- `Bool`
- `String`
- `Void`

## 5. Functions

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

`main()` return value is used as process exit code when execution succeeds.

## 6. Statements

Supported statements:

- `let` declaration (typed or inferred)
- assignment
- expression statement
- `if / else / else if`
- `while`
- `for`
- `break`
- `continue`
- `return`

### `for` Syntax

```sk
for (init; condition; step) {
  // body
}
```

Each clause is optional:

- `for (;;)`
- `for (let i = 0;;)`
- `for (; i < n;)`
- `for (;; i = i + 1)`

`init` supports:

- `let` declaration
- assignment
- expression

`step` supports:

- assignment
- expression

Examples:

```sk
let x: Int = 1;
let y = x;
x = 2;

if (x > 0) { y = 1; } else { y = 2; }

while (x < 10) {
  x = x + 1;
  if (x == 5) { break; }
  if (x == 2) { continue; }
}

for (let i = 0; i < 8; i = i + 1) {
  if (i == 2) { continue; }
  if (i == 6) { break; }
}

return y;
```

`break` and `continue` are valid only inside loop bodies (`while` or `for`).

## 7. Expressions

Supported forms:

- Primary: literals, identifiers, dotted paths, grouping
- Calls: `fnName(a, b)`, `io.println("ok")`
- Unary: `+expr`, `-expr`, `!expr`
- Binary operators (precedence high -> low):
  1. `*`, `/`, `%`
  2. `+`, `-`
  3. `<`, `<=`, `>`, `>=`
  4. `==`, `!=`
  5. `&&`
  6. `||`

All binary operators are left-associative.

Short-circuiting:

- `false && rhs` does not evaluate `rhs`
- `true || rhs` does not evaluate `rhs`

## 8. Type Rules

- Variable type is fixed after declaration.
- Assignment type must match target type.
- Conditions for `if`, `while`, and `for` must be `Bool`.
- Function arity and argument types must match function signatures.
- Return type must match function return type.
- Non-`Void` functions must return on all paths.
- `break` and `continue` must appear inside loop bodies.
- Unary `+` and unary `-` require numeric operands (`Int` or `Float`).
- Numeric operations are strict:
  - `Int` ops require `Int` + `Int`
  - `Float` ops require `Float` + `Float`
  - `%` is `Int % Int` only
  - No implicit numeric promotion
  - Mixed numeric operations are rejected

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
- `4`: `Unit`

Decoder rejects invalid magic, unsupported versions, and truncated payloads.

## 11. Not in v0.1.x

- User-defined types/structs
- Collections (arrays/maps)
- Casting and implicit numeric promotion
- String interpolation
- Package manager / dependency system
