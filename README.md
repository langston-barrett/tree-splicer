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
[releases]: https://github.com/langston-barrett/tree-splicer/releases
[rustup]: https://rustup.rs/
