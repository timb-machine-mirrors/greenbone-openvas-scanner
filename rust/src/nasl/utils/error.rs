// SPDX-FileCopyrightText: 2024 Greenbone AG
//
// SPDX-License-Identifier: GPL-2.0-or-later WITH x11vnc-openssl-exception

use thiserror::Error;

use crate::nasl::builtin::BuiltinError;
use crate::nasl::prelude::NaslValue;

use crate::storage::StorageError;

#[derive(Debug, Clone, Error)]
#[error("{kind}")]
pub struct FnError {
    #[source]
    pub kind: FnErrorKind,
    return_value: Option<NaslValue>,
    retryable: bool,
}

impl FnError {
    pub fn return_value(&self) -> &Option<NaslValue> {
        &self.return_value
    }

    pub fn retryable(&self) -> bool {
        self.retryable
    }

    fn from_kind(kind: FnErrorKind) -> FnError {
        Self {
            kind,
            return_value: None,
            retryable: false,
        }
    }
}

impl From<FnErrorKind> for FnError {
    fn from(kind: FnErrorKind) -> Self {
        FnError::from_kind(kind)
    }
}

impl From<ArgumentError> for FnError {
    fn from(kind: ArgumentError) -> Self {
        FnError::from_kind(FnErrorKind::Argument(kind))
    }
}

impl From<BuiltinError> for FnError {
    fn from(kind: BuiltinError) -> Self {
        FnError::from_kind(FnErrorKind::Builtin(kind))
    }
}

impl From<InternalError> for FnError {
    fn from(kind: InternalError) -> Self {
        let retryable = kind.retryable();
        Self {
            kind: FnErrorKind::Internal(kind),
            retryable,
            return_value: None,
        }
    }
}

#[derive(Debug, Clone, Error)]
pub enum FnErrorKind {
    #[error("{0}")]
    Argument(ArgumentError),
    #[error("{0}")]
    Builtin(BuiltinError),
    #[error("{0}")]
    Internal(InternalError),
}

#[derive(Debug, Clone, PartialEq, Error)]
pub enum ArgumentError {
    #[error("Missing positional arguments. Expected {expected} but got {got}.")]
    MissingPositionals { expected: usize, got: usize },
    #[error("Trailing positional arguments. Expected {expected} but got {got}.")]
    TrailingPositionals { expected: usize, got: usize },
    #[error("Missing named arguments: {}", .0.join(", "))]
    MissingNamed(Vec<String>),
    #[error("Unknown named argument given: {}", .0)]
    UnexpectedArgument(String),
    #[error("Wrong arguments given: {0}")]
    WrongArgument(String),
}

#[derive(Debug, Clone, PartialEq, Error)]
pub enum InternalError {
    #[error("{0}")]
    Storage(#[from] StorageError),
}

impl InternalError {
    fn retryable(&self) -> bool {
        // Keep this match exhaustive without a catchall
        // to make sure we implement future internal errors
        // properly.
        match self {
            InternalError::Storage(StorageError::Retry(_)) => true,
            InternalError::Storage(_) => false,
        }
    }
}

pub trait WithErrorInfo<Info> {
    fn with(self, e: Info) -> Self;
}

pub struct ReturnValue<T>(pub T);

impl<T: Into<NaslValue>> WithErrorInfo<ReturnValue<T>> for FnError {
    fn with(mut self, val: ReturnValue<T>) -> Self {
        self.return_value = Some(val.0.into());
        self
    }
}

impl From<StorageError> for FnError {
    fn from(value: StorageError) -> Self {
        FnErrorKind::Internal(InternalError::Storage(value)).into()
    }
}

impl TryFrom<FnError> for ArgumentError {
    type Error = ();

    fn try_from(value: FnError) -> Result<Self, Self::Error> {
        match value.kind {
            FnErrorKind::Argument(e) => Ok(e),
            _ => Err(()),
        }
    }
}

impl TryFrom<FnError> for InternalError {
    type Error = ();

    fn try_from(value: FnError) -> Result<Self, Self::Error> {
        match value.kind {
            FnErrorKind::Internal(e) => Ok(e),
            _ => Err(()),
        }
    }
}

impl TryFrom<FnError> for BuiltinError {
    type Error = ();

    fn try_from(value: FnError) -> Result<Self, Self::Error> {
        match value.kind {
            FnErrorKind::Builtin(e) => Ok(e),
            _ => Err(()),
        }
    }
}

impl ArgumentError {
    /// Helper function to quickly construct a `WrongArgument` variant
    /// containing the name of the argument, the expected value and
    /// the actual value.
    pub fn wrong_argument(key: &str, expected: &str, got: &str) -> Self {
        ArgumentError::WrongArgument(format!("Expected {key} to be {expected} but it is {got}"))
    }
}

impl FnError {
    /// Helper function to quickly construct a `WrongArgument` variant
    /// containing the name of the argument, the expected value and
    /// the actual value.
    pub fn wrong_unnamed_argument(expected: &str, got: &str) -> Self {
        FnErrorKind::Argument(ArgumentError::WrongArgument(format!(
            "Expected {expected} but {got}"
        )))
        .into()
    }

    /// Helper function to quickly construct a `MissingArguments` variant
    /// for a single missing argument.
    pub fn missing_argument(val: &str) -> Self {
        FnErrorKind::Argument(ArgumentError::MissingNamed(vec![val.to_string()])).into()
    }
}
