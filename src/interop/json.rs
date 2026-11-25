// SPDX-License-Identifier: MIT OR Apache-2.0
//! JSON text and JSON-on-KDL contract adapters.
//!
//! The neutral interop model is the semantic contract. JSON text parses into a
//! value-root [`InteropDocument`], while KDL uses an explicit generated node
//! contract so conversion stays lossless and benchmarkable.

use alloc::string::String;
use alloc::vec::Vec;
use core::fmt::Write as _;

use rs_json::{Error as JsonError, Value as JsonValue, ValueKind, parse};
use smol_str::SmolStr;
use thiserror::Error;

use crate::{AdapterDiagnostic, Category, Document, FormatError, Limits, Position};

use super::kdl::InteropError;
use super::model::{
    InteropDocument, InteropField, InteropMapEntry, InteropNode, InteropNumber, InteropRoot,
    InteropTypedValue, InteropValue,
};

const JSON_OBJECT: &str = "json-object";
const JSON_ARRAY: &str = "json-array";
const JSON_ENTRY: &str = "json-entry";
const JSON_STRING: &str = "json-string";
const JSON_NUMBER: &str = "json-number";
const JSON_BOOL: &str = "json-bool";
const JSON_NULL: &str = "json-null";

/// Errors for JSON text and JSON-on-KDL contract conversion.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum JsonInteropError {
    #[error(transparent)]
    Json(#[from] AdapterDiagnostic),
    #[error(transparent)]
    Kdl(#[from] InteropError),
    #[error("expected a value-root interop document for JSON text conversion")]
    ExpectedValueRoot,
    #[error("invalid JSON contract document: {0}")]
    InvalidContract(&'static str),
    #[error("invalid decoded JSON string: {0}")]
    InvalidString(&'static str),
}

impl From<JsonError> for JsonInteropError {
    fn from(error: JsonError) -> Self {
        Self::Json(AdapterDiagnostic::capture(&error))
    }
}

impl FormatError for JsonInteropError {
    fn category(&self) -> Category {
        match self {
            Self::Json(e) => e.category(),
            Self::Kdl(e) => e.category(),
            _ => Category::Data,
        }
    }
    fn position(&self) -> Option<Position> {
        match self {
            Self::Json(e) => e.position(),
            _ => None,
        }
    }
}

impl InteropDocument {
    /// Convert an already-parsed JSON DOM item into a value-root neutral
    /// interop document.
    ///
    /// This is the direct shared-model bridge for callers that already use the
    /// `json` crate and want to avoid routing through generated KDL contract
    /// documents unless they explicitly need them.
    ///
    /// # Errors
    ///
    /// Returns [`JsonInteropError`] if a string escape cannot be decoded into
    /// the neutral model.
    pub fn from_json_item(item: JsonValue<'_, '_>) -> Result<Self, JsonInteropError> {
        Ok(Self::value(InteropValue::from_json_item(item)?))
    }

    /// Parse JSON text into a value-root neutral interop document.
    ///
    /// # Errors
    ///
    /// Returns [`JsonInteropError`] if the JSON text is invalid or if a string
    /// escape cannot be decoded into the neutral model.
    pub fn from_json_str(source: &str) -> Result<Self, JsonInteropError> {
        Self::from_json_item(parse(source)?.root())
    }
    /// Parse JSON text with an explicit resource policy.
    ///
    /// # Errors
    /// Returns a diagnostic when the text fails parsing, limits, or conversion.
    pub fn from_json_str_with_limits(
        source: &str,
        limits: Limits,
    ) -> Result<Self, JsonInteropError> {
        Self::from_json_item(
            rs_json::parse_with_limits(source, crate::diagnostic::tree_limits(limits))?.root(),
        )
    }

    /// Serialize a value-root neutral interop document into canonical JSON text.
    ///
    /// # Errors
    ///
    /// Returns [`JsonInteropError::ExpectedValueRoot`] if the document is
    /// node-root instead of value-root.
    pub fn to_json_string(&self) -> Result<String, JsonInteropError> {
        match &self.root {
            InteropRoot::Value(value) => {
                let mut output = String::new();
                write_json_value(value, &mut output);
                Ok(output)
            }
            InteropRoot::Nodes(_) => Err(JsonInteropError::ExpectedValueRoot),
        }
    }

    /// Encode a value-root interop document as a KDL contract document.
    ///
    /// # Errors
    ///
    /// Returns [`JsonInteropError::ExpectedValueRoot`] if the interop document
    /// is node-root instead of value-root.
    pub fn to_json_contract_document(&self) -> Result<Document, JsonInteropError> {
        let contract = interop_to_json_contract(self)?;
        Ok(Document::from_interop(&contract)?)
    }

    /// Decode a KDL JSON contract document back into a value-root interop document.
    ///
    /// # Errors
    ///
    /// Returns [`JsonInteropError`] if the document does not follow the JSON
    /// contract shape.
    pub fn from_json_contract_document(document: &Document) -> Result<Self, JsonInteropError> {
        json_contract_to_interop(&document.to_interop())
    }
}

impl Document {
    /// Convert an already-parsed JSON DOM item into the generated KDL JSON
    /// contract document shape.
    ///
    /// # Errors
    ///
    /// Returns [`JsonInteropError`] if the JSON item cannot be represented by
    /// the neutral interop model.
    pub fn from_json_item(item: JsonValue<'_, '_>) -> Result<Self, JsonInteropError> {
        InteropDocument::from_json_item(item)?.to_json_contract_document()
    }

    /// Parse JSON text into the generated KDL JSON contract document shape.
    ///
    /// # Errors
    ///
    /// Returns [`JsonInteropError`] if JSON parsing fails or if contract
    /// generation fails.
    pub fn from_json_str(source: &str) -> Result<Self, JsonInteropError> {
        Self::from_json_item(parse(source)?.root())
    }
    /// Parse JSON text with an explicit resource policy.
    ///
    /// # Errors
    /// Returns a diagnostic when the text fails parsing, limits, or conversion.
    pub fn from_json_str_with_limits(
        source: &str,
        limits: Limits,
    ) -> Result<Self, JsonInteropError> {
        Self::from_json_item(
            rs_json::parse_with_limits(source, crate::diagnostic::tree_limits(limits))?.root(),
        )
    }

    /// Convert this KDL JSON contract document back into neutral JSON interop.
    ///
    /// # Errors
    ///
    /// Returns [`JsonInteropError`] if the document does not match the JSON
    /// contract shape.
    pub fn to_json_interop(&self) -> Result<InteropDocument, JsonInteropError> {
        InteropDocument::from_json_contract_document(self)
    }

    /// Convert this KDL JSON contract document back into canonical JSON text.
    ///
    /// # Errors
    ///
    /// Returns [`JsonInteropError`] if the document does not match the JSON
    /// contract shape.
    pub fn to_json_string(&self) -> Result<String, JsonInteropError> {
        self.to_json_interop()?.to_json_string()
    }
}

impl InteropValue {
    /// Convert an already-parsed JSON DOM item into the neutral interop value
    /// tree.
    ///
    /// # Errors
    ///
    /// Returns [`JsonInteropError`] if a JSON string escape cannot be decoded
    /// into the neutral model.
    pub fn from_json_item(item: JsonValue<'_, '_>) -> Result<Self, JsonInteropError> {
        interop_value_from_json_item(item)
    }
}

fn interop_value_from_json_item(item: JsonValue<'_, '_>) -> Result<InteropValue, JsonInteropError> {
    Ok(match item.kind() {
        ValueKind::String => {
            let text = item
                .as_str()
                .ok_or(JsonInteropError::InvalidString("invalid JSON string"))?;
            InteropValue::String(SmolStr::new(text))
        }
        ValueKind::Number => InteropValue::Number(InteropNumber::new(item.raw())),
        ValueKind::Bool => InteropValue::Bool(item.raw() == "true"),
        ValueKind::Null => InteropValue::Null,
        ValueKind::Array => InteropValue::Sequence(
            item.elements()
                .ok_or(JsonInteropError::InvalidContract("missing array elements"))?
                .map(interop_value_from_json_item)
                .collect::<Result<_, _>>()?,
        ),
        ValueKind::Object => InteropValue::Map(
            item.entries()
                .ok_or(JsonInteropError::InvalidContract("missing object entries"))?
                .map(|(key, value)| {
                    Ok(InteropMapEntry {
                        name: SmolStr::new(key.as_str()),
                        value: interop_value_from_json_item(value)?,
                    })
                })
                .collect::<Result<_, JsonInteropError>>()?,
        ),
    })
}

fn interop_to_json_contract(
    document: &InteropDocument,
) -> Result<InteropDocument, JsonInteropError> {
    match &document.root {
        InteropRoot::Value(value) => Ok(InteropDocument::nodes(vec![contract_node_from_value(
            value,
        )])),
        InteropRoot::Nodes(_) => Err(JsonInteropError::ExpectedValueRoot),
    }
}

fn json_contract_to_interop(
    document: &InteropDocument,
) -> Result<InteropDocument, JsonInteropError> {
    match &document.root {
        InteropRoot::Nodes(nodes) => {
            if nodes.len() != 1 {
                return Err(JsonInteropError::InvalidContract(
                    "expected exactly one root node for the JSON contract",
                ));
            }
            Ok(InteropDocument::value(value_from_contract_node(&nodes[0])?))
        }
        InteropRoot::Value(_) => Err(JsonInteropError::InvalidContract(
            "expected a node-root KDL contract document",
        )),
    }
}

fn contract_node_from_value(value: &InteropValue) -> InteropNode {
    match value {
        InteropValue::String(text) => InteropNode {
            name: JSON_STRING.into(),
            type_hint: None,
            fields: vec![InteropField::Value(InteropTypedValue::new(
                InteropValue::String(text.clone()),
            ))],
            children: Vec::new(),
        },
        InteropValue::Temporal(temporal) => InteropNode {
            name: JSON_STRING.into(),
            type_hint: None,
            fields: vec![InteropField::Value(InteropTypedValue::new(
                InteropValue::String(temporal.text.clone()),
            ))],
            children: Vec::new(),
        },
        InteropValue::Number(number) => InteropNode {
            name: JSON_NUMBER.into(),
            type_hint: None,
            fields: vec![InteropField::Value(InteropTypedValue::new(
                InteropValue::Number(number.clone()),
            ))],
            children: Vec::new(),
        },
        InteropValue::Bool(value) => InteropNode {
            name: JSON_BOOL.into(),
            type_hint: None,
            fields: vec![InteropField::Value(InteropTypedValue::new(
                InteropValue::Bool(*value),
            ))],
            children: Vec::new(),
        },
        InteropValue::Null => InteropNode {
            name: JSON_NULL.into(),
            type_hint: None,
            fields: Vec::new(),
            children: Vec::new(),
        },
        InteropValue::Sequence(items) => InteropNode {
            name: JSON_ARRAY.into(),
            type_hint: None,
            fields: Vec::new(),
            children: items.iter().map(contract_node_from_value).collect(),
        },
        InteropValue::Map(entries) => InteropNode {
            name: JSON_OBJECT.into(),
            type_hint: None,
            fields: Vec::new(),
            children: entries
                .iter()
                .map(|entry| InteropNode {
                    name: JSON_ENTRY.into(),
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

fn value_from_contract_node(node: &InteropNode) -> Result<InteropValue, JsonInteropError> {
    match node.name.as_str() {
        JSON_STRING => {
            ensure_childless(node, JSON_STRING)?;
            let typed = single_value_field(node, JSON_STRING)?;
            match &typed.value {
                InteropValue::String(text) => Ok(InteropValue::String(text.clone())),
                _ => Err(JsonInteropError::InvalidContract(
                    "json-string must contain a single string field",
                )),
            }
        }
        JSON_NUMBER => {
            ensure_childless(node, JSON_NUMBER)?;
            let typed = single_value_field(node, JSON_NUMBER)?;
            match &typed.value {
                InteropValue::Number(number) => Ok(InteropValue::Number(number.clone())),
                _ => Err(JsonInteropError::InvalidContract(
                    "json-number must contain a single number field",
                )),
            }
        }
        JSON_BOOL => {
            ensure_childless(node, JSON_BOOL)?;
            let typed = single_value_field(node, JSON_BOOL)?;
            match typed.value {
                InteropValue::Bool(value) => Ok(InteropValue::Bool(value)),
                _ => Err(JsonInteropError::InvalidContract(
                    "json-bool must contain a single bool field",
                )),
            }
        }
        JSON_NULL => {
            if !node.fields.is_empty() || !node.children.is_empty() {
                return Err(JsonInteropError::InvalidContract(
                    "json-null must not contain fields or children",
                ));
            }
            Ok(InteropValue::Null)
        }
        JSON_ARRAY => {
            if !node.fields.is_empty() {
                return Err(JsonInteropError::InvalidContract(
                    "json-array must not contain scalar fields",
                ));
            }
            node.children
                .iter()
                .map(value_from_contract_node)
                .collect::<Result<Vec<_>, _>>()
                .map(InteropValue::Sequence)
        }
        JSON_OBJECT => {
            if !node.fields.is_empty() {
                return Err(JsonInteropError::InvalidContract(
                    "json-object must not contain scalar fields",
                ));
            }
            node.children
                .iter()
                .map(map_entry_from_contract_node)
                .collect::<Result<Vec<_>, _>>()
                .map(InteropValue::Map)
        }
        _ => Err(JsonInteropError::InvalidContract(
            "unexpected node name in JSON contract document",
        )),
    }
}

fn map_entry_from_contract_node(node: &InteropNode) -> Result<InteropMapEntry, JsonInteropError> {
    if node.name.as_str() != JSON_ENTRY {
        return Err(JsonInteropError::InvalidContract(
            "json-object children must be json-entry nodes",
        ));
    }

    let typed = single_value_field(node, JSON_ENTRY)?;
    let key = match &typed.value {
        InteropValue::String(text) => text.clone(),
        _ => {
            return Err(JsonInteropError::InvalidContract(
                "json-entry must carry a single string key field",
            ));
        }
    };

    if node.children.len() != 1 {
        return Err(JsonInteropError::InvalidContract(
            "json-entry must contain exactly one child value node",
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
) -> Result<&'a InteropTypedValue, JsonInteropError> {
    if node.fields.len() != 1 {
        return Err(JsonInteropError::InvalidContract(match kind {
            JSON_STRING => "json-string must contain exactly one value field",
            JSON_NUMBER => "json-number must contain exactly one value field",
            JSON_BOOL => "json-bool must contain exactly one value field",
            JSON_ENTRY => "json-entry must contain exactly one key field",
            _ => "expected exactly one value field",
        }));
    }

    match &node.fields[0] {
        InteropField::Value(typed) => Ok(typed),
        InteropField::Property(_) => Err(JsonInteropError::InvalidContract(
            "JSON contract nodes must not use property fields",
        )),
    }
}

fn ensure_childless(node: &InteropNode, kind: &'static str) -> Result<(), JsonInteropError> {
    if node.children.is_empty() {
        Ok(())
    } else {
        Err(JsonInteropError::InvalidContract(match kind {
            JSON_STRING => "json-string must not contain child nodes",
            JSON_NUMBER => "json-number must not contain child nodes",
            JSON_BOOL => "json-bool must not contain child nodes",
            _ => "contract node must not contain children",
        }))
    }
}

fn write_json_value(value: &InteropValue, output: &mut String) {
    match value {
        InteropValue::String(text) => write_json_string(text, output),
        InteropValue::Temporal(temporal) => write_json_string(&temporal.text, output),
        InteropValue::Number(number) => output.push_str(number.as_str()),
        InteropValue::Bool(true) => output.push_str("true"),
        InteropValue::Bool(false) => output.push_str("false"),
        InteropValue::Null => output.push_str("null"),
        InteropValue::Sequence(items) => {
            output.push('[');
            for (index, item) in items.iter().enumerate() {
                if index > 0 {
                    output.push(',');
                }
                write_json_value(item, output);
            }
            output.push(']');
        }
        InteropValue::Map(entries) => {
            output.push('{');
            for (index, entry) in entries.iter().enumerate() {
                if index > 0 {
                    output.push(',');
                }
                write_json_string(&entry.name, output);
                output.push(':');
                write_json_value(&entry.value, output);
            }
            output.push('}');
        }
    }
}

fn write_json_string(text: &str, output: &mut String) {
    output.push('"');
    for ch in text.chars() {
        match ch {
            '"' => output.push_str("\\\""),
            '\\' => output.push_str("\\\\"),
            '\u{08}' => output.push_str("\\b"),
            '\u{0C}' => output.push_str("\\f"),
            '\n' => output.push_str("\\n"),
            '\r' => output.push_str("\\r"),
            '\t' => output.push_str("\\t"),
            ch if ch.is_control() => {
                write!(output, "\\u{:04X}", ch as u32).expect("string write should not fail");
            }
            _ => output.push(ch),
        }
    }
    output.push('"');
}
