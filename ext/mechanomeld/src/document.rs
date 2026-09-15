use std::cell::{Ref, RefCell};

use automerge::{AutoCommit, LoadOptions, ObjType, ReadDoc, TextEncoding, ROOT};
use magnus::{prelude::*, Error, RString, Ruby, Value};

use crate::errors::{automerge_error, error};
use crate::{path, read};

/// Text indexes count Unicode code points, matching Ruby's `String#length`.
const ENCODING: TextEncoding = TextEncoding::UnicodeCodePoint;

#[magnus::wrap(class = "Mechanomeld::Document", free_immediately, size)]
pub struct Document {
    inner: RefCell<AutoCommit>,
}

impl Document {
    fn doc(&self, ruby: &Ruby) -> Result<Ref<'_, AutoCommit>, Error> {
        self.inner
            .try_borrow()
            .map_err(|_| error(ruby, "document is being modified"))
    }

    /// `Document.load(bytes)`
    pub fn load(ruby: &Ruby, bytes: RString) -> Result<Self, Error> {
        let data = unsafe { bytes.as_slice() }.to_vec();
        let doc = AutoCommit::load_with_options(&data, LoadOptions::new().text_encoding(ENCODING))
            .map_err(|e| error(ruby, format!("could not load Automerge document: {e}")))?;
        Ok(Self {
            inner: RefCell::new(doc),
        })
    }

    /// `doc.get(path)`: the value at `path`, or nil when anything along it is missing.
    pub fn get(ruby: &Ruby, rb_self: &Self, path: Value) -> Result<Value, Error> {
        let segments = path::segments(path)?;
        let doc = rb_self.doc(ruby)?;
        let Some((last, parents)) = segments.split_last() else {
            return read::object(ruby, &doc, &ROOT, ObjType::Map);
        };
        let Some((parent, parent_type)) = path::resolve(ruby, &doc, parents)? else {
            return Ok(ruby.qnil().as_value());
        };
        let prop = path::prop(ruby, parent_type, *last)?;
        match doc
            .get(&parent, prop)
            .map_err(|e| automerge_error(ruby, e))?
        {
            Some((value, id)) => read::value(ruby, &doc, value, id),
            None => Ok(ruby.qnil().as_value()),
        }
    }
}
