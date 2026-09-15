//! Converting Ruby values into automerge operations.

use automerge::{transaction::Transactable, AutoCommit, ObjId, ObjType, Prop, ScalarValue};
use magnus::{
    prelude::*,
    r_hash::ForEach,
    value::{Qfalse, Qtrue},
    Error, Float, Integer, RArray, RHash, RObject, RString, Ruby, TryConvert, Value,
};

use crate::classes::{BYTES, COUNTER, TEXT, TIMESTAMP, UINT};
use crate::errors::{arg_error, automerge_error, type_error};
use crate::path;

/// Where a value goes: over an existing prop, or inserted into a list.
pub enum Slot {
    Put(Prop),
    Insert(usize),
}

enum Written {
    Scalar(ScalarValue),
    Map(RHash),
    List(RArray),
    Text(String),
}

pub fn write(
    ruby: &Ruby,
    doc: &mut AutoCommit,
    obj: &ObjId,
    slot: Slot,
    value: Value,
) -> Result<(), Error> {
    match classify(ruby, value)? {
        Written::Scalar(scalar) => match slot {
            Slot::Put(prop) => doc.put(obj, prop, scalar),
            Slot::Insert(index) => doc.insert(obj, index, scalar),
        }
        .map_err(|e| automerge_error(ruby, e)),
        Written::Map(hash) => {
            let child = create(ruby, doc, obj, slot, ObjType::Map)?;
            hash.foreach(|key: Value, item: Value| {
                let key = path::key(ruby, key)?;
                write(ruby, doc, &child, Slot::Put(Prop::Map(key)), item)?;
                Ok(ForEach::Continue)
            })
        }
        Written::List(array) => {
            let child = create(ruby, doc, obj, slot, ObjType::List)?;
            for index in 0..array.len() {
                let item = array.entry::<Value>(index as isize)?;
                write(ruby, doc, &child, Slot::Insert(index), item)?;
            }
            Ok(())
        }
        Written::Text(text) => {
            let child = create(ruby, doc, obj, slot, ObjType::Text)?;
            doc.splice_text(&child, 0, 0, &text)
                .map_err(|e| automerge_error(ruby, e))
        }
    }
}

fn create(
    ruby: &Ruby,
    doc: &mut AutoCommit,
    obj: &ObjId,
    slot: Slot,
    obj_type: ObjType,
) -> Result<ObjId, Error> {
    match slot {
        Slot::Put(prop) => doc.put_object(obj, prop, obj_type),
        Slot::Insert(index) => doc.insert_object(obj, index, obj_type),
    }
    .map_err(|e| automerge_error(ruby, e))
}

fn classify(ruby: &Ruby, value: Value) -> Result<Written, Error> {
    if value.is_nil() {
        return Ok(Written::Scalar(ScalarValue::Null));
    }
    if Qtrue::from_value(value).is_some() {
        return Ok(Written::Scalar(ScalarValue::Boolean(true)));
    }
    if Qfalse::from_value(value).is_some() {
        return Ok(Written::Scalar(ScalarValue::Boolean(false)));
    }
    if let Some(integer) = Integer::from_value(value) {
        return Ok(Written::Scalar(ScalarValue::Int(integer.to_i64()?)));
    }
    if let Some(float) = Float::from_value(value) {
        return Ok(Written::Scalar(ScalarValue::F64(float.to_f64())));
    }
    if let Some(string) = RString::from_value(value) {
        return Ok(Written::Scalar(ScalarValue::Str(
            utf8(ruby, string)?.into(),
        )));
    }
    if let Some(hash) = RHash::from_value(value) {
        return Ok(Written::Map(hash));
    }
    if let Some(array) = RArray::from_value(value) {
        return Ok(Written::List(array));
    }
    if value.is_kind_of(ruby.get_inner(&TEXT)) {
        return Ok(Written::Text(utf8(ruby, wrapped(value)?)?));
    }
    if value.is_kind_of(ruby.get_inner(&COUNTER)) {
        let count = wrapped::<Integer>(value)?.to_i64()?;
        return Ok(Written::Scalar(ScalarValue::counter(count)));
    }
    if value.is_kind_of(ruby.get_inner(&TIMESTAMP)) {
        let millis = wrapped::<Integer>(value)?.to_i64()?;
        return Ok(Written::Scalar(ScalarValue::Timestamp(millis)));
    }
    if value.is_kind_of(ruby.get_inner(&UINT)) {
        let uint = wrapped::<Integer>(value)?.to_u64()?;
        return Ok(Written::Scalar(ScalarValue::Uint(uint)));
    }
    if value.is_kind_of(ruby.get_inner(&BYTES)) {
        let bytes = unsafe { wrapped::<RString>(value)?.as_slice() }.to_vec();
        return Ok(Written::Scalar(ScalarValue::Bytes(bytes)));
    }
    Err(type_error(
        ruby,
        format!(
            "unsupported Automerge value type: {}",
            path::class_name(value)
        ),
    ))
}

/// The `@value` of a scalar wrapper, read without calling Ruby methods.
fn wrapped<T: TryConvert>(value: Value) -> Result<T, Error> {
    RObject::try_convert(value)?.ivar_get("@value")
}

fn utf8(ruby: &Ruby, string: RString) -> Result<String, Error> {
    let bytes = unsafe { string.as_slice() }.to_vec();
    String::from_utf8(bytes).map_err(|_| arg_error(ruby, "string is not valid UTF-8"))
}
