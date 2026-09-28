//! Converting automerge patches into Ruby hashes.
//!
//! Every patch becomes `{"action" => ..., "path" => [...]}` plus the fields that
//! action carries. `path` addresses the property the patch touches from the root, as
//! JavaScript's `Automerge.diff` reports it. A put or insert of a map, list, or text
//! carries the empty container; the children arrive as their own patches.

use automerge::{marks::Mark, ObjType, Patch, PatchAction, Prop, Value as AmValue};
use magnus::{prelude::*, Error, RHash, Ruby, Value};

use crate::classes::TEXT;
use crate::errors::error;
use crate::read;

pub fn patch(ruby: &Ruby, patch: &Patch) -> Result<RHash, Error> {
    let (action, last) = match &patch.action {
        PatchAction::PutMap { key, .. } => ("put", Some(Prop::Map(key.clone()))),
        PatchAction::PutSeq { index, .. } => ("put", Some(Prop::Seq(*index))),
        PatchAction::Insert { index, .. } => ("insert", Some(Prop::Seq(*index))),
        PatchAction::SpliceText { index, .. } => ("splice_text", Some(Prop::Seq(*index))),
        PatchAction::Increment { prop, .. } => ("increment", Some(prop.clone())),
        PatchAction::Conflict { prop } => ("conflict", Some(prop.clone())),
        PatchAction::DeleteMap { key } => ("delete", Some(Prop::Map(key.clone()))),
        PatchAction::DeleteSeq { index, .. } => ("delete", Some(Prop::Seq(*index))),
        PatchAction::Mark { .. } => ("mark", None),
    };
    let path = ruby.ary_new_capa(patch.path.len() + 1);
    for (_, prop) in &patch.path {
        path.push(prop_value(ruby, prop))?;
    }
    if let Some(prop) = &last {
        path.push(prop_value(ruby, prop))?;
    }
    let hash = ruby.hash_new();
    hash.aset("action", action)?;
    hash.aset("path", path)?;
    match &patch.action {
        PatchAction::PutMap {
            value, conflict, ..
        }
        | PatchAction::PutSeq {
            value, conflict, ..
        } => {
            hash.aset("value", self::value(ruby, &value.0)?)?;
            hash.aset("conflict", *conflict)?;
        }
        PatchAction::Insert { values, .. } => {
            let array = ruby.ary_new_capa(values.len());
            for (item, _, _) in values.iter() {
                array.push(self::value(ruby, item)?)?;
            }
            hash.aset("values", array)?;
        }
        PatchAction::SpliceText { value, .. } => {
            hash.aset("value", ruby.str_new(&value.make_string()))?;
        }
        PatchAction::Increment { value, .. } => {
            hash.aset("value", *value)?;
        }
        PatchAction::DeleteSeq { index, length } => {
            hash.aset("index", *index)?;
            hash.aset("length", *length)?;
        }
        PatchAction::Mark { marks } => {
            let array = ruby.ary_new_capa(marks.len());
            for mark in marks {
                array.push(self::mark(ruby, mark)?)?;
            }
            hash.aset("marks", array)?;
        }
        PatchAction::Conflict { .. } | PatchAction::DeleteMap { .. } => {}
    }
    Ok(hash)
}

fn prop_value(ruby: &Ruby, prop: &Prop) -> Value {
    match prop {
        Prop::Map(key) => ruby.str_new(key).as_value(),
        Prop::Seq(index) => ruby.integer_from_u64(*index as u64).as_value(),
    }
}

fn value(ruby: &Ruby, value: &AmValue<'_>) -> Result<Value, Error> {
    match value {
        AmValue::Scalar(scalar) => read::scalar(ruby, scalar),
        AmValue::Object(ObjType::Map) => Ok(ruby.hash_new().as_value()),
        AmValue::Object(ObjType::List) => Ok(ruby.ary_new().as_value()),
        AmValue::Object(ObjType::Text) => read::wrap(ruby, &TEXT, ruby.str_new("")),
        AmValue::Object(ObjType::Table) => Err(error(ruby, "Automerge tables are not supported")),
    }
}

fn mark(ruby: &Ruby, mark: &Mark) -> Result<RHash, Error> {
    let hash = ruby.hash_new();
    hash.aset("name", mark.name.as_str())?;
    hash.aset("value", read::scalar(ruby, &mark.value)?)?;
    hash.aset("start", mark.start)?;
    hash.aset("end", mark.end)?;
    Ok(hash)
}
