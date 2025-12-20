#![allow(dead_code)]
use std::collections::{HashMap, HashSet};

use rand::{Rng, SeedableRng, prelude::StdRng, seq::IndexedRandom};
use tracing::trace;
use tree_sitter::{Language, Node, Tree};

use tree_sitter_edit::Editor;

use crate::node_types::{NodeTypes, Subtype};

#[derive(Debug, Default)]
struct Edits(HashMap<usize, Vec<u8>>);

impl Editor for Edits {
    fn has_edit(&self, _tree: &Tree, node: &Node<'_>) -> bool {
        self.0.contains_key(&node.id())
    }

    fn edit(&self, _source: &[u8], tree: &Tree, node: &Node<'_>) -> Vec<u8> {
        debug_assert!(self.has_edit(tree, node));
        self.0.get(&node.id()).unwrap().clone()
    }
}

#[derive(Debug)]
struct Branches<'a>(HashMap<&'static str, Vec<&'a [u8]>>);

impl<'a> Branches<'a> {
    fn new(trees: &[(&'a [u8], &'_ Tree)], node_types: &NodeTypes) -> Self {
        let mut branches = HashMap::with_capacity(trees.len()); // min
        for &(text, tree) in trees {
            traverse(tree, |node| {
                branches
                    .entry(node.kind())
                    .or_insert_with(|| HashSet::with_capacity(1))
                    .insert(&text[node.byte_range()]);
            });
        }
        let mut result: HashMap<&'static str, Vec<&'a [u8]>> = branches
            .into_iter()
            .map(|(k, s)| (k, s.into_iter().collect()))
            .collect();

        let mut kinds = result.keys().copied().collect::<HashSet<_>>();
        kinds.extend(node_types.children.keys());
        kinds.extend(node_types.fields.keys());
        let mut queue = Vec::<&str>::new();
        let mut visited = HashSet::<&str>::new();
        for kind in kinds {
            queue.clear();
            visited.clear();
            queue.push(kind);
            let mut entries_to_add = Vec::<&[u8]>::new();

            while let Some(current_kind) = queue.pop() {
                let novel = visited.insert(current_kind);
                if !novel {
                    continue;
                }
                entries_to_add.clear();

                let Some(descendants) = node_types.get_subtypes(current_kind) else {
                    continue;
                };

                for descendant in descendants {
                    if !visited.contains(descendant.as_str()) {
                        queue.push(descendant);
                    }

                    if descendant.as_str() == kind {
                        continue;
                    }
                    if let Some(descendant_entries) = result.get(descendant.as_str()) {
                        entries_to_add.extend(descendant_entries.iter().copied());
                    }
                    result
                        .entry(kind)
                        .or_default()
                        .extend(entries_to_add.iter());
                }
            }
        }

        Branches(result)
    }

    fn possible(&self) -> usize {
        let mut possible_mutations = 0;
        for s in self.0.values() {
            possible_mutations += s.len() - 1;
        }
        possible_mutations
    }
}

fn parse(language: &Language, code: &[u8]) -> Tree {
    let mut parser = tree_sitter::Parser::new();
    parser
        .set_language(language)
        .expect("Failed to set tree-sitter parser language");
    parser.parse(code, None).expect("Failed to parse code")
}

/// Splicing configuration
#[derive(Debug)]
pub struct Config {
    /// Percent chance to perform chaotic mutation
    ///
    /// Chaotic mutations may result in invalid syntax.
    pub chaos: u8,
    /// Percent chance to perform a deletion.
    ///
    /// By default, deletes optional nodes. Chaotic deletions delete any node.
    pub deletions: u8,
    pub language: Language,
    pub intra_splices: usize,
    /// Perform anywhere from zero to this many inter-file splices per test.
    pub inter_splices: usize,
    /// Approximate maximum file size to produce (bytes)
    ///
    /// Some of the input tests should be below this size.
    pub max_size: usize,
    pub node_types: NodeTypes,
    /// Re-parse the file after this many mutations.
    ///
    /// When this is more than `inter_splices`, never re-parse.
    pub reparse: usize,
    pub seed: u64,
}

#[derive(Debug)]
pub struct Splicer<'a> {
    pub language: Language,
    branches: Branches<'a>,
    chaos: u8,
    deletions: u8,
    kinds: Vec<&'static str>,
    intra_splices: usize,
    inter_splices: usize,
    max_size: usize,
    node_types: NodeTypes,
    trees: Vec<(&'a [u8], &'a Tree)>,
    reparse: usize,
    rng: StdRng,
}

impl<'a> Splicer<'a> {
    fn delta(node: &Node<'_>, replace: &[u8]) -> isize {
        isize::try_from(replace.len()).unwrap_or_default()
            - isize::try_from(node.byte_range().len()).unwrap_or_default()
    }

