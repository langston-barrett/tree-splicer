use anyhow::Result;

fn main() -> Result<()> {
    tree_splicer::cli::main(
        tree_sitter_openscad_ng::LANGUAGE.into(),
        tree_sitter_openscad_ng::NODE_TYPES,
    )
}
