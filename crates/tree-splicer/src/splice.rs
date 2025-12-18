#![allow(dead_code)]
use std::collections::{HashMap, HashSet};

use rand::{prelude::StdRng, seq::IndexedRandom, Rng, SeedableRng};
use tree_sitter::{Language, Node, Tree};

use tree_sitter_edit::Editor;

use crate::node_types::NodeTypes;

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
    fn new(trees: &[(&'a [u8], &'a Tree)]) -> Self {
        let mut branches = HashMap::with_capacity(trees.len()); // min
        for &(text, tree) in trees {
            traverse(tree, |node| {
                branches
                    .entry(node.kind())
                    .or_insert_with(|| HashSet::with_capacity(1))
                    .insert(&text[node.byte_range()]);
            });
        }
        Branches(
            branches
                .into_iter()
                .map(|(k, s)| (k, s.iter().copied().collect()))
                .collect(),
        )
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
    // pub intra_splices: usize,
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
    // intra_splices: usize,
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

        let branches = Branches::new(&trees);
        let rng = StdRng::seed_from_u64(config.seed);
        let kinds = branches.0.keys().copied().collect();
        Some(Splicer {
            chaos: config.chaos,
            deletions: config.deletions,
            language: config.language,
            branches,
            kinds,
            // intra_splices: config.intra_splices,
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

    fn delete_node(&mut self, _text: &[u8], nodes: &[Node<'_>]) -> (usize, Vec<u8>, isize) {
        let delete_ret = |node: &Node<'_>| (node.id(), Vec::new(), Self::delta(node, &[]));

        let chaotic = self.rng.random_range(0..100) < self.chaos;

        let mut node = nodes.choose(&mut self.rng).unwrap();
        if chaotic || nodes.iter().all(|n| !self.node_types.optional_node(n)) {
            return delete_ret(node);
        }
        while !self.node_types.optional_node(node) {
            node = nodes.choose(&mut self.rng).unwrap();
        }
        delete_ret(node)
    }

    fn splice_node(&mut self, text: &[u8], nodes: &[Node<'_>]) -> (usize, Vec<u8>, isize) {
        let chaotic = self.rng.random_range(0..100) < self.chaos;

        // When modified trees are re-parsed, their nodes may have novel kinds
        // not in Branches (candidates.len() == 0). Also, avoid not mutating
        // (candidates.len() == 1).
        let mut node;
        let mut candidates;
        loop {
            node = nodes.choose(&mut self.rng).unwrap();
            candidates = if chaotic {
                let kind = *self.kinds.choose(&mut self.rng).unwrap();
                self.branches.0[kind].as_slice()
            } else {
                self.branches
                    .0
                    .get(node.kind())
                    .map(Vec::as_slice)
                    .unwrap_or_default()
            };
            if candidates.len() > 1 {
                break;
            }
        }

        // Try to avoid not mutating
        let node_text = &text[node.byte_range()];
        let mut candidate;
        loop {
            candidate = *candidates.choose(&mut self.rng).unwrap();
            if candidate != node_text {
                break;
            }
        }
        // eprintln!(
        //     "Replacing '{}' with '{}'",
        //     std::str::from_utf8(&text[node.byte_range()]).unwrap(),
        //     std::str::from_utf8(candidate).unwrap(),
        // );
        let replace = Vec::from(candidate);
        let delta = Self::delta(node, replace.as_slice());
        (node.id(), replace, delta)
    }

    pub fn splice_tree(&mut self, text0: &[u8], mut tree: Tree) -> Option<Vec<u8>> {
        // TODO: Assert that text0 and tree.root_node() are the same length?
        if self.inter_splices == 0 {
            return None;
        }
        let splices = self.rng.random_range(1..self.inter_splices);
        let mut edits = Edits::default();
        let mut text = Vec::from(text0);
        let mut sz = isize::try_from(text.len()).unwrap_or_default();
        let mut nodes = Self::all_nodes(&tree);
        for i in 0..splices {
            let (id, bytes, delta) = if self.rng.random_range(0..100) < self.deletions {
                self.delete_node(&text, &nodes)
            } else {
                self.splice_node(&text, &nodes)
            };
            edits.0.insert(id, bytes);
            sz += delta;
            let sz_u = usize::try_from(sz).unwrap_or_default();
            let sized_out = sz_u >= self.max_size;
            if i % self.reparse == 0 || i + 1 == splices || sized_out {
                let mut result = Vec::with_capacity(sz_u);
                tree_sitter_edit::render(&mut result, &tree, &text, &edits).ok()?;
                text = result;
                tree = parse(&self.language, &text);
                nodes = Self::all_nodes(&tree);
                edits.0.clear();
            }
            if sized_out {
                break;
            }
        }
        Some(text)
    }
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
