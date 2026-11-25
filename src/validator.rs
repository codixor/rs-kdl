// SPDX-License-Identifier: MIT OR Apache-2.0
//! Verification of event streams.
//!
//! Most library functions produce and expect to receive structurally-valid
//! event streams, use this to ensure untrusted event streams are valid.
//!
//! You probably want to start at [`Validator`].

use crate::dom::Event;
use thiserror::Error;

/// Error in validation
#[derive(Clone, Debug, Error)]
#[non_exhaustive]
pub enum ValidatorError {
    #[error("Too many End events")]
    TooManyEnd,
    #[error("Unclosed nodes at end")]
    Unclosed,
    #[error("Expected {0}")]
    ExpectedEvent(&'static str),
}

#[derive(Debug, Clone, Copy)]
enum State {
    /// Node, End, Done
    Block,
    /// Entry, Children, End
    Node,
}

/// Event stream validator
#[derive(Debug)]
pub struct Validator {
    nest: usize,
    state: State,
}

impl Default for Validator {
    fn default() -> Self {
        Self::new()
    }
}

impl Validator {
    /// Create a new validator.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            nest: 0,
            state: State::Block,
        }
    }
    /// Feed an event into validator.
    /// # Errors
    /// Returns any validation errors.
    pub fn push(&mut self, event: &Event) -> Result<(), ValidatorError> {
        self.state = match (self.state, event) {
            (_, Event::End) => {
                if let Some(nest) = self.nest.checked_sub(1) {
                    self.nest = nest;
                    State::Block
                } else {
                    return Err(ValidatorError::TooManyEnd);
                }
            }
            (State::Node, Event::Entry { .. }) => State::Node,
            (State::Node, Event::Children) => State::Block,
            (State::Block, Event::Node { .. }) => {
                self.nest += 1;
                State::Node
            }
            (State::Node, _) => {
                return Err(ValidatorError::ExpectedEvent("Entry, Children, or End"));
            }
            (State::Block, _) => {
                return Err(ValidatorError::ExpectedEvent("Node or End"));
            }
        };
        Ok(())
    }
    /// Mark the end of the event stream.
    /// # Errors
    /// Returns a validation error from any unclosed nodes
    pub fn done(self) -> Result<(), ValidatorError> {
        if self.nest > 0 {
            Err(ValidatorError::Unclosed)
        } else {
            Ok(())
        }
    }
}
