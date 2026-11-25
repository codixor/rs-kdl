// SPDX-License-Identifier: MIT OR Apache-2.0
//! TOML text and TOML-on-KDL contract adapters.
//!
//! The neutral interop model is still the semantic contract. TOML parses into a
//! map-root value document, while the generated KDL contract exists for callers
//! that explicitly need a real [`crate::Document`].

use alloc::string::{String, ToString};
use alloc::vec::Vec;
use core::str::FromStr;

use rs_toml::{Arena, Item, Key, Table, ToTomlError};
use thiserror::Error;

use crate::Document;

use super::kdl::InteropError;
use super::model::{
    InteropDocument, InteropField, InteropMapEntry, InteropNode, InteropNumber, InteropRoot,
    InteropTemporal, InteropTemporalKind, InteropTypedValue, InteropValue,
};

const TOML_TABLE: &str = "toml-table";
const TOML_ENTRY: &str = "toml-entry";
const TOML_ARRAY: &str = "toml-array";
const TOML_STRING: &str = "toml-string";
const TOML_NUMBER: &str = "toml-number";
const TOML_BOOL: &str = "toml-bool";
const TOML_NULL: &str = "toml-null";
const TOML_DATETIME: &str = "toml-datetime";

/// Errors for TOML text and TOML-on-KDL contract conversion.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum TomlInteropError {
    #[error(transparent)]
    TomlParse(rs_toml::Error),
    #[error("TOML parse failed")]
    TomlParseFailed,
    #[error(transparent)]
    TomlEmit(ToTomlError),
    #[error("TOML emit failed")]
    TomlEmitFailed,
    #[error(transparent)]
    Kdl(#[from] InteropError),
    #[error("expected a value-root interop document for TOML conversion")]
    ExpectedValueRoot,
    #[error("expected a map-root interop document for TOML conversion")]
    ExpectedMapRoot,
    #[error("invalid TOML contract document: {0}")]
    InvalidContract(&'static str),
    #[error("TOML does not support null values")]
    NullUnsupported,
    #[error("TOML sequence length exceeds implementation limits")]
    SequenceTooLarge,
    #[error("TOML table entry count exceeds implementation limits")]
    TableTooLarge,
    #[error("invalid neutral TOML number literal: {0}")]
    InvalidNumber(String),
    #[error("invalid neutral TOML datetime literal: {0}")]
    InvalidDateTime(String),
}

impl From<rs_toml::Error> for TomlInteropError {
    fn from(_: rs_toml::Error) -> Self {
        Self::TomlParseFailed
    }
}

impl From<ToTomlError> for TomlInteropError {
    fn from(_: ToTomlError) -> Self {
        Self::TomlEmitFailed
    }
}

impl InteropDocument {
    /// Convert a parsed TOML document into a map-root neutral interop document.
    #[must_use]
    pub fn from_toml_document(document: &rs_toml::Document<'_>) -> Self {
        Self::from_toml_table(document.table())
    }

    /// Convert a parsed TOML table into a map-root neutral interop document.
    #[must_use]
    pub fn from_toml_table(table: &rs_toml::Table<'_>) -> Self {
        Self::value(InteropValue::Map(
            table
                .iter()
                .map(|(key, item)| InteropMapEntry {
                    name: key.name.into(),
                    value: InteropValue::from_toml_item(item),
                })
                .collect(),
        ))
    }

    /// Parse TOML text into a map-root neutral interop document.
    ///
    /// # Errors
    ///
    /// Returns [`TomlInteropError`] if the TOML text is invalid.
    pub fn from_toml_str(source: &str) -> Result<Self, TomlInteropError> {
        let arena = Arena::new();
        let document = rs_toml::parse(source, &arena)?;
        Ok(Self::from_toml_document(&document))
    }

    /// Convert a map-root interop document into a TOML table.
    ///
    /// # Errors
    ///
    /// Returns [`TomlInteropError`] if the interop document is not a map-root
    /// value document or if it contains TOML-incompatible values.
    pub fn to_toml_table<'a>(
        &self,
        arena: &'a Arena,
    ) -> Result<rs_toml::Table<'a>, TomlInteropError> {
        match &self.root {
            InteropRoot::Value(InteropValue::Map(entries)) => {
                toml_table_from_entries(entries, arena)
            }
            InteropRoot::Value(_) => Err(TomlInteropError::ExpectedMapRoot),
            InteropRoot::Nodes(_) => Err(TomlInteropError::ExpectedValueRoot),
        }
    }

    /// Serialize a map-root interop document into canonical TOML text.
    ///
    /// # Errors
    ///
    /// Returns [`TomlInteropError`] if the interop document is not a map-root
    /// value document or if it contains TOML-incompatible values.
    pub fn to_toml_string(&self) -> Result<String, TomlInteropError> {
        let arena = Arena::new();
        let table = self.to_toml_table(&arena)?;
        Ok(rs_toml::to_string(&table)?)
    }

    /// Encode a value-root interop document as a generated TOML-on-KDL
    /// contract document.
    ///
    /// # Errors
    ///
    /// Returns [`TomlInteropError::ExpectedValueRoot`] if the interop document
    /// is node-root instead of value-root.
    pub fn to_toml_contract_document(&self) -> Result<Document, TomlInteropError> {
        let contract = interop_to_toml_contract(self)?;
        Ok(Document::from_interop(&contract)?)
    }

    /// Decode a TOML-on-KDL contract document back into a value-root interop
    /// document.
    ///
    /// # Errors
    ///
    /// Returns [`TomlInteropError`] if the document does not follow the TOML
    /// contract shape.
    pub fn from_toml_contract_document(document: &Document) -> Result<Self, TomlInteropError> {
        toml_contract_to_interop(&document.to_interop())
    }
}

