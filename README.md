# tree-splicer

tree-splicer is a simple grammar-based test case generator. It parses a number
of input files using [tree-sitter][tree-sitter] grammars, and produces new
files formed by splicing together their ASTs.

tree-splicer generates test cases in the [tree-crasher][tree-crasher] fuzzer
and in [icemaker][icemaker], though it can also be used as a standalone tool.

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

### boa

[#2717](https://github.com/boa-dev/boa/issues/2717)
[#2718](https://github.com/boa-dev/boa/issues/2718)
[#2719](https://github.com/boa-dev/boa/issues/2719)

### clang

[#61635](https://github.com/llvm/llvm-project/issues/61635)
[#61666](https://github.com/llvm/llvm-project/issues/61666)
[#61667](https://github.com/llvm/llvm-project/issues/61667)

### deno

[#18338](https://github.com/denoland/deno/issues/18338)

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
[#109146](https://github.com/rust-lang/rust/issues/109146)
[#109147](https://github.com/rust-lang/rust/issues/109147)
[#109148](https://github.com/rust-lang/rust/issues/109148)
[#109152](https://github.com/rust-lang/rust/issues/109152)
[#109178](https://github.com/rust-lang/rust/issues/109178)
[#109188](https://github.com/rust-lang/rust/issues/109188)
[#109191](https://github.com/rust-lang/rust/issues/109191)
[#109204](https://github.com/rust-lang/rust/issues/109204)
[#109232](https://github.com/rust-lang/rust/issues/109232)
[#109239](https://github.com/rust-lang/rust/issues/109239)
[#109296](https://github.com/rust-lang/rust/issues/109296)
[#109297](https://github.com/rust-lang/rust/issues/109297)
[#109298](https://github.com/rust-lang/rust/issues/109298)
[#109299](https://github.com/rust-lang/rust/issues/109299)
[#109300](https://github.com/rust-lang/rust/issues/109300)
[#109304](https://github.com/rust-lang/rust/issues/109304)
[#109305](https://github.com/rust-lang/rust/issues/109305)

### rustfmt

[#5716](https://github.com/rust-lang/rustfmt/issues/5716)

## Installation

### From a release

Statically-linked Linux binaries are available on the [releases page][releases].

### From crates.io

You can build a released version from [crates.io][crates-io]. You'll need the
Rust compiler and the [Cargo][cargo] build tool. [rustup][rustup] makes it very
easy to obtain these. Then, to install the generator for the language `<LANG>`,
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
[icemaker]: https://github.com/matthiaskrgr/icemaker
[radamsa]: https://gitlab.com/akihe/radamsa
[releases]: https://github.com/langston-barrett/tree-splicer/releases
[rustup]: https://rustup.rs/
[tree-crasher]: https://github.com/langston-barrett/tree-crasher
[tree-sitter]: https://tree-sitter.github.io/tree-sitter/
