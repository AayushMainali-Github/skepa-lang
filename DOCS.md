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
                 | map_type
                 | fn_type ;

primitive_type   = "Int" | "Float" | "Bool" | "String" | "Bytes" | "Void" ;
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
- Builtin package names (`io`, `str`, `bytes`, `map`, `arr`, `datetime`, `random`, `os`, `fs`, `net`, `vec`, `task`, `ffi`) are reserved package roots.
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
- Maps are runtime-sized in type syntax (`Map[String, T]`).
- Struct methods: first parameter must be `self: StructName`.
- Function literals are non-capturing.

## 8. Builtin Packages (Current)

- `io`: print/read and formatting helpers
- `str`: string utilities (`len`, `contains`, `startsWith`, `endsWith`, `trim`, `toLower`, `toUpper`, `indexOf`, `lastIndexOf`, `slice`, `replace`, `repeat`, `isEmpty`)
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
- `os.platform() -> String`
- `os.arch() -> String`
- `os.arg(index: Int) -> String`
- `os.envHas(name: String) -> Bool`
- `os.envGet(name: String) -> String`
- `os.envSet(name: String, value: String) -> Void`
- `os.envRemove(name: String) -> Void`
- `os.sleep(ms: Int) -> Void`
- `os.exit(code: Int) -> Void`
- `os.exec(program: String, args: Vec[String]) -> Int`
- `os.execOut(program: String, args: Vec[String]) -> String`

Behavior:
- All `os` functions are synchronous/blocking.
- `os.platform()` returns one of `windows`, `linux`, `macos`.
- `os.arch()` returns the host architecture string from the runtime environment.
- `os.arg(index)` returns the process argument at `index`.
- `os.envHas(name)` returns whether an environment variable is present.
- `os.envGet(name)` returns the environment variable value.
- `os.envSet(name, value)` sets an environment variable for the current process.
- `os.envRemove(name)` removes an environment variable from the current process.
- `os.sleep(ms)` requires non-negative milliseconds; negative values raise a runtime error.
- `os.exit(code)` terminates the current process with the provided exit code.
- `os.exec(program, args)` runs the program directly with argv arguments and returns the process exit code.
- `os.execOut(program, args)` runs the program directly with argv arguments and returns stdout as `String`.

Notes:
- `os.arg(index)` raises a runtime error for negative or out-of-range indices.
- `os.envGet(name)` raises a runtime error if the variable is missing or not valid UTF-8.
- `os.exec*` raises a runtime error if the program cannot be spawned.
- `os.execOut(program, args)` uses lossy UTF-8 decoding for stdout and trims trailing line endings.
- If a process exits without a normal exit code, `os.exec(program, args)` returns `-1`.

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

### 8.9 `bytes`

Signatures:
- `bytes.fromString(s: String) -> Bytes`
- `bytes.toString(b: Bytes) -> String`
- `bytes.len(b: Bytes) -> Int`
- `bytes.get(b: Bytes, i: Int) -> Int`
- `bytes.slice(b: Bytes, start: Int, end: Int) -> Bytes`
- `bytes.concat(a: Bytes, b: Bytes) -> Bytes`
- `bytes.push(b: Bytes, x: Int) -> Bytes`
- `bytes.append(a: Bytes, b: Bytes) -> Bytes`

Behavior:
- `Bytes` is an immutable runtime-managed byte container.
- All `bytes` helpers are non-mutating and return derived values.
- `bytes.fromString` encodes UTF-8 bytes from a `String`.
- `bytes.toString` decodes UTF-8 and raises a runtime error on invalid data.

Notes:
- `bytes.get` returns the byte value as `Int` in `0..=255`.
- `bytes.push` requires the appended byte value to be in `0..=255`.
- `bytes.slice` requires valid non-negative bounds with `start <= end`.
- `Bytes` supports language-level `==` / `!=` by content.

### 8.10 `map`

Signatures:
- `map.new() -> Map[String, T]` (typed context required)
- `map.len(m: Map[String, T]) -> Int`
- `map.has(m: Map[String, T], key: String) -> Bool`
- `map.get(m: Map[String, T], key: String) -> T`
- `map.insert(m: Map[String, T], key: String, value: T) -> Void`
- `map.remove(m: Map[String, T], key: String) -> T`

Behavior:
- Maps are runtime-sized, mutable, and keyed by `String`.
- `map.insert` mutates the map in place, replacing any existing value for the key.
- `map.remove` removes the key and returns the removed value.