impl InteropValue {
    /// Convert a parsed TOML item into the neutral interop value tree.
    #[must_use]
    pub fn from_toml_item(item: &rs_toml::Item<'_>) -> Self {
        match item.value() {
            rs_toml::Value::String(text) => Self::String((*text).into()),
            rs_toml::Value::Integer(value) => {
                Self::Number(InteropNumber::new(value.as_i128().to_string()))
            }
            rs_toml::Value::Float(value) => {
                Self::Number(InteropNumber::new(render_toml_float(*value)))
            }
            rs_toml::Value::Boolean(value) => Self::Bool(*value),
            rs_toml::Value::DateTime(value) => Self::Temporal(interop_temporal_from_toml(value)),
            rs_toml::Value::Array(array) => {
                Self::Sequence(array.iter().map(Self::from_toml_item).collect())
            }
            rs_toml::Value::Table(table) => Self::Map(
                table
                    .iter()
                    .map(|(key, item)| InteropMapEntry {
                        name: key.name.into(),
                        value: Self::from_toml_item(item),
                    })
                    .collect(),
            ),
        }
    }
}

impl Document {
    /// Convert a parsed TOML document into the generated TOML-on-KDL contract
    /// document shape.
    ///
    /// # Errors
    ///
    /// Returns [`TomlInteropError`] if contract generation fails.
    pub fn from_toml_document(document: &rs_toml::Document<'_>) -> Result<Self, TomlInteropError> {
        InteropDocument::from_toml_document(document).to_toml_contract_document()
    }

    /// Convert a parsed TOML table into the generated TOML-on-KDL contract
    /// document shape.
    ///
    /// # Errors
    ///
    /// Returns [`TomlInteropError`] if contract generation fails.
    pub fn from_toml_table(table: &rs_toml::Table<'_>) -> Result<Self, TomlInteropError> {
        InteropDocument::from_toml_table(table).to_toml_contract_document()
    }

    /// Parse TOML text into the generated TOML-on-KDL contract document shape.
    ///
    /// # Errors
    ///
    /// Returns [`TomlInteropError`] if TOML parsing fails or if contract
    /// generation fails.
    pub fn from_toml_str(source: &str) -> Result<Self, TomlInteropError> {
        InteropDocument::from_toml_str(source)?.to_toml_contract_document()
    }

    /// Convert this TOML-on-KDL contract document back into neutral TOML
    /// interop.
    ///
    /// # Errors
    ///
    /// Returns [`TomlInteropError`] if the document does not match the TOML
    /// contract shape.
    pub fn to_toml_interop(&self) -> Result<InteropDocument, TomlInteropError> {
        InteropDocument::from_toml_contract_document(self)
    }

