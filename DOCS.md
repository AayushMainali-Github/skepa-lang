# Skepa Language Docs (v0)

## 1. Source Files

- Extension: `.sk`
- Top-level declarations:
  - `import <module>;`
  - `fn <name>(...) -> <Type> { ... }`

## 2. Lexical Rules

- Whitespace ignored between tokens
- Comments:
  - `// ...`
  - `/* ... */`
- Identifiers: start with letter or `_`, continue with letter/digit/`_`

### Keywords

`import`, `fn`, `let`, `if`, `else`, `while`, `break`, `continue`, `return`, `true`, `false`

### Built-in Type Names

`Int`, `Float`, `Bool`, `String`, `Void`

### Literals

- Int: `42`
- Float: `3.14`
- Bool: `true`, `false`
- String: `"hello"`

### String Escapes

Supported: `\n`, `\t`, `\r`, `\"`, `\\`

## 3. Imports and Builtins

```sk
import io;
```

Built-in `io` surface:

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

## 6. Statements

- `let` (typed or inferred)
- assignment
- expression statement
- `if / else / else if`
- `while`
- `break`
- `continue`
- `return`

Examples:

```sk
let x: Int = 1;
let y = x;
x = 2;
if (x > 0) { y = 1; } else { y = 2; }
while (x < 10) { x = x + 1; }
if (x == 5) { break; }
if (x == 2) { continue; }
return y;
```

`break` and `continue` are valid only inside `while` loops.

## 7. Expressions

- Primary: literals, identifiers, dotted paths, grouping
- Calls: `fnName(a, b)`, `io.println("ok")`
- Unary: `+expr`, `-expr`, `!expr`
- Binary operators:
  1. `*`, `/`, `%`
  2. `+`, `-`
  3. `<`, `<=`, `>`, `>=`
  4. `==`, `!=`
  5. `&&`
  6. `||`

All binary operators are left-associative.

`&&` and `||` use short-circuit evaluation:
- `false && rhs` does not evaluate `rhs`
- `true || rhs` does not evaluate `rhs`

## 8. Type Rules (v0)

- Variable type is fixed after declaration
- Assignment type must match target type
- Conditions for `if`/`while` must be `Bool`
- Function arity and argument types must match signatures
- Return type must match function return type
- `break` / `continue` must be inside `while`
- Unary `+` and unary `-` require `Int` or `Float`
- Numeric rules are strict:
  - `Int` ops require `Int` + `Int`
  - `Float` ops require `Float` + `Float`
  - `%` is `Int % Int` only
  - No implicit `Int -> Float` promotion
  - Mixed numeric ops are rejected

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

`skeparun` uses `main() -> Int` as process code on successful execution.

### Runtime Error Labels

Examples:

- `E-VM-DIV-ZERO`
- `E-VM-TYPE`
- `E-VM-ARITY`
- `E-VM-STACK-OVERFLOW`

Compiler diagnostic phase labels:

- `E-PARSE`
- `E-SEMA`
- `E-CODEGEN`

## 10. Bytecode (`.skbc`) Format

- Magic header: `SKBC`
- Version: `1` (`u32`, little-endian)
- Deterministic function serialization (name-sorted)

Value tags:

- `0`: `Int(i64)`
- `1`: `Float(f64)`
- `2`: `Bool(u8)`
- `3`: `String(len + bytes)`
- `4`: `Unit`

Decoder rejects invalid magic, unsupported versions, and truncated payloads.

## 11. Unsupported in v0

- User-defined types
- Collections
- `for` loops
- Casting / implicit numeric promotion
- String interpolation
- Package manager features
