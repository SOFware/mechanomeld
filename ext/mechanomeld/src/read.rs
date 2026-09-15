//! Converting automerge values into Ruby values.

use automerge::{AutoCommit, ObjId, ObjType, ReadDoc, ScalarValue, Value as AmValue};
use magnus::{prelude::*, value::Lazy, Error, IntoValue, RClass, RObject, Ruby, Value};

use crate::classes::{BYTES, COUNTER, TEXT, TIMESTAMP, UINT};
use crate::errors::{automerge_error, error};

pub fn value(ruby: &Ruby, doc: &AutoCommit, value: AmValue<'_>, id: ObjId) -> Result<Value, Error> {
    match value {
        AmValue::Object(obj_type) => object(ruby, doc, &id, obj_type),
        AmValue::Scalar(scalar_value) => scalar(ruby, &scalar_value),
    }
}

pub fn object(
    ruby: &Ruby,
    doc: &AutoCommit,
    obj: &ObjId,
    obj_type: ObjType,
) -> Result<Value, Error> {
    match obj_type {
        ObjType::Map => {
            let hash = ruby.hash_new();
            for key in doc.keys(obj) {
                if let Some((child, id)) = doc
                    .get(obj, key.as_str())
                    .map_err(|e| automerge_error(ruby, e))?
                {
                    hash.aset(key, value(ruby, doc, child, id)?)?;
                }
            }
            Ok(hash.as_value())
        }
        ObjType::List => {
            let length = doc.length(obj);
            let array = ruby.ary_new_capa(length);
            for index in 0..length {
                match doc.get(obj, index).map_err(|e| automerge_error(ruby, e))? {
                    Some((child, id)) => array.push(value(ruby, doc, child, id)?)?,
                    None => array.push(ruby.qnil())?,
                }
            }
            Ok(array.as_value())
        }
        ObjType::Text => {
            let text = doc.text(obj).map_err(|e| automerge_error(ruby, e))?;
            wrap(ruby, &TEXT, ruby.str_new(&text))
        }
        ObjType::Table => Err(error(ruby, "Automerge tables are not supported")),
    }
}

pub fn scalar(ruby: &Ruby, scalar: &ScalarValue) -> Result<Value, Error> {
    match scalar {
        ScalarValue::Str(string) => Ok(ruby.str_new(string.as_str()).as_value()),
        ScalarValue::Int(int) => Ok(ruby.integer_from_i64(*int).as_value()),
        ScalarValue::F64(float) => Ok(ruby.float_from_f64(*float).as_value()),
        ScalarValue::Boolean(boolean) => Ok(boolean.into_value_with(ruby)),
        ScalarValue::Null => Ok(ruby.qnil().as_value()),
        ScalarValue::Counter(counter) => {
            wrap(ruby, &COUNTER, ruby.integer_from_i64(i64::from(counter)))
        }
        ScalarValue::Timestamp(millis) => wrap(ruby, &TIMESTAMP, ruby.integer_from_i64(*millis)),
        ScalarValue::Uint(uint) => wrap(ruby, &UINT, ruby.integer_from_u64(*uint)),
        ScalarValue::Bytes(bytes) => wrap(ruby, &BYTES, ruby.str_from_slice(bytes)),
        ScalarValue::Unknown { type_code, .. } => Err(error(
            ruby,
            format!("unsupported Automerge value type code {type_code}"),
        )),
    }
}

/// Builds a scalar wrapper by allocating it and setting `@value`, without running Ruby code.
fn wrap(ruby: &Ruby, class: &'static Lazy<RClass>, value: impl IntoValue) -> Result<Value, Error> {
    let instance = ruby.get_inner(class).obj_alloc()?;
    let object = RObject::try_convert(instance.as_value())?;
    object.ivar_set("@value", value)?;
    Ok(object.as_value())
}