Notes:
- `map.new()` currently requires typed context (for example `let headers: Map[String, Int] = map.new();`).
- Map values use shared handle semantics: assignment/pass/return aliases the same underlying map.
- `map.get` and `map.remove` raise a runtime error for missing keys.
- `Map` values cannot currently be compared with `==` or `!=`.

### 8.11 `net`

Opaque types:
- `net.Socket`
- `net.Listener`

Signatures:
- `net.connect(address: String) -> net.Socket`
- `net.tlsConnect(host: String, port: Int) -> net.Socket`
- `net.resolve(host: String) -> String`
- `net.parseUrl(url: String) -> Map[String, String]`
- `net.fetch(url: String, options: Map[String, String]) -> Map[String, String]`
- `net.listen(address: String) -> net.Listener`
- `net.accept(listener: net.Listener) -> net.Socket`
- `net.read(socket: net.Socket) -> String`
- `net.write(socket: net.Socket, data: String) -> Void`
- `net.readBytes(socket: net.Socket) -> Bytes`
- `net.writeBytes(socket: net.Socket, data: Bytes) -> Void`
- `net.readN(socket: net.Socket, count: Int) -> Bytes`
- `net.localAddr(socket: net.Socket) -> String`
- `net.peerAddr(socket: net.Socket) -> String`
- `net.flush(socket: net.Socket) -> Void`
- `net.setReadTimeout(socket: net.Socket, ms: Int) -> Void`
- `net.setWriteTimeout(socket: net.Socket, ms: Int) -> Void`
- `net.close(socket: net.Socket) -> Void`
- `net.closeListener(listener: net.Listener) -> Void`

Behavior:
- All `net` functions are synchronous/blocking.
- `net.connect(address)` opens a blocking TCP client connection.
- `net.tlsConnect(host, port)` opens a blocking TLS client connection with certificate and hostname verification.
- `net.resolve(host)` resolves the host name and returns the first resolved IP address as text.
- `net.parseUrl(url)` parses a URL and returns a `Map[String, String]` with keys: `scheme`, `host`, `port`, `path`, `query`, and `fragment`.
- `net.fetch(url, options)` performs a blocking HTTP request and returns a response `Map[String, String]`.
- `net.listen(address)` binds a blocking TCP listener. Using port `0` lets the OS choose an ephemeral port.
- `net.accept(listener)` blocks until a client connects, then returns a new `net.Socket`.
- `net.read(socket)` performs a single blocking read of up to 4096 bytes and returns a `String`.
- `net.write(socket, data)` writes the UTF-8 bytes of `data` to the socket.
- `net.readBytes(socket)` performs a single blocking read of up to 4096 bytes and returns raw `Bytes`.
- `net.writeBytes(socket, data)` writes raw bytes to the socket.
- `net.readN(socket, count)` performs a blocking exact read of `count` bytes and returns `Bytes`.
- `net.localAddr(socket)` returns the socket's local address as `host:port`.
- `net.peerAddr(socket)` returns the socket's peer address as `host:port`.
- `net.flush(socket)` flushes pending buffered socket/TLS writes.
- `net.setReadTimeout(socket, ms)` sets the read timeout in milliseconds. `0` clears the timeout.
- `net.setWriteTimeout(socket, ms)` sets the write timeout in milliseconds. `0` clears the timeout.
- `net.close(socket)` closes a socket handle.
- `net.closeListener(listener)` closes a listener handle.

Handle semantics:
- `net.Socket` and `net.Listener` are opaque builtin handle types, not structs.
- Users cannot construct or inspect these types directly.
- Handle assignment/pass/return aliases the same underlying resource; it does not duplicate the socket/listener.
- Closing one alias closes the shared underlying resource for all aliases.

Notes:
- `net` supports both text and byte-oriented I/O.
- `net.read` requires valid UTF-8. Non-UTF-8 payloads raise a runtime error.
- `net.readBytes` / `net.readN` are the correct APIs for binary protocols and arbitrary payloads.
- `net.read` is not a read-to-EOF helper; it returns one chunk from a single read call.
- `net.tlsConnect` is client-side only in the current surface. There is no TLS listener/accept API yet.
- `net.resolve` returns the first resolved address only. It is a convenience helper, not a full DNS result-set API.
- `net.parseUrl` is a convenience parser for common URLs. Missing optional parts are returned as empty strings.
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
- Address parse/bind/connect/read/write failures raise runtime errors.
- TLS handshake and certificate validation failures raise runtime errors.
- Resources are also cleaned up when the process/runtime exits, but explicit close is still the intended ownership model.

Examples:

Client:
```sk
import net;

fn main() -> Int {
  let socket: net.Socket = net.connect("127.0.0.1:8080");
  net.write(socket, "ping");
  net.close(socket);
  return 0;
}
```

