use anyhow::Result;

fn main() -> Result<()> {
    tree_splicer::cli::main(tree_sitter_rust::LANGUAGE.into(), tree_sitter_rust::NODE_TYPES)
}
