// SPDX-License-Identifier: MIT OR Apache-2.0
//! YAML text and YAML-on-KDL contract adapters.
//!
//! The neutral interop model remains the semantic contract. YAML parses into a
//! value-root [`InteropDocument`], while the generated KDL contract exists only
//! for workflows that explicitly need a real KDL [`crate::Document`].

use alloc::string::{String, ToString};
use alloc::vec::Vec;
use core::fmt::Write as _;

use rs_yaml::{EmitError, Emitter, ParseError, ScalarOwned, Value};
use thiserror::Error;

use crate::Document;

use super::kdl::InteropError;
use super::model::{
    InteropDocument, InteropField, InteropMapEntry, InteropNode, InteropNumber, InteropRoot,
    InteropTypedValue, InteropValue,
};

const YAML_MAP: &str = "yaml-map";
const YAML_ENTRY: &str = "yaml-entry";
const YAML_SEQ: &str = "yaml-seq";
const YAML_STRING: &str = "yaml-string";
const YAML_NUMBER: &str = "yaml-number";
const YAML_BOOL: &str = "yaml-bool";
const YAML_NULL: &str = "yaml-null";

/// Errors for YAML text and YAML-on-KDL contract conversion.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum YamlInteropError {
    #[error(transparent)]
    YamlParse(ParseError),
    #[error("YAML parse failed")]
    YamlParseFailed,
    #[error(transparent)]
    YamlEmit(EmitError),
    #[error("YAML emit failed")]
    YamlEmitFailed,
    #[error(transparent)]
    Kdl(#[from] InteropError),
    #[error("expected a value-root interop document for YAML conversion")]
    ExpectedValueRoot,
    #[error("invalid YAML contract document: {0}")]
    InvalidContract(&'static str),
    #[error("YAML interop only supports string map keys, found {0}")]
    UnsupportedMapKey(&'static str),
    #[error("YAML interop does not support {0} in the neutral bridge")]
    UnsupportedYamlValue(&'static str),
    #[error("invalid neutral YAML number literal: {0}")]
    InvalidNumber(String),
}

impl From<ParseError> for YamlInteropError {
    fn from(_: ParseError) -> Self {
        Self::YamlParseFailed
    }
}

impl From<EmitError> for YamlInteropError {
    fn from(_: EmitError) -> Self {
        Self::YamlEmitFailed
    }
}

impl InteropDocument {
    /// Convert an already-parsed YAML value tree into a value-root neutral
    /// interop document.
    ///
    /// # Errors
    ///
    /// Returns [`YamlInteropError`] when the YAML value uses features that the
    /// current neutral bridge cannot represent losslessly, such as non-string
    /// map keys, aliases, or explicit non-core tags.
    pub fn from_yaml_value(value: &Value) -> Result<Self, YamlInteropError> {
        Ok(Self::value(InteropValue::from_yaml_value(value)?))
    }

    /// Parse YAML text into a value-root neutral interop document.
    ///
    /// # Errors
    ///
    /// Returns [`YamlInteropError`] if parsing fails or if the parsed YAML uses
    /// unsupported bridge features.
    pub fn from_yaml_str(source: &str) -> Result<Self, YamlInteropError> {
        Self::from_yaml_value(&rs_yaml::safe_load(source)?)
    }

    /// Convert a value-root neutral interop document into a YAML value tree.
    ///
    /// # Errors
    ///
    /// Returns [`YamlInteropError::ExpectedValueRoot`] if the document is
    /// node-root instead of value-root.
    pub fn to_yaml_value(&self) -> Result<Value, YamlInteropError> {
        match &self.root {
            InteropRoot::Value(value) => interop_to_yaml_value(value),
            InteropRoot::Nodes(_) => Err(YamlInteropError::ExpectedValueRoot),
        }
    }

    /// Serialize a value-root neutral interop document into YAML text.
    ///
    /// # Errors
    ///
    /// Returns [`YamlInteropError`] if the value-root document cannot be
    /// represented as YAML.
    pub fn to_yaml_string(&self) -> Result<String, YamlInteropError> {
        match &self.root {
            InteropRoot::Value(value) => render_yaml_text(value),
            InteropRoot::Nodes(_) => Err(YamlInteropError::ExpectedValueRoot),
        }
    }

    /// Encode a value-root interop document as a generated YAML-on-KDL
    /// contract document.
    ///
    /// # Errors
    ///
    /// Returns [`YamlInteropError::ExpectedValueRoot`] if the interop document
    /// is node-root instead of value-root.
    pub fn to_yaml_contract_document(&self) -> Result<Document, YamlInteropError> {
        let contract = interop_to_yaml_contract(self)?;
        Ok(Document::from_interop(&contract)?)
    }

    /// Decode a YAML-on-KDL contract document back into a value-root interop
    /// document.
    ///
    /// # Errors
    ///
    /// Returns [`YamlInteropError`] if the document does not follow the YAML
    /// contract shape.
    pub fn from_yaml_contract_document(document: &Document) -> Result<Self, YamlInteropError> {
        yaml_contract_to_interop(&document.to_interop())
    }
}

impl InteropValue {
    /// Convert an already-parsed YAML value tree into the neutral interop value
    /// tree.
    ///
    /// # Errors
    ///
    /// Returns [`YamlInteropError`] when the YAML value uses features that the
    /// neutral bridge cannot represent losslessly.
    pub fn from_yaml_value(value: &Value) -> Result<Self, YamlInteropError> {
        yaml_value_to_interop(value)
    }
}

impl Document {
    /// Convert an already-parsed YAML value tree into the generated YAML-on-KDL
    /// contract document shape.
    ///
    /// # Errors
    ///
    /// Returns [`YamlInteropError`] if the YAML value cannot be represented by
    /// the neutral interop model.
    pub fn from_yaml_value(value: &Value) -> Result<Self, YamlInteropError> {
        InteropDocument::from_yaml_value(value)?.to_yaml_contract_document()
    }

    /// Parse YAML text into the generated YAML-on-KDL contract document shape.
    ///
    /// # Errors
    ///
    /// Returns [`YamlInteropError`] if YAML parsing fails or if contract
    /// generation fails.
    pub fn from_yaml_str(source: &str) -> Result<Self, YamlInteropError> {
        Self::from_yaml_value(&rs_yaml::safe_load(source)?)
    }

    /// Convert this YAML-on-KDL contract document back into neutral YAML
    /// interop.
    ///
    /// # Errors
    ///
    /// Returns [`YamlInteropError`] if the document does not match the YAML
    /// contract shape.
    pub fn to_yaml_interop(&self) -> Result<InteropDocument, YamlInteropError> {
        InteropDocument::from_yaml_contract_document(self)
    }

    /// Convert this YAML-on-KDL contract document back into YAML text.
    ///
    /// # Errors
    ///
    /// Returns [`YamlInteropError`] if the document does not match the YAML
    /// contract shape.
    pub fn to_yaml_string(&self) -> Result<String, YamlInteropError> {
        self.to_yaml_interop()?.to_yaml_string()
    }
}

fn yaml_value_to_interop(value: &Value) -> Result<InteropValue, YamlInteropError> {
    Ok(match value {
        Value::Value(ScalarOwned::Null) => InteropValue::Null,
        Value::Value(ScalarOwned::Boolean(value)) => InteropValue::Bool(*value),
        Value::Value(ScalarOwned::Integer(value)) => {
            InteropValue::Number(InteropNumber::new(value.to_string()))
        }
        Value::Value(ScalarOwned::FloatingPoint(value)) => {
            InteropValue::Number(InteropNumber::new(render_yaml_float(value.0)))
        }
        Value::Value(ScalarOwned::String(text)) | Value::Representation(text, _, None) => {
            InteropValue::String(text.clone().into())
        }
        Value::Sequence(items) => InteropValue::Sequence(
            items
                .iter()
                .map(yaml_value_to_interop)
                .collect::<Result<Vec<_>, _>>()?,
        ),
        Value::Mapping(map) => InteropValue::Map(
            map.iter()
                .map(
                    |(key, value)| -> Result<InteropMapEntry, YamlInteropError> {
                        Ok(InteropMapEntry {
                            name: yaml_map_key_to_string(key)?.into(),
                            value: yaml_value_to_interop(value)?,
                        })
                    },
                )
                .collect::<Result<Vec<_>, _>>()?,
        ),
        Value::Representation(_, _, Some(_)) => {
            return Err(YamlInteropError::UnsupportedYamlValue(
                "explicitly tagged scalar representation",
            ));
        }
        Value::Tagged(_, _) => {
            return Err(YamlInteropError::UnsupportedYamlValue("tagged value"));
        }
        Value::Alias(_) => {
            return Err(YamlInteropError::UnsupportedYamlValue("alias"));
        }
        Value::BadValue => {
            return Err(YamlInteropError::UnsupportedYamlValue("bad value"));
        }
    })
}

fn interop_to_yaml_value(value: &InteropValue) -> Result<Value, YamlInteropError> {
    Ok(match value {
        InteropValue::String(text) => Value::Value(ScalarOwned::String(text.to_string())),
        InteropValue::Temporal(temporal) => {
            Value::Value(ScalarOwned::String(temporal.text.to_string()))
        }
        InteropValue::Number(number) => yaml_number_to_value(number)?,
        InteropValue::Bool(value) => Value::Value(ScalarOwned::Boolean(*value)),
        InteropValue::Null => Value::Value(ScalarOwned::Null),
        InteropValue::Sequence(items) => Value::Sequence(
            items
                .iter()
                .map(interop_to_yaml_value)
                .collect::<Result<Vec<_>, _>>()?,
        ),
        InteropValue::Map(entries) => {
            let mut map = rs_yaml::Map::new();
            for entry in entries {
                map.insert(
                    Value::Value(ScalarOwned::String(entry.name.to_string())),
                    interop_to_yaml_value(&entry.value)?,
                );
            }
            Value::Mapping(map)
        }
    })
}

fn yaml_number_to_value(number: &InteropNumber) -> Result<Value, YamlInteropError> {
    let parsed = rs_yaml::safe_load(number.as_str())?;
    match parsed {
        Value::Value(ScalarOwned::Integer(_) | ScalarOwned::FloatingPoint(_)) => Ok(parsed),
        _ => Err(YamlInteropError::InvalidNumber(String::new())),
    }
}

fn yaml_map_key_to_string(key: &Value) -> Result<&str, YamlInteropError> {
    match key {
        Value::Value(ScalarOwned::String(text)) | Value::Representation(text, _, None) => {
            Ok(text.as_str())
        }
        Value::Value(ScalarOwned::Null) => Err(YamlInteropError::UnsupportedMapKey("null scalar")),
        Value::Value(ScalarOwned::Boolean(_)) => {
            Err(YamlInteropError::UnsupportedMapKey("boolean scalar"))
        }
        Value::Value(ScalarOwned::Integer(_)) => {
            Err(YamlInteropError::UnsupportedMapKey("integer scalar"))
        }
        Value::Value(ScalarOwned::FloatingPoint(_)) => {
            Err(YamlInteropError::UnsupportedMapKey("floating-point scalar"))
        }
        Value::Representation(_, _, Some(_)) => Err(YamlInteropError::UnsupportedMapKey(
            "tagged scalar representation",
        )),
        Value::Sequence(_) => Err(YamlInteropError::UnsupportedMapKey("sequence")),
        Value::Mapping(_) => Err(YamlInteropError::UnsupportedMapKey("mapping")),
        Value::Tagged(_, _) => Err(YamlInteropError::UnsupportedMapKey("tagged value")),
        Value::Alias(_) => Err(YamlInteropError::UnsupportedMapKey("alias")),
        Value::BadValue => Err(YamlInteropError::UnsupportedMapKey("bad value")),
    }
}

fn render_yaml_text(value: &InteropValue) -> Result<String, YamlInteropError> {
    let mut output = String::new();
    write_yaml_value(value, &mut output, 0)?;
    if !output.ends_with('\n') {
        output.push('\n');
    }
    Ok(output)
}

fn write_yaml_value(
    value: &InteropValue,
    output: &mut String,
    indent: usize,
) -> Result<(), YamlInteropError> {
    match value {
        InteropValue::String(_)
        | InteropValue::Temporal(_)
        | InteropValue::Number(_)
        | InteropValue::Bool(_)
        | InteropValue::Null => {
            write_indent(output, indent);
            output.push_str(&render_yaml_scalar(value)?);
            output.push('\n');
        }
        InteropValue::Sequence(items) => {
            if items.is_empty() {
                write_indent(output, indent);
                output.push_str("[]\n");
                return Ok(());
            }
            for item in items {
                write_indent(output, indent);
                match item {
                    InteropValue::String(_)
                    | InteropValue::Temporal(_)
                    | InteropValue::Number(_)
                    | InteropValue::Bool(_)
                    | InteropValue::Null => {
                        output.push_str("- ");
                        output.push_str(&render_yaml_scalar(item)?);
                        output.push('\n');
                    }
                    InteropValue::Sequence(_) | InteropValue::Map(_) => {
                        output.push_str("-\n");
                        write_yaml_value(item, output, indent + 2)?;
                    }
                }
            }
        }
        InteropValue::Map(entries) => {
            if entries.is_empty() {
                write_indent(output, indent);
                output.push_str("{}\n");
                return Ok(());
            }
            for entry in entries {
                write_indent(output, indent);
                output.push_str(&render_yaml_string(entry.name.as_str())?);
                output.push(':');
                match &entry.value {
                    InteropValue::String(_)
                    | InteropValue::Temporal(_)
                    | InteropValue::Number(_)
                    | InteropValue::Bool(_)
                    | InteropValue::Null => {
                        output.push(' ');
                        output.push_str(&render_yaml_scalar(&entry.value)?);
                        output.push('\n');
                    }
                    InteropValue::Sequence(_) | InteropValue::Map(_) => {
                        output.push('\n');
                        write_yaml_value(&entry.value, output, indent + 2)?;
                    }
                }
            }
        }
    }
    Ok(())
}

fn render_yaml_scalar(value: &InteropValue) -> Result<String, YamlInteropError> {
    match value {
        InteropValue::String(text) => render_yaml_string(text),
        InteropValue::Temporal(temporal) => render_yaml_string(&temporal.text),
        InteropValue::Number(number) => Ok(number.as_str().to_owned()),
        InteropValue::Bool(value) => Ok(if *value { "true" } else { "false" }.to_owned()),
        InteropValue::Null => Ok("null".to_owned()),
        InteropValue::Sequence(_) | InteropValue::Map(_) => Err(YamlInteropError::InvalidContract(
            "composite values cannot be rendered as inline YAML scalars",
        )),
    }
}

fn render_yaml_string(text: &str) -> Result<String, YamlInteropError> {
    let rendered = Emitter::emit_str(&Value::Value(ScalarOwned::String(text.to_owned())))?;
    Ok(rendered.trim_end_matches('\n').to_owned())
}

fn write_indent(output: &mut String, indent: usize) {
    let _ = write!(output, "{:indent$}", "", indent = indent);
}

fn render_yaml_float(value: f64) -> String {
    if value.is_nan() {
        ".nan".to_owned()
    } else if value == f64::INFINITY {
        ".inf".to_owned()
    } else if value == f64::NEG_INFINITY {
        "-.inf".to_owned()
    } else {
        format!("{value:?}")
    }
}

fn interop_to_yaml_contract(
    document: &InteropDocument,
) -> Result<InteropDocument, YamlInteropError> {
    match &document.root {
        InteropRoot::Value(value) => Ok(InteropDocument::nodes(vec![contract_node_from_value(
            value,
        )])),
        InteropRoot::Nodes(_) => Err(YamlInteropError::ExpectedValueRoot),
    }
}

fn yaml_contract_to_interop(
    document: &InteropDocument,
) -> Result<InteropDocument, YamlInteropError> {
    match &document.root {
        InteropRoot::Nodes(nodes) => {
            if nodes.len() != 1 {
                return Err(YamlInteropError::InvalidContract(
                    "expected exactly one root node for the YAML contract",
                ));
            }
            Ok(InteropDocument::value(value_from_contract_node(&nodes[0])?))
        }
        InteropRoot::Value(_) => Err(YamlInteropError::InvalidContract(
            "expected a node-root KDL contract document",
        )),
    }
}

fn contract_node_from_value(value: &InteropValue) -> InteropNode {
    match value {
        InteropValue::String(text) => InteropNode {
            name: YAML_STRING.into(),
            type_hint: None,
            fields: vec![InteropField::Value(InteropTypedValue::new(
                InteropValue::String(text.clone()),
            ))],
            children: Vec::new(),
        },
        InteropValue::Temporal(temporal) => InteropNode {
            name: YAML_STRING.into(),
            type_hint: None,
            fields: vec![InteropField::Value(InteropTypedValue::new(
                InteropValue::String(temporal.text.clone()),
            ))],
            children: Vec::new(),
        },
        InteropValue::Number(number) => InteropNode {
            name: YAML_NUMBER.into(),
            type_hint: None,
            fields: vec![InteropField::Value(InteropTypedValue::new(
                InteropValue::Number(number.clone()),
            ))],
            children: Vec::new(),
        },
        InteropValue::Bool(value) => InteropNode {
            name: YAML_BOOL.into(),
            type_hint: None,
            fields: vec![InteropField::Value(InteropTypedValue::new(
                InteropValue::Bool(*value),
            ))],
            children: Vec::new(),
        },
        InteropValue::Null => InteropNode {
            name: YAML_NULL.into(),
            type_hint: None,
            fields: Vec::new(),
            children: Vec::new(),
        },
        InteropValue::Sequence(items) => InteropNode {
            name: YAML_SEQ.into(),
            type_hint: None,
            fields: Vec::new(),
            children: items.iter().map(contract_node_from_value).collect(),
        },
        InteropValue::Map(entries) => InteropNode {
            name: YAML_MAP.into(),
            type_hint: None,
            fields: Vec::new(),
            children: entries
                .iter()
                .map(|entry| InteropNode {
                    name: YAML_ENTRY.into(),
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

fn value_from_contract_node(node: &InteropNode) -> Result<InteropValue, YamlInteropError> {
    match node.name.as_str() {
        YAML_STRING => {
            ensure_childless(node, YAML_STRING)?;
            let typed = single_value_field(node, YAML_STRING)?;
            match &typed.value {
                InteropValue::String(text) => Ok(InteropValue::String(text.clone())),
                _ => Err(YamlInteropError::InvalidContract(
                    "yaml-string must contain a single string field",
                )),
            }
        }
        YAML_NUMBER => {
            ensure_childless(node, YAML_NUMBER)?;
            let typed = single_value_field(node, YAML_NUMBER)?;
            match &typed.value {
                InteropValue::Number(number) => Ok(InteropValue::Number(number.clone())),
                _ => Err(YamlInteropError::InvalidContract(
                    "yaml-number must contain a single number field",
                )),
            }
        }
        YAML_BOOL => {
            ensure_childless(node, YAML_BOOL)?;
            let typed = single_value_field(node, YAML_BOOL)?;
            match typed.value {
                InteropValue::Bool(value) => Ok(InteropValue::Bool(value)),
                _ => Err(YamlInteropError::InvalidContract(
                    "yaml-bool must contain a single bool field",
                )),
            }
        }
        YAML_NULL => {
            if !node.fields.is_empty() || !node.children.is_empty() {
                return Err(YamlInteropError::InvalidContract(
                    "yaml-null must not contain fields or children",
                ));
            }
            Ok(InteropValue::Null)
        }
        YAML_SEQ => {
            if !node.fields.is_empty() {
                return Err(YamlInteropError::InvalidContract(
                    "yaml-seq must not contain scalar fields",
                ));
            }
            node.children
                .iter()
                .map(value_from_contract_node)
                .collect::<Result<Vec<_>, _>>()
                .map(InteropValue::Sequence)
        }
        YAML_MAP => {
            if !node.fields.is_empty() {
                return Err(YamlInteropError::InvalidContract(
                    "yaml-map must not contain scalar fields",
                ));
            }
            node.children
                .iter()
                .map(map_entry_from_contract_node)
                .collect::<Result<Vec<_>, _>>()
                .map(InteropValue::Map)
        }
        _ => Err(YamlInteropError::InvalidContract(
            "unexpected node name in YAML contract document",
        )),
    }
}

fn map_entry_from_contract_node(node: &InteropNode) -> Result<InteropMapEntry, YamlInteropError> {
    if node.name.as_str() != YAML_ENTRY {
        return Err(YamlInteropError::InvalidContract(
            "yaml-map children must be yaml-entry nodes",
        ));
    }

    let key = single_value_field(node, YAML_ENTRY)?;
    let name = match &key.value {
        InteropValue::String(text) => text.clone(),
        _ => {
            return Err(YamlInteropError::InvalidContract(
                "yaml-entry key must be a single string field",
            ));
        }
    };

    if node.children.len() != 1 {
        return Err(YamlInteropError::InvalidContract(
            "yaml-entry must contain exactly one child value node",
        ));
    }

    Ok(InteropMapEntry {
        name,
        value: value_from_contract_node(&node.children[0])?,
    })
}

fn single_value_field<'a>(
    node: &'a InteropNode,
    expected_name: &'static str,
) -> Result<&'a InteropTypedValue, YamlInteropError> {
    if node.fields.len() != 1 {
        return Err(YamlInteropError::InvalidContract(match expected_name {
            YAML_STRING => "yaml-string must contain exactly one field",
            YAML_NUMBER => "yaml-number must contain exactly one field",
            YAML_BOOL => "yaml-bool must contain exactly one field",
            YAML_ENTRY => "yaml-entry must contain exactly one key field",
            _ => "contract node must contain exactly one field",
        }));
    }

    match &node.fields[0] {
        InteropField::Value(value) => Ok(value),
        InteropField::Property(_) => Err(YamlInteropError::InvalidContract(
            "contract nodes must not use named properties",
        )),
    }
}

fn ensure_childless(
    node: &InteropNode,
    expected_name: &'static str,
) -> Result<(), YamlInteropError> {
    if !node.children.is_empty() {
        return Err(YamlInteropError::InvalidContract(match expected_name {
            YAML_STRING => "yaml-string must not contain child nodes",
            YAML_NUMBER => "yaml-number must not contain child nodes",
            YAML_BOOL => "yaml-bool must not contain child nodes",
            _ => "contract node must not contain child nodes",
        }));
    }
    Ok(())
}
