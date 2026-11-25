// SPDX-License-Identifier: MIT OR Apache-2.0
//! Neutral interop model and KDL conversion hooks.
//!
//! This module intentionally separates the neutral model from any future
//! format-specific adapters. KDL converts through the model without routing
//! other formats through raw KDL text.

#[cfg(feature = "json-interop")]
#[cfg_attr(docsrs, doc(cfg(feature = "json-interop")))]
pub mod json;
pub mod kdl;
pub mod model;
#[cfg(feature = "toml-interop")]
#[cfg_attr(docsrs, doc(cfg(feature = "toml-interop")))]
pub mod toml;
#[cfg(feature = "yaml-interop")]
#[cfg_attr(docsrs, doc(cfg(feature = "yaml-interop")))]
pub mod yaml;

#[cfg(feature = "json-interop")]
pub use self::json::JsonInteropError;
pub use self::kdl::InteropError;
pub use self::model::{
    InteropDocument, InteropField, InteropMapEntry, InteropNode, InteropNumber, InteropProperty,
    InteropRoot, InteropTemporal, InteropTemporalKind, InteropTypedValue, InteropValue,
};
#[cfg(feature = "toml-interop")]
#[cfg_attr(docsrs, doc(cfg(feature = "toml-interop")))]
pub use self::toml::TomlInteropError;
#[cfg(feature = "yaml-interop")]
pub use self::yaml::YamlInteropError;
