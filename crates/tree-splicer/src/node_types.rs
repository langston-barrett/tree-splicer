//! Read tree-sitter's `node-types.json`.
//
// Copied in part from [treeedbgen] and treereduce.
//
// [treeedbgen]: https://github.com/langston-barrett/treeedb/blob/1a2fae3509c76cd5a8e1004f808ea800d49d1a19/treeedbgen/src/lib.rs

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// node-types.json
#[derive(Clone, Eq, PartialEq, Serialize, Deserialize, Debug)]
struct Node {
    #[serde(rename(deserialize = "type", serialize = "type"))]
    ty: String,
    named: bool,
    #[serde(default)] // empty
    children: Children,
    #[serde(default)] // empty
    fields: HashMap<String, Field>,
    #[serde(default)] // empty
    subtypes: Vec<Subtype>,
}

#[derive(Default, Clone, Eq, PartialEq, Serialize, Deserialize, Debug)]
pub(crate) struct Children {
    multiple: bool,
    required: bool,
    pub(crate) types: Vec<Subtype>,
}

#[derive(Clone, Eq, PartialEq, Serialize, Deserialize, Debug)]
pub(crate) struct Field {
    multiple: bool,
    required: bool,
    pub(crate) types: Vec<Subtype>,
}

#[derive(Clone, Eq, PartialEq, Serialize, Deserialize, Debug)]
pub(crate) struct Subtype {
    #[serde(rename(deserialize = "type", serialize = "type"))]
    pub(crate) ty: String,
    named: bool,
}

#[derive(Clone, Debug)]
pub struct FieldInfo {
    parent_ty: String,
    multiple: bool,
    required: bool,
}

#[derive(Clone, Debug)]
pub struct NodeTypes {
    pub(crate) children: HashMap<&'static str, Children>,
    subtypes: HashMap<&'static str, Vec<String>>,
    pub(crate) fields: HashMap<&'static str, HashMap<String, Field>>,
    reverse_fields: HashMap<String, Vec<FieldInfo>>,
}

fn subtypes(name: &str, nodes: &Vec<Node>) -> Vec<String> {
    let mut r = vec![name.to_string()];
    for n in nodes {
        if n.ty == name {
            for subty in &n.subtypes {
                r.push(subty.ty.clone());
                r.extend(subtypes(&subty.ty, nodes));
            }
        }
    }
    r
}

impl NodeTypes {
    pub fn new(node_types_json_str: &'static str) -> Result<Self, serde_json::Error> {
        let find = |s: &str| {
            let idx = node_types_json_str.find(s).unwrap();
            &node_types_json_str[idx..idx + s.len()]
        };
        let nodes: Vec<Node> = serde_json::from_str(node_types_json_str)?;
        let children = nodes
            .iter()
            .filter(|n| n.named)
            .map(|n| (find(&n.ty), n.children.clone()))
            .collect();
        let subtypes: HashMap<_, _> = nodes
            .iter()
            .map(|n| (find(&n.ty), subtypes(&n.ty, &nodes)))
            .collect();
        let fields = nodes
            .iter()
            .map(|n| (find(&n.ty), n.fields.clone()))
            .collect();
        let mut reverse_fields = HashMap::new();

        // For each type of node...
        for node in &nodes {
            // Loop through it's fields...
            for field in node.fields.values() {
                // And save the name of all types that the field could be.
                for subtype in &field.types {
                    for subsubty in subtypes.get(subtype.ty.as_str()).unwrap_or(&Vec::new()) {
                        let entry = reverse_fields.entry(subsubty.clone());
                        entry
                            .and_modify(|v: &mut Vec<FieldInfo>| {
                                v.push(FieldInfo {
                                    parent_ty: node.ty.clone(),
                                    multiple: field.multiple,
                                    required: field.required,
                                });
                            })
                            .or_insert_with(|| {
                                vec![FieldInfo {
                                    parent_ty: node.ty.clone(),
                                    multiple: field.multiple,
                                    required: field.required,
                                }]
                            });
                    }
                }
            }
        }
        Ok(NodeTypes {
            children,
            subtypes,
            fields,
            reverse_fields,
        })
    }

    /// Defaults to `true` if the real answer can't be determined.
    fn optional(&self, node_kind: &str, parent_kind: &str) -> bool {
        if let Some(flds) = self.reverse_fields.get(node_kind) {
            for fi in flds {
                if parent_kind == fi.parent_ty && (!fi.multiple || fi.required) {
                    return false;
                }
            }
        }
        true
    }

    /// Defaults to `true` if the real answer can't be determined.
    #[must_use]
    pub fn optional_node(&self, node: &tree_sitter::Node<'_>) -> bool {
        if let Some(p) = node.parent() {
            self.optional(node.kind(), p.kind())
        } else {
            true
        }
    }

    // TODO(#21): Also include fields, include multiple and not required
    #[must_use]
    pub fn list_types(&self, node: &tree_sitter::Node<'_>) -> Vec<String> {
        let mut kinds = Vec::new();
        if let Some(children) = self.children.get(node.kind())
            && children.multiple
            && !children.required
        {
            for child in &children.types {
                kinds.push(child.ty.clone());
            }
        }
        kinds
    }

    /// # Panics
    /// When kind can't be found
    #[must_use]
    pub fn subtypes(&self, kind: &str) -> &[String] {
        self.subtypes.get(kind).expect("Invalid node kind")
    }

    /// Returns subtypes for a kind, or None if the kind doesn't exist
    #[must_use]
    pub fn get_subtypes(&self, kind: &str) -> Option<&[String]> {
        self.subtypes.get(kind).map(|v| v.as_slice())
    }
}