    /// Convert this TOML-on-KDL contract document back into canonical TOML
    /// text.
    ///
    /// # Errors
    ///
    /// Returns [`TomlInteropError`] if the document does not match the TOML
    /// contract shape or if it contains TOML-incompatible values.
    pub fn to_toml_string(&self) -> Result<String, TomlInteropError> {
        self.to_toml_interop()?.to_toml_string()
    }
}

fn interop_temporal_from_toml(value: &rs_toml::DateTime) -> InteropTemporal {
    let kind = match (value.date(), value.time(), value.offset()) {
        (Some(_), Some(_), Some(_)) => InteropTemporalKind::OffsetDateTime,
        (Some(_), None, None) => InteropTemporalKind::LocalDate,
        (None, Some(_), None) => InteropTemporalKind::LocalTime,
        _ => InteropTemporalKind::LocalDateTime,
    };

    let mut buffer = core::mem::MaybeUninit::uninit();
    InteropTemporal::new(kind, value.format(&mut buffer))
}

fn render_toml_float(value: f64) -> String {
    if value.is_nan() {
        return if value.is_sign_negative() {
            "-nan".to_owned()
        } else {
            "nan".to_owned()
        };
    }

    if value.is_infinite() {
        return if value.is_sign_negative() {
            "-inf".to_owned()
        } else {
            "inf".to_owned()
        };
    }

    let text = format!("{value:?}");
    if text.contains(['.', 'e', 'E']) {
        text
    } else {
        format!("{text}.0")
    }
}

fn toml_table_from_entries<'a>(
    entries: &[InteropMapEntry],
    arena: &'a Arena,
) -> Result<Table<'a>, TomlInteropError> {
    let Some(mut table) = Table::try_with_capacity(entries.len(), arena) else {
        return Err(TomlInteropError::TableTooLarge);
    };

    for entry in entries {
        let key = Key::new(arena.alloc_str(entry.name.as_str()));
        let value = toml_item_from_interop_value(&entry.value, arena)?;
        table.insert_unique(key, value, arena);
    }

    Ok(table)
}

fn toml_item_from_interop_value<'a>(
    value: &InteropValue,
    arena: &'a Arena,
) -> Result<Item<'a>, TomlInteropError> {
    match value {
        InteropValue::String(text) => Ok(Item::string(arena.alloc_str(text.as_str()))),
        InteropValue::Number(number) => toml_item_from_number(number),
        InteropValue::Bool(value) => Ok(Item::from(*value)),
        InteropValue::Null => Err(TomlInteropError::NullUnsupported),
        InteropValue::Temporal(temporal) => toml_item_from_temporal(temporal),
        InteropValue::Sequence(items) => {
            let Some(mut array) = rs_toml::Array::try_with_capacity(items.len(), arena) else {
                return Err(TomlInteropError::SequenceTooLarge);
            };
            for item in items {
                array.push(toml_item_from_interop_value(item, arena)?, arena);
            }
            Ok(array.into_item())
        }
        InteropValue::Map(entries) => toml_table_from_entries(entries, arena).map(Table::into_item),
    }
}

fn toml_item_from_number<'a>(number: &InteropNumber) -> Result<Item<'a>, TomlInteropError> {
    let text = number.as_str().replace('_', "");
    if is_toml_float_literal(&text) {
        let value =
            f64::from_str(&text).map_err(|_| TomlInteropError::InvalidNumber(String::new()))?;
        Ok(Item::from(value))
    } else {
        let value =
            i128::from_str(&text).map_err(|_| TomlInteropError::InvalidNumber(String::new()))?;
        Ok(Item::from(value))
    }
}

fn is_toml_float_literal(text: &str) -> bool {
    text.contains(['.', 'e', 'E'])
        || matches!(text, "inf" | "+inf" | "-inf" | "nan" | "+nan" | "-nan")
}

fn toml_item_from_temporal<'a>(temporal: &InteropTemporal) -> Result<Item<'a>, TomlInteropError> {
    let value = rs_toml::DateTime::from_str(temporal.as_str())
        .map_err(|_| TomlInteropError::InvalidDateTime(String::new()))?;
    Ok(Item::from(value))
}

