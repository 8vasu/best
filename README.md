# The **best** programming language

**best** is a statically typed Lisp interpreter. Every construct is an
s-expression; a mandatory `declare` form immediately before every `defun`
carries the function's full type signature. A static type checker runs as a
separate pass over the AST before any evaluation begins, so type errors are
caught at compile time and cannot occur at runtime in a well-typed program.

```lisp
(declare factorial (int) int)
(defun factorial (n)
  (if (== n 0)
    1
    (* n (factorial (- n 1)))))

(print (factorial 10))   ; => 3628800
```

---

## Prerequisites

| Tool | Minimum version | Purpose |
|------|----------------|---------|
| [Rust](https://www.rust-lang.org/tools/install) | 1.70 | compiles all hand-written code |
| [flex](https://github.com/westes/flex) | 2.6 | generates the lexer from `best.l` |
| [bison](https://www.gnu.org/software/bison/) | 3.0 | generates the parser from `best.y` |

The `cc` crate (a Cargo build-dependency) compiles the C files that flex and
bison emit -- no separate C toolchain setup is needed beyond having gcc or clang
on `PATH`, which is the default on Debian.

### Debian / Ubuntu

```bash
sudo apt install flex bison build-essential
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

---

## Building

Clone the repository and run `make`:

```bash
git clone <repo-url>
cd best
make          # debug build   -> target/debug/best
make release  # release build -> target/release/best
```

`make` is a thin wrapper around `cargo build`. The build script (`build.rs`)
invokes `flex --posix` and `bison --yacc` automatically -- you do not need to
run them by hand. Generated C files are written to Cargo's `OUT_DIR` and are
never committed to the repository.

To run all bundled example programs:

```bash
make examples
```

To verify the project compiles without producing a binary:

```bash
make check
```

To remove all build artifacts:

```bash
make clean
```

---

## Quick start

```bash
./target/debug/best examples/factorial.best   # 3628800
./target/debug/best examples/fibonacci.best   # 55
./target/debug/best examples/while_loop.best  # 10 9 8 ... 1
./target/debug/best examples/strings.best     # Hello, World / 12
```

---

## Language reference

### Primitive types

| Type    | Literals              |
|---------|-----------------------|
| `int`   | `0`, `42`, `1000`     |
| `float` | `3.14`, `0.5`         |
| `bool`  | `true`, `false`       |
| `str`   | `"hello"`, `"world"`  |
| `void`  | (no literal; returned by statements) |

### Special forms

```lisp
; Variable declaration (immutable type, mutable value)
(let x int 5)

; Assignment
(assign x 10)

; Conditional -- both branches must have the same type
(if (> x 0) (print x) (print 0))

; Loop -- body is one or more expressions
(while (> x 0)
  (print x)
  (assign x (- x 1)))

; Print any non-void value
(print x)
(print "hello")
```

### Functions

A `declare` form carrying the full type signature **must immediately precede**
the corresponding `defun`. Any other form between them is an error.

```lisp
(declare add (int int) int)
(defun add (a b)
  (+ a b))
```

Function bodies are a single expression (use `if`, `while`, or nested calls to
compose multiple steps).

### Built-in operators

| Category    | Operators                          |
|-------------|------------------------------------|
| Arithmetic  | `+`  `-`  `*`  `/`                 |
| Comparison  | `==`  `!=`  `<`  `>`  `<=`  `>=`  |
| Boolean     | `and`  `or`  `not`                 |
| String      | `concat`  `length`                 |

### Comments

`;` begins a comment that runs to the end of the line.

---

## Design decisions

### Grammar notation: POSIX Yacc

The grammar is written in strict POSIX Yacc format (IEEE Std 1003.1). This is a
deliberate portability choice: a POSIX Yacc `.y` file is a self-contained,
formally standardised artifact. Anyone with any POSIX-compliant yacc tool --
Berkeley yacc, byacc, bison in yacc-compatibility mode -- can take `best.y` and
regenerate the parser independently of this project. No vendor lock-in, no
proprietary grammar extensions.

The `.y` file therefore contains no bison-specific directives. There is no
`%define`, no `%code`, no `%language`, no `%locations`, no `%skeleton`, and no
`%empty`. The `yyerror` function is defined by hand as required by the POSIX
standard. The empty alternative in the `program` rule is expressed as a bare `|`
rather than the bison-specific `%empty` keyword.

### Lexer: `flex --posix`

The `--posix` flag instructs flex to enforce compliance with the POSIX
1003.2-1992 lex specification. The `best.l` file therefore:

- Contains no `%option` directives (a flex extension).
- Defines `yywrap()` explicitly to return 1 at end-of-input, as required by
  POSIX lex.
- Uses only constructs defined in the POSIX lex standard: named character-class
  definitions in the definitions section (`DIGIT`, `ID_CHAR`), string-literal
  rules, bracket expressions, and the `+` and `*` quantifiers.

The result is a lexer specification that is portable to any POSIX lex
implementation.

### Parser: `bison --yacc` and LALR(1)

The `--yacc` flag puts bison into strict POSIX compatibility mode. It produces
the standard output file names `y.tab.c` and `y.tab.h`, and rejects any grammar
construct that is not valid POSIX Yacc.

The parsing algorithm is LALR(1) -- the algorithm mandated by the POSIX Yacc
standard. LALR(1) has three properties that matter here:

1. **Static ambiguity detection.** Any ambiguity in the grammar produces a
   shift/reduce or reduce/reduce conflict that bison reports at grammar-compile
   time and refuses to silently resolve. The grammar is proven unambiguous before
   the parser is ever run on source code. PEG parsers, by contrast, use ordered
   choice to silently discard ambiguous alternatives, giving no static guarantee.

2. **Linear time.** LALR(1) parsers run in O(n) time with no backtracking.

3. **Left-recursion support.** The `expr_list` rule is left-recursive
   (`expr_list : expr_list expr | expr`), which is the natural form for a
   sequence. LALR(1) handles left recursion directly. Recursive-descent and
   LL parsers cannot.

> **LALR(1) vs canonical LR(1).** LALR(1) is strictly weaker than canonical
> LR(1): there exist grammars that are LR(1) but not LALR(1). The Dragon Book
> gives a concrete example (Example 4.44 in the first edition, 4.58 in the
> second): a grammar over `{a, b, c, d, e}` that generates exactly four strings.
> An LR(1) automaton builds two distinct states with identical item sets but
> different lookaheads -- no conflicts. The LALR algorithm merges states whose
> item sets are the same, collapsing those lookaheads and introducing a
> reduce/reduce conflict that did not exist before. The grammar is LR(1) but not
> LALR(1). In practice this gap is irrelevant for **best**: a five-rule
> s-expression grammar cannot produce the kind of overlapping lookahead sets that
> trigger it. POSIX Yacc mandates LALR(1), and we accept that constraint knowing
> the grammar is well within it.

The grammar itself is minimal -- five rules -- because s-expression syntax is
structurally uniform. All syntactic structure is expressed through nesting, not
operator precedence or associativity rules. There are no shift/reduce conflicts.

### Parsing algorithm choice

| Algorithm | Ambiguity | Backtracking | Left recursion | Standard |
|-----------|-----------|--------------|----------------|----------|
| LALR(1)   | Static error | None | Native | POSIX |
| PEG       | Silent discard | None | Requires rewrite | No |
| LL(*)     | Runtime detection | Possible | Requires rewrite | No |
| Earley    | Runtime detection | N/A (chart) | Native | No |

ANTLR4 (Adaptive LL(*)) was considered and rejected: it defers ambiguity
detection to runtime, has no stable officially maintained Rust target, and is
not standardised.

For a thorough academic treatment of these trade-offs see Laurence Tratt,
[_Which Parsing Approach?_](https://tratt.net/laurie/blog/2020/which_parsing_approach.html)
(2020).

### Implementation language: Rust

All hand-written code outside `best.l` and `best.y` is Rust. The only C in the
project is generated by flex and bison; it is never touched by hand. This gives
memory safety, a strong type system, and zero-cost abstractions for the AST,
type checker, and evaluator without requiring a C runtime in the final binary.

### FFI boundary

The semantic actions in `best.y` call Rust functions declared `extern "C"`. The
AST nodes are allocated on the Rust heap using `Box::into_raw` and passed
through the grammar as opaque `void *` pointers. Ownership is reclaimed on the
Rust side with `Box::from_raw` when the node is consumed. No C data structures,
no C logic beyond what bison requires syntactically.

`build.rs` invokes flex and bison as build steps, then compiles the generated C
files using the `cc` crate (a build-time dependency only -- zero runtime cost).
The final binary is a standalone executable with no shared-library dependencies
beyond libc.

### Static type checker

The type checker is a separate pass over the AST that runs before any evaluation
begins. This separation is intentional:

- **Correctness.** All type errors are reported before the program produces any
  output or side effects. A program either type-checks completely or does not
  run at all.
- **Clarity.** Type errors carry source locations and precise messages. The
  evaluator can then assume well-typedness and omit redundant checks.

The checker does a pre-pass to collect all `declare` signatures before type-
checking any bodies. This allows mutual recursion and forward calls between
functions regardless of definition order, while still requiring `declare` to
immediately precede its `defun`.

### Turing completeness

**best** is Turing-complete via the combination of `while` (unbounded
iteration), `assign` (mutable state), `if` (conditional branching), and
recursion. Recursive functions achieve the same expressive power as general
recursion.

---

## Project structure

```
best/
  best.l       POSIX lex lexer grammar
  best.y       POSIX yacc parser grammar
  build.rs     invokes flex and bison; compiles generated C via cc crate
  Cargo.toml   cc as build-dependency only
  Makefile     convenience targets: build, release, examples, clean
  examples/
    factorial.best
    fibonacci.best
    while_loop.best
    strings.best
  src/
    main.rs        entry point; calls parse_file, type checker, evaluator
    ast.rs         Expr type; all extern "C" FFI constructors
    error.rs       BestError with source location
    env.rs         TypeEnv and EvalEnv
    typecheck.rs   static type checker pass
    eval.rs        tree-walking evaluator
```
