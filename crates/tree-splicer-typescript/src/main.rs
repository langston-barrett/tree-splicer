use anyhow::Result;

fn main() -> Result<()> {
    tree_splicer::cli::main(
        tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
        tree_sitter_typescript::TYPESCRIPT_NODE_TYPES,
    )
}
