#![allow(dead_code)]
use std::collections::{HashMap, HashSet};

use rand::{prelude::StdRng, Rng, SeedableRng};
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
    fn new(trees: Vec<(&'a [u8], &'a Tree)>) -> Self {
        let mut branches = HashMap::with_capacity(trees.len()); // min
        for (text, tree) in trees {
            let mut nodes = vec![tree.root_node()];
            while !nodes.is_empty() {
                let mut children = Vec::with_capacity(nodes.len()); // guesstimate
                for node in nodes {
                    branches
                        .entry(node.kind())
                        .or_insert_with(|| HashSet::with_capacity(1))
                        .insert(&text[node.byte_range()]);
                    let mut i = 0;
                    while let Some(child) = node.child(i) {
                        children.push(child);
                        i += 1;
                    }
                }
                nodes = children;
            }
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

fn parse(language: &Language, code: &str) -> Tree {
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
    fn delta(node: Node<'_>, replace: &[u8]) -> isize {
        let range = node.byte_range();
        isize::try_from(replace.len()).unwrap_or_default()
            - isize::try_from(range.end - range.start).unwrap_or_default()
    }

    #[must_use]
    pub fn new(config: Config, files: &'a HashMap<String, (Vec<u8>, Tree)>) -> Option<Self> {
        let trees: Vec<_> = files
            .iter()
            .map(|(_, (txt, tree))| (txt.as_ref(), tree))
            .collect();

        let mut all_empty = true;
        for (_bytes, tree) in files.values() {
            if tree.root_node().child_count() != 0 {
                all_empty = false;
                break;
            }
        }
        if all_empty {
            return None;
        }

        let branches = Branches::new(
            files
                .iter()
                .map(|(_, (txt, tree))| (txt.as_ref(), tree))
                .collect(),
        );
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

    fn pick_usize(&mut self, n: usize) -> usize {
        self.rng.random_range(0..n)
    }

    fn pick_idx<T>(&mut self, v: &[T]) -> usize {
        self.pick_usize(v.len())
    }

    fn all_nodes(tree: &Tree) -> Vec<Node<'_>> {
        let mut all = Vec::with_capacity(16); // min
        let root = tree.root_node();
        let mut cursor = tree.walk();
        let mut nodes: HashSet<_> = root.children(&mut cursor).collect();
        while !nodes.is_empty() {
            let mut next = HashSet::new();
            for node in nodes {
                debug_assert!(!next.contains(&node));
                all.push(node);
                let mut child_cursor = tree.walk();
                for child in node.children(&mut child_cursor) {
                    debug_assert!(child.id() != node.id());
                    debug_assert!(!next.contains(&child));
                    next.insert(child);
                }
            }
            nodes = next;
        }
        all
    }

    fn pick_node<'b>(&mut self, tree: &'b Tree) -> Node<'b> {
        let nodes = Self::all_nodes(tree);
        if nodes.is_empty() {
            return tree.root_node();
        }
        *nodes.get(self.pick_idx(nodes.as_slice())).unwrap()
    }

    fn delete_node(&mut self, _text: &[u8], tree: &Tree) -> (usize, Vec<u8>, isize) {
        let chaotic = self.rng.random_range(0..100) < self.chaos;
        if chaotic {
            let node = self.pick_node(tree);
            return (node.id(), Vec::new(), Self::delta(node, &[]));
        }
        let nodes = Self::all_nodes(tree);
        if nodes.iter().all(|n| !self.node_types.optional_node(n)) {
            let node = self.pick_node(tree);
            return (node.id(), Vec::new(), Self::delta(node, &[]));
        }
        let mut node = nodes.get(self.pick_idx(nodes.as_slice())).unwrap();
        while !self.node_types.optional_node(node) {
            node = nodes.get(self.pick_idx(nodes.as_slice())).unwrap();
        }
        (node.id(), Vec::new(), Self::delta(*node, &[]))
    }

    fn splice_node(&mut self, text: &[u8], tree: &Tree) -> (usize, Vec<u8>, isize) {
        let chaotic = self.rng.random_range(0..100) < self.chaos;

        let mut node = tree.root_node();
        let mut candidates = Vec::new();
        // When modified trees are re-parsed, their nodes may have novel kinds
        // not in Branches (candidates.len() == 0). Also, avoid not mutating
        // (candidates.len() == 1).
        while candidates.len() <= 1 {
            node = self.pick_node(tree);
            candidates = if chaotic {
                let kind_idx = self.rng.random_range(0..self.kinds.len());
                let kind = self.kinds.get(kind_idx).unwrap();
                self.branches.0.get(kind).unwrap().clone()
            } else {
                self.branches
                    .0
                    .get(node.kind())
                    .cloned()
                    .unwrap_or_default()
            };
        }

        let idx = self.rng.random_range(0..candidates.len());
        let mut candidate = candidates.get(idx).unwrap();
        // Try to avoid not mutating
        let node_text = &text[node.byte_range()];
        while candidates.len() > 1 && candidate == &node_text {
            let idx = self.rng.random_range(0..candidates.len());
            candidate = candidates.get(idx).unwrap();
        }
        // eprintln!(
        //     "Replacing '{}' with '{}'",
        //     std::str::from_utf8(&text[node.byte_range()]).unwrap(),
        //     std::str::from_utf8(candidate).unwrap(),
        // );
        let replace = Vec::from(*candidate);
        let delta = Self::delta(node, replace.as_slice());
        (node.id(), replace, delta)
    }

    pub fn splice_tree(&mut self, text0: &[u8], mut tree: Tree) -> Option<Vec<u8>> {
        // TODO: Assert that text0 and tree.root_node() are the same length?
        let mut edits = Edits::default();
        if self.inter_splices == 0 {
            return None;
        }
        let splices = self.rng.random_range(1..self.inter_splices);
        let mut text = Vec::from(text0);
        let mut sz = isize::try_from(text.len()).unwrap_or_default();
        for i in 0..splices {
            let (id, bytes, delta) = if self.rng.random_range(0..100) < self.deletions {
                self.delete_node(text.as_slice(), &tree)
            } else {
                self.splice_node(text.as_slice(), &tree)
            };
            sz += delta;
            let sized_out = usize::try_from(sz).unwrap_or_default() >= self.max_size;
            edits.0.insert(id, bytes);
            if i % self.reparse == 0 || i + 1 == splices || sized_out {
                let mut result = Vec::with_capacity(usize::try_from(sz).unwrap_or_default());
                tree_sitter_edit::render(&mut result, &tree, text.as_slice(), &edits).ok()?;
                text = result.clone();
                tree = parse(&self.language, &String::from_utf8_lossy(text.as_slice()));
                edits = Edits::default();
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
        let mut tree_idx: usize = self.pick_usize(self.trees.len());
        let (mut text, mut tree) = *self.trees.get(tree_idx).unwrap();
        while text.len() > self.max_size {
            tree_idx = self.pick_usize(self.trees.len());
            (text, tree) = *self.trees.get(tree_idx).unwrap();
        }
        self.splice_tree(text, tree.clone())
    }
}
