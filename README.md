# tree-splicer

tree-splicer is a simple grammar-based test case generator (black-box fuzzer).
It uses tree-sitter grammars to parse a number of input files, and produces new
files formed by splicing together parts of the input files.

tree-splicer aims to occupy a different niche from more advanced grammar-based 
fuzzers like Gramatron, Nautilus, and Grammarinator. Rather than achieve
maximal coverage and bug-finding through complete, hand-written grammars and
complex techniques like coverage-based feedback, tree-splicer aims to achieve
maximal ease-of-use by using off-the-shelf tree-sitter grammars and not
requiring any instrumentation (nor even source code) for the target.
In short, tree-splicer wants to be the [Radamsa][radamsa] of grammar-based
fuzzing.

tree-sitter grammars are resistant to syntax errors. Therefore, tree-splicer
can even mutate syntactically-invalid inputs! You can also use tree-splicer
with an incomplete grammar.

## Example

Given this simple Rust program:

```rust
use std::env;

fn even(x: usize) -> bool {
    if x % 2 == 0 {
        return true;
    } else {
        return false;
    }
}

fn main() -> () {
    let argc = env::args().len();
    println!("Hello, world!");
    if even(argc) {
        println!("Even!");
    } else {
        println!("Odd!");
    }
    return ();
}
```

Here are a few candidates created by `tree-splicer-rust`:

```rust
use even::env;

fn even() -> bool {
    if even(argc) {
        println!("Even!");
    } else {
        println!("Odd!");
    }
}

fn std() -> () {
    return true;
}
```
```rust
use args::env;

fn argc(main: usize) -> bool {
    return true;
}

fn even(x: usize) -> bool {
    if x % 2 == 0 {
        return true;
    } else {
        return false;
    }
}
```
```rust
use std::env;

fn x(x: usize) -> bool {
    return true;
}

fn x(x: usize) -> () {
    return false;
}
```

## Supported languages

Languages are easy to add, see
[PR #3](https://github.com/langston-barrett/tree-splicer/pull/3) for an
example.

- JavaScript
- Rust
- TypeScript

## Bugs found

### rustc

[#109066](https://github.com/rust-lang/rust/issues/109066)
[#109071](https://github.com/rust-lang/rust/issues/109071)
[#109072](https://github.com/rust-lang/rust/issues/109072)
[#109078](https://github.com/rust-lang/rust/issues/109078)
[#109079](https://github.com/rust-lang/rust/issues/109079)
[#109090](https://github.com/rust-lang/rust/issues/109090)
[#109129](https://github.com/rust-lang/rust/issues/109129)
[#109141](https://github.com/rust-lang/rust/issues/109141)
[#109143](https://github.com/rust-lang/rust/issues/109143)
[#109144](https://github.com/rust-lang/rust/issues/109144)
[#109145](https://github.com/rust-lang/rust/issues/109145)
[#109146](https://github.com/rust-lang/rust/issues/109146)
[#109147](https://github.com/rust-lang/rust/issues/109147)
[#109148](https://github.com/rust-lang/rust/issues/109148)

## Installation

### From a release

Statically-linked Linux binaries are available on the [releases page][releases].

### From crates.io

You can build a released version from [crates.io][crates-io]. You'll need the
Rust compiler and the [Cargo][cargo] build tool. [rustup][rustup] makes it very
easy to obtain these. Then, to install the reducer for the language `<LANG>`,
run:

```
cargo install tree-splicer-<LANG>
```

This will install binaries in `~/.cargo/bin` by default.

## Build

To build from source, you'll need the Rust compiler and the [Cargo][cargo] build
tool. [rustup][rustup] makes it very easy to obtain these. Then, get the source:

```bash
git clone https://github.com/langston-barrett/tree-splicer
cd tree-splicer
```

Finally, build everything:

```bash
cargo build --release
```

You can find binaries in `target/release`. Run tests with `cargo test`.

[cargo]: https://doc.rust-lang.org/cargo/
[crates-io]: https://crates.io/
[radamsa]: https://gitlab.com/akihe/radamsa
[releases]: https://github.com/langston-barrett/tree-splicer/releases
[rustup]: https://rustup.rs/