fn interop_to_toml_contract(
    document: &InteropDocument,
) -> Result<InteropDocument, TomlInteropError> {
    match &document.root {
        InteropRoot::Value(value) => Ok(InteropDocument::nodes(vec![contract_node_from_value(
            value,
        )])),
        InteropRoot::Nodes(_) => Err(TomlInteropError::ExpectedValueRoot),
    }
}

fn toml_contract_to_interop(
    document: &InteropDocument,
) -> Result<InteropDocument, TomlInteropError> {
    match &document.root {
        InteropRoot::Nodes(nodes) => {
            if nodes.len() != 1 {
                return Err(TomlInteropError::InvalidContract(
                    "expected exactly one root node for the TOML contract",
                ));
            }
            Ok(InteropDocument::value(value_from_contract_node(&nodes[0])?))
        }
        InteropRoot::Value(_) => Err(TomlInteropError::InvalidContract(
            "expected a node-root KDL contract document",
        )),
    }
}

fn contract_node_from_value(value: &InteropValue) -> InteropNode {
    match value {
        InteropValue::String(text) => InteropNode {
            name: TOML_STRING.into(),
            type_hint: None,
            fields: vec![InteropField::Value(InteropTypedValue::new(
                InteropValue::String(text.clone()),
            ))],
            children: Vec::new(),
        },
        InteropValue::Number(number) => InteropNode {
            name: TOML_NUMBER.into(),
            type_hint: None,
            fields: vec![InteropField::Value(InteropTypedValue::new(
                InteropValue::Number(number.clone()),
            ))],
            children: Vec::new(),
        },
        InteropValue::Bool(value) => InteropNode {
            name: TOML_BOOL.into(),
            type_hint: None,
            fields: vec![InteropField::Value(InteropTypedValue::new(
                InteropValue::Bool(*value),
            ))],
            children: Vec::new(),
        },
        InteropValue::Null => InteropNode {
            name: TOML_NULL.into(),
            type_hint: None,
            fields: Vec::new(),
            children: Vec::new(),
        },
        InteropValue::Temporal(temporal) => InteropNode {
            name: TOML_DATETIME.into(),
            type_hint: None,
            fields: vec![InteropField::Value(InteropTypedValue::new(
                InteropValue::Temporal(temporal.clone()),
            ))],
            children: Vec::new(),
        },
        InteropValue::Sequence(items) => InteropNode {
            name: TOML_ARRAY.into(),
            type_hint: None,
            fields: Vec::new(),
            children: items.iter().map(contract_node_from_value).collect(),
        },
        InteropValue::Map(entries) => InteropNode {
            name: TOML_TABLE.into(),
            type_hint: None,
            fields: Vec::new(),
            children: entries
                .iter()
                .map(|entry| InteropNode {
                    name: TOML_ENTRY.into(),
                    type_hint: None,
                    fields: vec![InteropField::Value(InteropTypedValue::new(
                        InteropValue::String(entry.name.clone()),
                    ))],
                    children: vec![contract_node_from_value(&entry.value)],
                })
                .collect(),
        },
    }
}

