//! Turning Ruby paths like `["todos", 0, "title"]` into automerge objects and props.

use automerge::{AutoCommit, ObjId, ObjType, Prop, ReadDoc, Value as AmValue, ROOT};
use magnus::{prelude::*, Error, Integer, RArray, RString, Ruby, Symbol, Value};

use crate::errors::{arg_error, automerge_error, error, type_error};

/// `nil` is the root, an Array is a list of segments, anything else is a single segment.
///
/// The returned Values stay alive for the method call because the caller's `path`
/// argument still references them.
pub fn segments(path: Value) -> Result<Vec<Value>, Error> {
    if path.is_nil() {
        return Ok(Vec::new());
    }
    let Some(array) = RArray::from_value(path) else {
        return Ok(vec![path]);
    };
    (0..array.len())
        .map(|index| array.entry::<Value>(index as isize))
        .collect()
}

pub fn key(ruby: &Ruby, segment: Value) -> Result<String, Error> {
    if let Some(symbol) = Symbol::from_value(segment) {
        return Ok(symbol.name()?.into_owned());
    }
    if let Some(string) = RString::from_value(segment) {
        return string.to_string();
    }
    Err(type_error(
        ruby,
        format!(
            "map key must be a String or Symbol, got {}",
            class_name(segment)
        ),
    ))
}

pub fn index(ruby: &Ruby, segment: Value) -> Result<usize, Error> {
    let Some(integer) = Integer::from_value(segment) else {
        return Err(type_error(
            ruby,
            format!("list index must be an Integer, got {}", class_name(segment)),
        ));
    };
    usize::try_from(integer.to_i64()?)
        .map_err(|_| arg_error(ruby, "list index must be non-negative"))
}

pub fn class_name(value: Value) -> String {
    unsafe { value.classname() }.into_owned()
}

/// The prop addressing `segment` inside an object of `obj_type`.
pub fn prop(ruby: &Ruby, obj_type: ObjType, segment: Value) -> Result<Prop, Error> {
    match obj_type {
        ObjType::Map | ObjType::Table => Ok(Prop::Map(key(ruby, segment)?)),
        ObjType::List => Ok(Prop::Seq(index(ruby, segment)?)),
        ObjType::Text => Err(error(ruby, "cannot descend into Automerge text")),
    }
}

/// Follows `segments` from the root. `None` when a key or index along the way is missing.
pub fn resolve(
    ruby: &Ruby,
    doc: &AutoCommit,
    segments: &[Value],
) -> Result<Option<(ObjId, ObjType)>, Error> {
    let mut obj = ROOT;
    let mut obj_type = ObjType::Map;
    for segment in segments {
        let prop = prop(ruby, obj_type, *segment)?;
        match doc.get(&obj, prop).map_err(|e| automerge_error(ruby, e))? {
            None => return Ok(None),
            Some((AmValue::Object(child_type), child)) => {
                obj = child;
                obj_type = child_type;
            }
            Some((AmValue::Scalar(_), _)) => {
                return Err(error(ruby, "path does not resolve to an Automerge object"))
            }
        }
    }
    Ok(Some((obj, obj_type)))
}

/// Like [`resolve`], but a missing key or index is an error.
pub fn resolve_existing(
    ruby: &Ruby,
    doc: &AutoCommit,
    segments: &[Value],
) -> Result<(ObjId, ObjType), Error> {
    resolve(ruby, doc, segments)?.ok_or_else(|| error(ruby, "path does not exist"))
}