TLS client:
```sk
import net;

fn main() -> Int {
  let socket: net.Socket = net.tlsConnect("example.com", 443);
  net.write(socket, "GET / HTTP/1.0\r\nHost: example.com\r\n\r\n");
  let body = net.read(socket);
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

fn main() -> Int {
  let ip: String = net.resolve("example.com");
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

fn main() -> Int {
  let parts: Map[String, String] = net.parseUrl("https://example.com:8443/api?q=1#frag");
  let host: String = map.get(parts, "host");
  let path: String = map.get(parts, "path");
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
import str;

fn main() -> Int {
  let options: Map[String, String] = map.new();
  let response: Map[String, String] = net.fetch("https://example.com/", options);
  let body: String = map.get(response, "body");
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
import str;

fn main() -> Int {
  let options: Map[String, String] = map.new();
  map.insert(options, "method", "POST");
  map.insert(options, "body", "{\"ok\":true}");
  map.insert(options, "contentType", "application/json");
  let response: Map[String, String] = net.fetch("https://example.com/api", options);
  let body: String = map.get(response, "body");
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

fn main() -> Int {
  let options: Map[String, String] = map.new();
  map.insert(options, "method", "POST");
  map.insert(options, "body", "{\"ok\":true}");
  map.insert(options, "contentType", "application/json");

  let response: Map[String, String] = net.fetch("https://example.com/api", options);
  let status: String = map.get(response, "status");
  let body: String = map.get(response, "body");

  if ((status == "200") && (body != "")) {
    return 0;
  }
  return 1;
}
```

Server:
```sk
import net;

fn main() -> Int {
  let listener: net.Listener = net.listen("127.0.0.1:8080");
  let socket: net.Socket = net.accept(listener);
  let req = net.read(socket);
  net.write(socket, "ok");
  net.close(socket);
  net.closeListener(listener);
  return 0;
}
```

### 8.12 `task`

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
- `task.recv` on an empty channel raises a runtime error.
- `task.join` on the same task more than once raises a runtime error.
- The interpreter keeps a deterministic inline fallback for task execution; native and CLI paths use real background threads.

### 8.13 `ffi`

Opaque types:
- `ffi.Library`
- `ffi.Symbol`

Signatures:
- `ffi.open(path: String) -> ffi.Library`
- `ffi.bind(lib: ffi.Library, symbol: String) -> ffi.Symbol`
- `ffi.closeLibrary(lib: ffi.Library) -> Void`
- `ffi.closeSymbol(sym: ffi.Symbol) -> Void`
- `ffi.call0Int(sym: ffi.Symbol) -> Int`
- `ffi.call1Int(sym: ffi.Symbol, value: Int) -> Int`
- `ffi.call1StringInt(sym: ffi.Symbol, value: String) -> Int`
- `ffi.call1BytesInt(sym: ffi.Symbol, value: Bytes) -> Int`

Behavior:
- `ffi.open` loads a shared library from the host OS.
- `ffi.bind` looks up a symbol within that library.
- `ffi.closeLibrary` and `ffi.closeSymbol` close the corresponding handles.
- `ffi.call0Int` calls a zero-argument foreign function returning an integer.
- `ffi.call1Int` calls a one-argument integer foreign function.
- `ffi.call1StringInt` calls a one-argument foreign function that receives a borrowed string and returns an integer.
- `ffi.call1BytesInt` calls a one-argument foreign function that receives borrowed bytes and returns an integer.

Borrowing and ownership rules:
- `ffi.Library` and `ffi.Symbol` are opaque runtime-managed handles, not structs.
- Closing a library or symbol invalidates that handle for all aliases.
- `ffi.call1StringInt` passes a temporary borrowed NUL-terminated string pointer for the duration of the call only.
- `ffi.call1BytesInt` passes a temporary borrowed `(ptr, len)` byte view for the duration of the call only.
- Foreign code must not retain these borrowed string/byte pointers after the call returns.
- No ownership transfer APIs exist yet for strings, bytes, or raw pointers.

String and buffer rules:
- `ffi.call1StringInt` rejects `String` values containing an embedded NUL byte at runtime.
- `ffi.call1BytesInt` accepts arbitrary `Bytes`, including zero bytes.
- These call helpers intentionally cover only narrow, explicit ABI shapes. General pointer-based FFI is not exposed yet.

Notes:
- Symbol ABI/signature correctness is the caller's responsibility.
- Calling a symbol with the wrong ABI or signature is undefined behavior at the foreign boundary.
- The current FFI surface is intentionally narrow while ownership rules remain explicit and testable.

### 8.14 `vec`

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