    #[must_use]
    pub fn new(config: Config, files: &'a HashMap<String, (Vec<u8>, Tree)>) -> Option<Self> {
        let mut all_empty = true;
        let trees: Vec<_> = files
            .iter()
            .map(|(_, (txt, tree))| {
                if tree.root_node().child_count() != 0 {
                    all_empty = false;
                }
                (txt.as_ref(), tree)
            })
            .collect();
        if all_empty {
            return None;
        }

        let branches = Branches::new(&trees, &config.node_types);
        let rng = StdRng::seed_from_u64(config.seed);
        let kinds = branches.0.keys().copied().collect();
        Some(Splicer {
            chaos: config.chaos,
            deletions: config.deletions,
            language: config.language,
            branches,
            kinds,
            intra_splices: config.intra_splices,
            inter_splices: config.inter_splices,
            max_size: config.max_size,
            node_types: config.node_types,
            reparse: config.reparse,
            rng,
            trees,
        })
    }

    fn all_nodes(tree: &Tree) -> Vec<Node<'_>> {
        let mut all = Vec::with_capacity(16); // min
        traverse(tree, |node| all.push(node));
        all
    }

    fn delete_node(&mut self, _text: &[u8], nodes: &[Node<'_>]) -> Option<(usize, Vec<u8>, isize)> {
        let delete_ret = |node: &Node<'_>| (node.id(), Vec::new(), Self::delta(node, &[]));

        let chaotic = self.rng.random_range(0..100) < self.chaos;

        let mut node = nodes.choose(&mut self.rng).unwrap();
        if chaotic || nodes.iter().all(|n| !self.node_types.optional_node(n)) {
            return Some(delete_ret(node));
        }
        let mut i = 0;
        while !self.node_types.optional_node(node) {
            node = nodes.choose(&mut self.rng).unwrap();
            if i > 256 {
                trace!("Couldn't find any node to delete");
                return None;
            }
            i += 1;
        }
        Some(delete_ret(node))
    }

    pub fn splice_tree(&mut self, text0: &[u8], mut tree: Tree) -> Option<Vec<u8>> {
        trace!("Mutating file:\n{}", String::from_utf8_lossy(text0));
        // TODO: Assert that text0 and tree.root_node() are the same length?
        let inter_splices = if self.inter_splices <= 1 {
            self.inter_splices
        } else {
            self.rng.random_range(1..self.inter_splices)
        };
        let intra_splices = if self.intra_splices <= 1 {
            self.intra_splices
        } else {
            self.rng.random_range(1..self.intra_splices)
        };
        let splices = inter_splices.saturating_add(intra_splices);
        if splices == 0 {
            return None;
        }

        let mut edits = Edits::default();
        let mut text = Vec::from(text0);
        let mut sz = isize::try_from(text.len()).unwrap_or_default();
        let mut nodes = Self::all_nodes(&tree);
        let mut intra_branches = if self.intra_splices > 0 {
            Branches::new(&[(text0, &tree)], &self.node_types)
        } else {
            Branches::new(&[], &self.node_types)
        };

        for i in 0..splices {
            let result = if self.rng.random_range(0..100) < self.deletions {
                trace!("Performing deletion");
                self.delete_node(&text, &nodes)
            } else if i < self.intra_splices {
                trace!("Performing intra-file splice");
                debug_assert!(!intra_branches.0.is_empty());
                splice(
                    &mut self.rng,
                    self.chaos,
                    &self.kinds,
                    &intra_branches,
                    &text,
                    &nodes,
                    &self.node_types,
                )
            } else {
                trace!("Performing inter-file splice");
                splice(
                    &mut self.rng,
                    self.chaos,
                    &self.kinds,
                    &self.branches,
                    &text,
                    &nodes,
                    &self.node_types,
                )
            };
            let Some((id, bytes, delta)) = result else {
                continue;
            };
            edits.0.insert(id, bytes);
            sz += delta;
            let sz_u = usize::try_from(sz).unwrap_or_default();
            let sized_out = sz_u >= self.max_size;
            if i % self.reparse == 0 || i + 1 == inter_splices || sized_out {
                let mut result = Vec::with_capacity(sz_u);
                tree_sitter_edit::render(&mut result, &tree, &text, &edits).ok()?;
                text = result;
                tree = parse(&self.language, &text);
                nodes = Self::all_nodes(&tree);
                intra_branches = if i < self.intra_splices {
                    Branches::new(&[(text.as_slice(), &tree)], &self.node_types)
                } else {
                    Branches::new(&[], &self.node_types)
                };
                edits.0.clear();
            }
            if sized_out {
                trace!("Test case exceeds max size ({} >= {})", sz_u, self.max_size);
                break;
            }
        }

        Some(text)
    }
}

