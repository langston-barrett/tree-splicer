use anyhow::Result;

fn main() -> Result<()> {
    tree_splicer::cli::main(tree_sitter_rust_orchard::LANGUAGE.into(), tree_sitter_rust_orchard::NODE_TYPES)
}
