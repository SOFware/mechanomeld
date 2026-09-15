//! Ruby classes defined in lib/, looked up once on first use.

use magnus::{exception::ExceptionClass, prelude::*, value::Lazy, RClass, RModule};

pub static MECHANOMELD: Lazy<RModule> =
    Lazy::new(|ruby| ruby.define_module("Mechanomeld").unwrap());

pub static ERROR: Lazy<ExceptionClass> =
    Lazy::new(|ruby| ruby.get_inner(&MECHANOMELD).const_get("Error").unwrap());

pub static TEXT: Lazy<RClass> =
    Lazy::new(|ruby| ruby.get_inner(&MECHANOMELD).const_get("Text").unwrap());

pub static COUNTER: Lazy<RClass> =
    Lazy::new(|ruby| ruby.get_inner(&MECHANOMELD).const_get("Counter").unwrap());

pub static TIMESTAMP: Lazy<RClass> =
    Lazy::new(|ruby| ruby.get_inner(&MECHANOMELD).const_get("Timestamp").unwrap());

pub static UINT: Lazy<RClass> =
    Lazy::new(|ruby| ruby.get_inner(&MECHANOMELD).const_get("Uint").unwrap());

pub static BYTES: Lazy<RClass> =
    Lazy::new(|ruby| ruby.get_inner(&MECHANOMELD).const_get("Bytes").unwrap());
