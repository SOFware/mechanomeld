use std::borrow::Cow;
use std::fmt::Display;

use magnus::{Error, Ruby};

use crate::classes::ERROR;

/// A `Mechanomeld::Error`.
pub fn error<T: Into<Cow<'static, str>>>(ruby: &Ruby, message: T) -> Error {
    Error::new(ruby.get_inner(&ERROR), message)
}

/// A `Mechanomeld::Error` carrying an automerge error's message.
pub fn automerge_error(ruby: &Ruby, err: impl Display) -> Error {
    error(ruby, err.to_string())
}

pub fn type_error<T: Into<Cow<'static, str>>>(ruby: &Ruby, message: T) -> Error {
    Error::new(ruby.exception_type_error(), message)
}

pub fn arg_error<T: Into<Cow<'static, str>>>(ruby: &Ruby, message: T) -> Error {
    Error::new(ruby.exception_arg_error(), message)
}
