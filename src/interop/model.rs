// SPDX-License-Identifier: MIT OR Apache-2.0
//! Neutral data/document model for additive format interop.

use alloc::vec::Vec;

use smol_str::SmolStr;

/// Neutral document root for future format interop.
///
/// Some formats are naturally document/node oriented, while others are rooted
/// in a single value. Keeping both shapes here prevents the crate from forcing
/// every format through raw KDL syntax.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct InteropDocument {
    /// Root payload for the interop document.
    pub root: InteropRoot,
}

impl InteropDocument {
    /// Create a node-root interop document.
    #[must_use]
    pub fn nodes(nodes: Vec<InteropNode>) -> Self {
        Self {
            root: InteropRoot::Nodes(nodes),
        }
    }

    /// Create a value-root interop document.
    #[must_use]
    pub fn value(value: InteropValue) -> Self {
        Self {
            root: InteropRoot::Value(value),
        }
    }
}

/// Root payload for the interop document.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum InteropRoot {
    /// Document-like root made of ordered nodes.
    Nodes(Vec<InteropNode>),
    /// Value-like root for JSON/TOML/YAML-style trees.
    Value(InteropValue),
}

/// Neutral node representation used by the KDL adapter.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct InteropNode {
    /// Node name.
    pub name: SmolStr,
    /// Optional node type hint.
    pub type_hint: Option<SmolStr>,
    /// Ordered fields on the node.
    pub fields: Vec<InteropField>,
    /// Child nodes.
    pub children: Vec<InteropNode>,
}

impl InteropNode {
    /// Create a node with no fields or children.
    #[must_use]
    pub fn new<T: Into<SmolStr>>(name: T) -> Self {
        Self {
            name: name.into(),
            type_hint: None,
            fields: Vec::new(),
            children: Vec::new(),
        }
    }
}

/// Ordered field on a neutral node.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum InteropField {
    /// Positional value field.
    Value(InteropTypedValue),
    /// Named property field.
    Property(InteropProperty),
}

/// Typed scalar/composite value.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct InteropTypedValue {
    /// Optional type hint carried by formats that support it.
    pub type_hint: Option<SmolStr>,
    /// Actual payload value.
    pub value: InteropValue,
}

impl InteropTypedValue {
    /// Create an untyped interop value wrapper.
    #[must_use]
    pub fn new(value: InteropValue) -> Self {
        Self {
            type_hint: None,
            value,
        }
    }
}

/// Named property on a neutral node.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct InteropProperty {
    /// Property name.
    pub name: SmolStr,
    /// Typed property payload.
    pub typed: InteropTypedValue,
}

/// Neutral value tree used by future JSON/TOML/YAML adapters.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub enum InteropValue {
    /// String value.
    String(SmolStr),
    /// Number text kept in a format-neutral wrapper.
    Number(InteropNumber),
    /// Boolean value.
    Bool(bool),
    /// Null value.
    #[default]
    Null,
    /// Temporal TOML-style value preserved as canonical text plus kind.
    Temporal(InteropTemporal),
    /// Sequence value.
    Sequence(Vec<InteropValue>),
    /// Map/object value.
    Map(Vec<InteropMapEntry>),
}

/// Neutral object/map entry.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct InteropMapEntry {
    /// Map key.
    pub name: SmolStr,
    /// Map value.
    pub value: InteropValue,
}

/// Neutral textual number wrapper.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct InteropNumber {
    /// Number text in a format-neutral form.
    pub text: SmolStr,
}

impl InteropNumber {
    /// Create a neutral number wrapper.
    #[must_use]
    pub fn new<T: Into<SmolStr>>(text: T) -> Self {
        Self { text: text.into() }
    }

    /// Borrow the raw number text.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.text
    }
}

/// Neutral textual temporal wrapper used by TOML interop.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct InteropTemporal {
    /// Temporal kind.
    pub kind: InteropTemporalKind,
    /// Canonical textual representation.
    pub text: SmolStr,
}

impl InteropTemporal {
    /// Create a neutral temporal wrapper.
    #[must_use]
    pub fn new<T: Into<SmolStr>>(kind: InteropTemporalKind, text: T) -> Self {
        Self {
            kind,
            text: text.into(),
        }
    }

    /// Borrow the raw temporal text.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.text
    }
}

/// Neutral temporal kind used by TOML interop.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum InteropTemporalKind {
    /// RFC 3339 offset date-time.
    OffsetDateTime,
    /// Local date-time with no offset.
    LocalDateTime,
    /// Local date with no time component.
    LocalDate,
    /// Local time with no date component.
    LocalTime,
}

impl InteropTemporalKind {
    /// Stable KDL type hint used to preserve TOML temporal kinds.
    #[must_use]
    pub fn kdl_type_hint(self) -> SmolStr {
        match self {
            Self::OffsetDateTime => "toml-offset-date-time".into(),
            Self::LocalDateTime => "toml-local-date-time".into(),
            Self::LocalDate => "toml-local-date".into(),
            Self::LocalTime => "toml-local-time".into(),
        }
    }

    /// Parse a KDL type hint back into a temporal kind.
    #[must_use]
    pub fn from_kdl_type_hint(type_hint: &str) -> Option<Self> {
        match type_hint {
            "toml-offset-date-time" => Some(Self::OffsetDateTime),
            "toml-local-date-time" => Some(Self::LocalDateTime),
            "toml-local-date" => Some(Self::LocalDate),
            "toml-local-time" => Some(Self::LocalTime),
            _ => None,
        }
    }
}