fn parsed_as<'a>(node: &Node<'_>, node_types: &'a NodeTypes) -> Option<&'a [Subtype]> {
    if !node.is_named() {
        return None;
    }
    let parent = node.parent()?;
    let kind = parent.kind();
    let fields = node_types.fields.get(kind)?;
    let mut cursor = parent.walk();
    for (idx, child) in parent.children(&mut cursor).enumerate() {
        if child.id() == node.id() {
            if let Some(name) = parent.field_name_for_child(idx.try_into().unwrap())
                && let Some(field) = fields.get(name)
            {
                return Some(field.types.as_slice());
            }
            break;
        }
    }
    node_types
        .children
        .get(kind)
        .map(|children| children.types.as_slice())
}

fn splice_candidates<'a>(
    rng: &mut StdRng,
    kinds: &[&'static str],
    branches: &'a Branches<'_>,
    node_types: &NodeTypes,
    chaotic: bool,
    node: &Node<'_>,
) -> &'a [&'a [u8]] {
    trace!("Chose node of kind {}", node.kind());
    let kind = if chaotic {
        let kind = *kinds.choose(rng).unwrap();
        trace!("Chose chaotic kind {kind}");
        kind
    } else if let Some(kinds) = parsed_as(node, node_types)
        && !kinds.is_empty()
    {
        let kind = kinds.choose(rng).unwrap().ty.as_str();
        trace!("Chose parsed-as kind {kind}");
        kind
    } else {
        node.kind()
    };
    if chaotic {
        branches.0[kind].as_slice()
    } else {
        branches.0.get(kind).map(Vec::as_slice).unwrap_or_default()
    }
}

