// SPDX-FileCopyrightText: 2023 Greenbone AG
//
// SPDX-License-Identifier: GPL-2.0-or-later WITH x11vnc-openssl-exception

use std::io;

use crate::nasl::syntax::LoadError;
use crate::nasl::syntax::{Statement, SyntaxError, TokenCategory};
use crate::nasl::utils::error::NaslError;
use crate::storage::StorageError;
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Error)]
/// An error that occurred while calling a function
#[error("Error while calling function '{function}': {kind}")]
pub struct FunctionError {
    /// Name of the function
    pub function: String,
    /// Kind of error
    #[source]
    pub kind: NaslError,
}

impl FunctionError {
    /// Creates a new FunctionError
    pub fn new(function: &str, kind: NaslError) -> Self {
        Self {
            function: function.to_owned(),
            kind,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
/// Is used to represent an error while interpreting
#[error("{}{kind}", self.origin.clone().map(|e| format!("{e}: ")).unwrap_or_default())]
pub struct InterpretError {
    /// Defined the type of error that occurred.
    #[source]
    pub kind: InterpretErrorKind,
    /// The statement on which this error occurred.
    pub origin: Option<Statement>,
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
/// Is used to give hints to the user how to react on an error while interpreting
pub enum InterpretErrorKind {
    /// When returned context is a function when a value is required.
    #[error("Expected a value but got a function.")]
    FunctionExpectedValue,
    /// When returned context is a value when a function is required.
    #[error("Expected a function but got a value.")]
    ValueExpectedFunction,
    /// When a specific type is expected
    #[error("Expected the type {0}")]
    WrongType(String),
    /// When a specific token category is required but not given.
    #[error("Expected the category {0}")]
    WrongCategory(TokenCategory),
    /// Regex parsing went wrong.
    #[error("Invalid regular expression: {0}")]
    InvalidRegex(String),
    /// A SyntaxError while including another script
    #[error("Error while including file {filename}{}: {err}", {err}.as_token() .map(|t| format!(", line: {}, col: {}", t.position.0, t.position.1)) .unwrap_or_default())]
    IncludeSyntaxError {
        /// The name of the file trying to include
        filename: String,
        /// The syntactical error that occurred
        err: SyntaxError,
    },
    /// SyntaxError
    #[error("{0}")]
    SyntaxError(SyntaxError),
    /// When the given key was not found in the context
    #[error("Key not found: {0}")]
    NotFound(String),
    /// A StorageError occurred
    // FIXME rename to general error
    #[error("{0}")]
    StorageError(StorageError),
    /// A LoadError occurred
    #[error("{0}")]
    LoadError(LoadError),
    /// A Formatting error occurred
    #[error("{0}")]
    FMTError(std::fmt::Error),
    /// An IOError occurred
    #[error("{0}")]
    IOError(io::ErrorKind),
    /// An error occurred while calling a built-in function.
    #[error("{0}")]
    FunctionCallError(FunctionError),
}

impl InterpretError {
    /// Creates a new Error with line and col set to 0
    ///
    /// Use this only when there is no statement available.
    /// If the line as well as col is null Interpreter::resolve will replace it
    /// with the line and col number based on the root statement.
    pub fn new(kind: InterpretErrorKind, origin: Option<Statement>) -> Self {
        Self { kind, origin }
    }

    /// Creates a new Error based on a given statement and reason
    pub fn from_statement(stmt: &Statement, kind: InterpretErrorKind) -> Self {
        InterpretError {
            kind,
            origin: Some(stmt.clone()),
        }
    }

    /// Returns the column number
    pub fn column(&self) -> usize {
        let (_, col) = self.line_column();
        col
    }

    /// Returns the line number
    pub fn line(&self) -> usize {
        let (line, _) = self.line_column();
        line
    }

    /// Returns the line and column number
    ///
    /// Based on the stored statement the line and column number are retained.
    /// That start both at 1 the only occurrence when there is no such number is when either
    /// no statement is stored in this error or when the given statement is either a
    /// - Statement::EoF,
    /// - Statement::AttackCategory,
    /// - Statement::Continue,
    /// - Statement::Break
    ///
    /// On resolve of an interpreter that should be an map_err adding a origin when there is none so the
    /// case of returning 0,0 is in most cases a bug.
    pub fn line_column(&self) -> (usize, usize) {
        self.origin
            .as_ref()
            .map(|stmt| stmt.as_token())
            .map(|x| x.line_column)
            .unwrap_or_default()
    }

    /// Creates a InterpreterError for an unsupported statement
    ///
    /// It produces the reason {root}: {statement} is not supported
    pub fn unsupported(stmt: &Statement, expected: &str) -> Self {
        Self::from_statement(stmt, InterpretErrorKind::WrongType(expected.to_string()))
    }

    /// Creates an InterpreterError if the found context is a function although a value is required
    pub fn expected_value() -> Self {
        Self::new(InterpretErrorKind::FunctionExpectedValue, None)
    }

    /// Creates an InterpreterError if the found context is a value although a function is required
    pub fn expected_function() -> Self {
        Self::new(InterpretErrorKind::ValueExpectedFunction, None)
    }

    /// Creates an error if the TokenCategory is wrong
    pub fn wrong_category(cat: &TokenCategory) -> Self {
        Self::new(InterpretErrorKind::WrongCategory(cat.clone()), None)
    }

    /// When something was not found
    pub fn not_found(name: &str) -> Self {
        Self::new(InterpretErrorKind::NotFound(name.to_owned()), None)
    }

    /// When a include file has syntactical errors
    pub fn include_syntax_error(file: &str, se: SyntaxError) -> Self {
        Self::new(
            InterpretErrorKind::IncludeSyntaxError {
                filename: file.to_owned(),
                err: se,
            },
            None,
        )
    }
    /// When a given regex is not parseable
    pub fn unparse_regex(rx: &str) -> Self {
        Self::new(InterpretErrorKind::InvalidRegex(rx.to_owned()), None)
    }
}

impl From<TokenCategory> for InterpretError {
    fn from(cat: TokenCategory) -> Self {
        Self::new(InterpretErrorKind::WrongCategory(cat), None)
    }
}

impl From<SyntaxError> for InterpretError {
    fn from(err: SyntaxError) -> Self {
        Self::new(InterpretErrorKind::SyntaxError(err), None)
    }
}

impl From<StorageError> for InterpretError {
    fn from(se: StorageError) -> Self {
        Self::new(InterpretErrorKind::StorageError(se), None)
    }
}

impl From<io::ErrorKind> for InterpretError {
    fn from(ie: io::ErrorKind) -> Self {
        Self::new(InterpretErrorKind::IOError(ie), None)
    }
}

impl From<io::Error> for InterpretError {
    fn from(e: io::Error) -> Self {
        e.kind().into()
    }
}

impl From<std::fmt::Error> for InterpretError {
    fn from(fe: std::fmt::Error) -> Self {
        Self::new(InterpretErrorKind::FMTError(fe), None)
    }
}

impl From<LoadError> for InterpretError {
    fn from(le: LoadError) -> Self {
        Self::new(InterpretErrorKind::LoadError(le), None)
    }
}

impl From<FunctionError> for InterpretError {
    fn from(fe: FunctionError) -> Self {
        match fe.kind {
            NaslError::IOError(ie) => ie.into(),
            NaslError::GeneralError(e) => Self::new(InterpretErrorKind::StorageError(e), None),
            _ => Self::new(InterpretErrorKind::FunctionCallError(fe), None),
        }
    }
}