fn value_from_contract_node(node: &InteropNode) -> Result<InteropValue, TomlInteropError> {
    match node.name.as_str() {
        TOML_STRING => {
            ensure_childless(node, TOML_STRING)?;
            let typed = single_value_field(node, TOML_STRING)?;
            match &typed.value {
                InteropValue::String(text) => Ok(InteropValue::String(text.clone())),
                _ => Err(TomlInteropError::InvalidContract(
                    "toml-string must contain a single string field",
                )),
            }
        }
        TOML_NUMBER => {
            ensure_childless(node, TOML_NUMBER)?;
            let typed = single_value_field(node, TOML_NUMBER)?;
            match &typed.value {
                InteropValue::Number(number) => Ok(InteropValue::Number(number.clone())),
                _ => Err(TomlInteropError::InvalidContract(
                    "toml-number must contain a single number field",
                )),
            }
        }
        TOML_BOOL => {
            ensure_childless(node, TOML_BOOL)?;
            let typed = single_value_field(node, TOML_BOOL)?;
            match typed.value {
                InteropValue::Bool(value) => Ok(InteropValue::Bool(value)),
                _ => Err(TomlInteropError::InvalidContract(
                    "toml-bool must contain a single bool field",
                )),
            }
        }
        TOML_NULL => {
            ensure_childless(node, TOML_NULL)?;
            if !node.fields.is_empty() {
                return Err(TomlInteropError::InvalidContract(
                    "toml-null must not contain scalar fields",
                ));
            }
            Ok(InteropValue::Null)
        }
        TOML_DATETIME => {
            ensure_childless(node, TOML_DATETIME)?;
            let typed = single_value_field(node, TOML_DATETIME)?;
            match &typed.value {
                InteropValue::Temporal(temporal) => Ok(InteropValue::Temporal(temporal.clone())),
                _ => Err(TomlInteropError::InvalidContract(
                    "toml-datetime must contain a single temporal field",
                )),
            }
        }
        TOML_ARRAY => {
            if !node.fields.is_empty() {
                return Err(TomlInteropError::InvalidContract(
                    "toml-array must not contain scalar fields",
                ));
            }
            node.children
                .iter()
                .map(value_from_contract_node)
                .collect::<Result<Vec<_>, _>>()
                .map(InteropValue::Sequence)
        }
        TOML_TABLE => {
            if !node.fields.is_empty() {
                return Err(TomlInteropError::InvalidContract(
                    "toml-table must not contain scalar fields",
                ));
            }
            node.children
                .iter()
                .map(map_entry_from_contract_node)
                .collect::<Result<Vec<_>, _>>()
                .map(InteropValue::Map)
        }
        _ => Err(TomlInteropError::InvalidContract(
            "unexpected node name in TOML contract document",
        )),
    }
}

fn map_entry_from_contract_node(node: &InteropNode) -> Result<InteropMapEntry, TomlInteropError> {
    if node.name.as_str() != TOML_ENTRY {
        return Err(TomlInteropError::InvalidContract(
            "toml-table children must be toml-entry nodes",
        ));
    }

    let typed = single_value_field(node, TOML_ENTRY)?;
    let key = match &typed.value {
        InteropValue::String(text) => text.clone(),
        _ => {
            return Err(TomlInteropError::InvalidContract(
                "toml-entry must carry a single string key field",
            ));
        }
    };

    if node.children.len() != 1 {
        return Err(TomlInteropError::InvalidContract(
            "toml-entry must contain exactly one child value node",
        ));
    }

    Ok(InteropMapEntry {
        name: key,
        value: value_from_contract_node(&node.children[0])?,
    })
}

fn single_value_field<'a>(
    node: &'a InteropNode,
    kind: &'static str,
) -> Result<&'a InteropTypedValue, TomlInteropError> {
    if node.fields.len() != 1 {
        return Err(TomlInteropError::InvalidContract(match kind {
            TOML_STRING => "toml-string must contain exactly one value field",
            TOML_NUMBER => "toml-number must contain exactly one value field",
            TOML_BOOL => "toml-bool must contain exactly one value field",
            TOML_DATETIME => "toml-datetime must contain exactly one value field",
            TOML_ENTRY => "toml-entry must contain exactly one key field",
            _ => "expected exactly one value field",
        }));
    }

    match &node.fields[0] {
        InteropField::Value(typed) => Ok(typed),
        InteropField::Property(_) => Err(TomlInteropError::InvalidContract(
            "TOML contract nodes must not use property fields",
        )),
    }
}

fn ensure_childless(node: &InteropNode, kind: &'static str) -> Result<(), TomlInteropError> {
    if node.children.is_empty() {
        Ok(())
    } else {
        Err(TomlInteropError::InvalidContract(match kind {
            TOML_STRING => "toml-string must not contain child nodes",
            TOML_NUMBER => "toml-number must not contain child nodes",
            TOML_BOOL => "toml-bool must not contain child nodes",
            TOML_DATETIME => "toml-datetime must not contain child nodes",
            TOML_NULL => "toml-null must not contain child nodes",
            _ => "contract node must not contain children",
        }))
    }
}
