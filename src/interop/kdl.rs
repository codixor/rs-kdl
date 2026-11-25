// SPDX-License-Identifier: MIT OR Apache-2.0
//! KDL <-> neutral model conversions.

use alloc::string::ToString;
use alloc::vec::Vec;
use core::str::FromStr;

use smol_str::SmolStr;
use thiserror::Error;

use crate::dom::Number;
use crate::{Document, Entry, Node, Value};

use super::model::{
    InteropDocument, InteropField, InteropMapEntry, InteropNode, InteropNumber, InteropProperty,
    InteropRoot, InteropTemporal, InteropTemporalKind, InteropTypedValue, InteropValue,
};

/// Error returned when the neutral interop model cannot be represented as KDL.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum InteropError {
    #[error("expected a node-root interop document, got a value-root document")]
    ExpectedNodeRoot,
    #[error("KDL entries cannot represent sequence or map values directly")]
    CompositeValueUnsupported,
    #[error("invalid neutral number literal for KDL")]
    InvalidNumber,
}

impl Document {
    /// Convert this KDL document into the neutral interop model.
    #[must_use]
    pub fn to_interop(&self) -> InteropDocument {
        self.into()
    }

    /// Convert a neutral interop document back into KDL.
    ///
    /// # Errors
    ///
    /// Returns [`InteropError`] when the neutral model cannot be represented as
    /// a KDL document, such as a value-root document or composite entry value.
    pub fn from_interop(model: &InteropDocument) -> Result<Self, InteropError> {
        Self::try_from(model)
    }
}

impl From<&Document> for InteropDocument {
    fn from(document: &Document) -> Self {
        Self::nodes(document.nodes.iter().map(InteropNode::from).collect())
    }
}

impl From<Document> for InteropDocument {
    fn from(document: Document) -> Self {
        Self::from(&document)
    }
}

impl TryFrom<&InteropDocument> for Document {
    type Error = InteropError;

    fn try_from(document: &InteropDocument) -> Result<Self, Self::Error> {
        match &document.root {
            InteropRoot::Nodes(nodes) => nodes
                .iter()
                .map(Node::try_from)
                .collect::<Result<Vec<_>, _>>()
                .map(Document::from),
            InteropRoot::Value(_) => Err(InteropError::ExpectedNodeRoot),
        }
    }
}

impl TryFrom<InteropDocument> for Document {
    type Error = InteropError;

    fn try_from(document: InteropDocument) -> Result<Self, Self::Error> {
        Self::try_from(&document)
    }
}

impl From<&Node> for InteropNode {
    fn from(node: &Node) -> Self {
        Self {
            name: node.name.clone(),
            type_hint: node.r#type.clone(),
            fields: node.entries.iter().map(InteropField::from).collect(),
            children: node
                .children
                .as_ref()
                .map_or_default(|children| children.nodes.iter().map(Self::from).collect()),
        }
    }
}

impl TryFrom<&InteropNode> for Node {
    type Error = InteropError;

    fn try_from(node: &InteropNode) -> Result<Self, Self::Error> {
        let children = (!node.children.is_empty())
            .then(|| {
                node.children
                    .iter()
                    .map(Self::try_from)
                    .collect::<Result<Vec<_>, _>>()
                    .map(Document::from)
            })
            .transpose()?;

        let entries = node
            .fields
            .iter()
            .map(Entry::try_from)
            .collect::<Result<Vec<_>, _>>()?;

        Ok(Self {
            r#type: node.type_hint.clone(),
            name: node.name.clone(),
            entries,
            children,
        })
    }
}

impl From<&Entry> for InteropField {
    fn from(entry: &Entry) -> Self {
        let typed = entry_to_interop_typed_value(entry);
        match &entry.name {
            Some(name) => Self::Property(InteropProperty {
                name: name.clone(),
                typed,
            }),
            None => Self::Value(typed),
        }
    }
}

impl TryFrom<&InteropField> for Entry {
    type Error = InteropError;

    fn try_from(field: &InteropField) -> Result<Self, Self::Error> {
        match field {
            InteropField::Value(typed) => {
                let (r#type, value) = kdl_entry_parts_from_typed_value(typed)?;
                Ok(Self {
                    name: None,
                    r#type,
                    value,
                })
            }
            InteropField::Property(property) => {
                let (r#type, value) = kdl_entry_parts_from_typed_value(&property.typed)?;
                Ok(Self {
                    name: Some(property.name.clone()),
                    r#type,
                    value,
                })
            }
        }
    }
}

impl From<&Value> for InteropValue {
    fn from(value: &Value) -> Self {
        match value {
            Value::String(text) => Self::String(text.clone()),
            Value::Number(number) => Self::Number(InteropNumber::new(number.to_string())),
            Value::Bool(value) => Self::Bool(*value),
            Value::Null => Self::Null,
        }
    }
}

impl TryFrom<&InteropValue> for Value {
    type Error = InteropError;

    fn try_from(value: &InteropValue) -> Result<Self, Self::Error> {
        match value {
            InteropValue::String(text) => Ok(Self::String(text.clone())),
            InteropValue::Number(number) => Number::from_str(number.as_str())
                .map(Self::Number)
                .map_err(|()| InteropError::InvalidNumber),
            InteropValue::Bool(value) => Ok(Self::Bool(*value)),
            InteropValue::Null => Ok(Self::Null),
            InteropValue::Temporal(temporal) => Ok(Self::String(temporal.text.clone())),
            InteropValue::Sequence(_) | InteropValue::Map(_) => {
                Err(InteropError::CompositeValueUnsupported)
            }
        }
    }
}

impl From<&Number> for InteropNumber {
    fn from(number: &Number) -> Self {
        Self::new(number.to_string())
    }
}

impl From<InteropValue> for InteropDocument {
    fn from(value: InteropValue) -> Self {
        Self::value(value)
    }
}

impl From<Vec<InteropNode>> for InteropDocument {
    fn from(nodes: Vec<InteropNode>) -> Self {
        Self::nodes(nodes)
    }
}

impl From<(SmolStr, InteropValue)> for InteropMapEntry {
    fn from((name, value): (SmolStr, InteropValue)) -> Self {
        Self { name, value }
    }
}

fn entry_to_interop_typed_value(entry: &Entry) -> InteropTypedValue {
    if let (Some(type_hint), Value::String(text)) = (&entry.r#type, &entry.value)
        && let Some(kind) = InteropTemporalKind::from_kdl_type_hint(type_hint.as_str())
    {
        return InteropTypedValue {
            type_hint: None,
            value: InteropValue::Temporal(InteropTemporal::new(kind, text.clone())),
        };
    }

    InteropTypedValue {
        type_hint: entry.r#type.clone(),
        value: InteropValue::from(&entry.value),
    }
}

fn kdl_entry_parts_from_typed_value(
    typed: &InteropTypedValue,
) -> Result<(Option<SmolStr>, Value), InteropError> {
    match &typed.value {
        InteropValue::Temporal(temporal) => Ok((
            Some(temporal.kind.kdl_type_hint()),
            Value::String(temporal.text.clone()),
        )),
        _ => Ok((typed.type_hint.clone(), Value::try_from(&typed.value)?)),
    }
}