fn splice(
    mut rng: &mut StdRng,
    chaos: u8,
    kinds: &[&'static str],
    branches: &Branches<'_>,
    text: &[u8],
    nodes: &[Node<'_>],
    node_types: &'_ NodeTypes,
) -> Option<(usize, Vec<u8>, isize)> {
    let chaotic = rng.random_range(0..100) < chaos;
    trace!("Chaotic? {chaotic}");

    // When modified trees are re-parsed, their nodes may have novel kinds
    // not in Branches (candidates.len() == 0). Also, avoid not mutating
    // (candidates.len() == 1).
    let mut node;
    let mut candidates;
    let mut i = 0;
    loop {
        node = nodes.choose(&mut rng).unwrap();
        candidates = splice_candidates(rng, kinds, branches, node_types, chaotic, node);
        if candidates.len() > 1 {
            break;
        }
        trace!("No mutation candidates for {}", node.kind());

        // Don't keep going forever. This can happen when performing an
        // intra-file splice on a small input program.
        if i > 256 {
            trace!("Couldn't find any node to mutate");
            return None;
        }
        i += 1;
    }

    trace!(
        "Replacing {}:\n{}",
        node.kind(),
        String::from_utf8_lossy(&text[node.byte_range()])
    );

    // Try to avoid not mutating
    let node_text = &text[node.byte_range()];
    let mut candidate;
    let mut i = 0;
    loop {
        debug_assert!(!candidates.is_empty());
        candidate = *candidates.choose(rng).unwrap();
        if candidate != node_text {
            break;
        }

        // don't keep going forever
        if i > 256 {
            break;
        }
        i += 1;
    }

    trace!("Replacing with:\n{}", String::from_utf8_lossy(candidate));

    // eprintln!(
    //     "Replacing '{}' with '{}'",
    //     std::str::from_utf8(&text[node.byte_range()]).unwrap(),
    //     std::str::from_utf8(candidate).unwrap(),
    // );
    let replace = Vec::from(candidate);
    let delta = Splicer::delta(node, replace.as_slice());
    Some((node.id(), replace, delta))
}

impl Iterator for Splicer<'_> {
    type Item = Vec<u8>;

    fn next(&mut self) -> Option<Self::Item> {
        let mut text;
        let mut tree;
        loop {
            (text, tree) = *self.trees.choose(&mut self.rng).unwrap();
            if text.len() <= self.max_size {
                break;
            }
        }
        self.splice_tree(text, tree.clone())
    }
}

/// Pre-order DFS traversal of `tree`.
///
/// Traversal order doesn't really matter in this file.
fn traverse<'a>(tree: &'a Tree, mut f: impl FnMut(Node<'a>)) {
    let mut cursor = tree.walk();
    let mut visited_children = false;
    loop {
        if visited_children {
            if cursor.goto_next_sibling() {
                visited_children = false;
            } else if !cursor.goto_parent() {
                break;
            }
        } else {
            f(cursor.node());
            if !cursor.goto_first_child() {
                visited_children = true;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Config, Splicer, parsed_as, traverse};
    use crate::node_types::NodeTypes;
    use std::collections::{HashMap, HashSet};
    use tree_sitter::{Node, Parser, Tree};

    fn go(splices: usize, original_program: &str, expected_mutants: &[&str]) {
        let language = tree_sitter_rust::LANGUAGE.into();
        let mut parser = Parser::new();
        parser
            .set_language(&language)
            .expect("Failed to set tree-sitter parser language");

        let tree = parser
            .parse(original_program.as_bytes(), None)
            .expect("Failed to parse code");
        assert!(!tree.root_node().has_error());

        let mut files = HashMap::new();
        files.insert(
            "test.rs".to_string(),
            (original_program.as_bytes().to_vec(), tree),
        );

        let node_types =
            NodeTypes::new(tree_sitter_rust::NODE_TYPES).expect("Failed to parse node types");
        let config = Config {
            chaos: 0,
            deletions: 0,
            language,
            intra_splices: 0,
            inter_splices: splices,
            max_size: 1024,
            node_types,
            reparse: 1,
            seed: 0,
        };

        let splicer = Splicer::new(config, &files).expect("Failed to create splicer");

        let expected = expected_mutants
            .iter()
            .map(|m| m.trim())
            .collect::<HashSet<_>>();
        let mut found_mutants: HashSet<String> = HashSet::new();
        for mutant in splicer.take(256) {
            let mutant_str = String::from_utf8_lossy(&mutant).trim().to_string();

            let tree = parser.parse(mutant.as_slice(), None).unwrap();
            assert!(!tree.root_node().has_error());

            eprintln!("{mutant_str}");
            if expected.contains(&mutant_str.as_str()) {
                found_mutants.insert(mutant_str);
            }
        }

        for expected_mutant in expected {
            assert!(
                found_mutants.contains(expected_mutant),
                "Expected mutant not found in first 256 mutants:\n{expected_mutant}",
            );
        }
    }

    #[test]
    fn readme() {
        go(
            1,
            "
fn even(x: usize) -> bool {
    if x % 2 == 0 {
        return true;
    } else {
        return false;
    }
}
",
            &[
                "
fn even(x: usize) -> usize {
    if x % 2 == 0 {
        return true;
    } else {
        return false;
    }
}
",
                "
fn even(x: bool) -> bool {
    if x % 2 == 0 {
        return true;
    } else {
        return false;
    }
}
",
                "
fn even(x: usize) -> bool {
    if x % 0 == 0 {
        return true;
    } else {
        return false;
    }
}
",
            ],
        );
    }

    #[test]
    fn test_binary_expression_operand_swap() {
        go(
            2,
            "let x = 1 + 2;",
            &[
                "let x = 1;",
                "let x = 2;",
                "let x = 1 + 1;",
                "let x = 2 + 2;",
                //
                // "let x = 2 + 1;"  // TODO: ?
            ],
        );
    }

    fn find_node_by_text<'a>(tree: &'a Tree, text: &[u8], source: &[u8]) -> Option<Node<'a>> {
        let mut candidates = Vec::new();
        traverse(tree, |node| {
            let node_text = &source[node.byte_range()];
            if node_text == text && node.is_named() {
                candidates.push(node);
            }
        });
        candidates.first().copied()
    }

    fn test_parse_as(
        program: &str,
        node_text: &str,
        expected_kind: &str,
        expected_parsed_as: &[&str],
    ) {
        let language = tree_sitter_rust::LANGUAGE.into();
        let mut parser = Parser::new();
        parser.set_language(&language).unwrap();
        let tree = parser.parse(program.as_bytes(), None).unwrap();
        let node_types = NodeTypes::new(tree_sitter_rust::NODE_TYPES).unwrap();
        let source = program.as_bytes();
        let node = find_node_by_text(&tree, node_text.as_bytes(), source).unwrap();
        assert_eq!(node.kind(), expected_kind, "Kind mismatch for {node_text}");
        let parsed_as_result = parsed_as(&node, &node_types);
        let parsed_as_kinds: Vec<&str> = parsed_as_result
            .map(|subtypes| subtypes.iter().map(|s| s.ty.as_str()).collect())
            .unwrap_or_default();
        assert_eq!(
            expected_parsed_as, parsed_as_kinds,
            "parsed_as mismatch for {node_text}",
        );
    }

    #[test]
    fn parse_as_int() {
        test_parse_as(
            "x.1",
            "1",
            "integer_literal",
            &["field_identifier", "integer_literal"],
        );
    }

    #[test]
    fn parse_as_expression() {
        test_parse_as("fn f() { let x = y; }", "y", "identifier", &["_expression"]);
    }

    #[test]
    fn parse_as_let() {
        test_parse_as(
            "fn f() { let x = 0; }",
            "let x = 0;",
            "let_declaration",
            &[
                "_declaration_statement",
                "_expression",
                "expression_statement",
                "label",
            ],
        );
    }
}
