// Copyright (C) 2023 Greenbone Networks GmbH
//
// SPDX-License-Identifier: GPL-2.0-or-later

use crate::{
    error::{FunctionError, FunctionErrorKind::GeneralError},
    ContextType, NaslFunction, NaslValue, Register,
};

pub mod aes_cbc;
pub mod aes_ccm;
pub mod aes_ctr;
pub mod aes_gcm;
pub mod hmac;

enum Crypt {
    Encrypt,
    Decrypt,
}

pub(crate) fn lookup<K>(function_name: &str) -> Option<NaslFunction<K>>
where
    K: AsRef<str>,
{
    aes_ccm::lookup(function_name)
        .or_else(|| hmac::lookup(function_name))
        .or_else(|| aes_cbc::lookup(function_name))
        .or_else(|| aes_ctr::lookup(function_name))
        .or_else(|| aes_gcm::lookup(function_name))
}

/// Get named argument of Type Data or String from the register with appropriate error handling.
/// In case the argument is required, the returned value is either an Error or the Option is always
/// set to Some value. If it is false, no error will be returned but the Option can be either Some
/// or None.
fn get_named_data<'a>(
    register: &'a Register,
    key: &'a str,
    required: bool,
    function: &str,
) -> Result<Option<&'a [u8]>, FunctionError> {
    match register.named(key) {
        Some(ContextType::Value(NaslValue::Data(x))) => Ok(Some(x.as_slice())),
        Some(ContextType::Value(NaslValue::String(x))) => Ok(Some(x.as_bytes())),
        Some(x) => Err(FunctionError::new(
            function,
            (key, "a String or Data Value", format!("{:?}", x).as_str()).into(),
        )),
        _ => {
            if required {
                Err(FunctionError::new(function, (key).into()))
            } else {
                Ok(None)
            }
        }
    }
}

/// Get named argument of Type Number from the register with appropriate error handling.
/// In case the argument is required, the returned value is either an Error or the Option is always
/// set to Some value. If it is false, no error will be returned but the Option can be either Some
/// or None.
fn get_named_number(
    register: &Register,
    key: &str,
    required: bool,
    function: &str,
) -> Result<Option<i64>, FunctionError> {
    match register.named(key) {
        Some(ContextType::Value(NaslValue::Number(x))) => Ok(Some(*x)),
        Some(x) => Err(FunctionError::new(
            function,
            (key, "a Number Value", format!("{:?}", x).as_str()).into(),
        )),
        _ => {
            if required {
                Err(FunctionError::new(function, (key).into()))
            } else {
                Ok(None)
            }
        }
    }
}

/// Get the required key argument or error.
fn get_key<'a>(register: &'a Register, function: &str) -> Result<&'a [u8], FunctionError> {
    Ok(get_named_data(register, "key", true, function)?.unwrap())
}

/// Get the required data argument or error.
fn get_data<'a>(register: &'a Register, function: &str) -> Result<&'a [u8], FunctionError> {
    Ok(get_named_data(register, "data", true, function)?.unwrap())
}

/// Get the required iv argument or error.
fn get_iv<'a>(register: &'a Register, function: &str) -> Result<&'a [u8], FunctionError> {
    Ok(get_named_data(register, "iv", true, function)?.unwrap())
}

/// Get the optional len argument with proper error handling.
fn get_len(register: &Register, function: &str) -> Result<Option<usize>, FunctionError> {
    let buf = get_named_number(register, "len", false, function)?;
    match buf {
        None => Ok(None),
        Some(x) => match x.try_into() {
            Ok(y) => Ok(Some(y)),
            Err(_) => Err(FunctionError::new(
                function,
                GeneralError(format!(
                    "System only supports numbers between {:?} and {:?} but was {:?}",
                    usize::MIN,
                    usize::MAX,
                    x
                )),
            )),
        },
    }
}