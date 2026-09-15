use std::cell::{Ref, RefCell, RefMut};
use std::str::FromStr;

use automerge::{
    transaction::Transactable, ActorId, AutoCommit, LoadOptions, ObjType, ReadDoc, TextEncoding,
    ROOT,
};
use magnus::{
    prelude::*,
    scan_args::{get_kwargs, scan_args},
    typed_data::Obj,
    Error, RArray, RHash, RString, Ruby, Value,
};

use crate::errors::{arg_error, automerge_error, error};
use crate::{path, read, write};

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

    fn doc_mut(&self, ruby: &Ruby) -> Result<RefMut<'_, AutoCommit>, Error> {
        self.inner
            .try_borrow_mut()
            .map_err(|_| error(ruby, "document is already in use"))
    }

    /// `Document.new(actor_id: nil)`
    pub fn new(ruby: &Ruby, args: &[Value]) -> Result<Self, Error> {
        let args = scan_args::<(), (), (), (), RHash, ()>(args)?;
        let kwargs =
            get_kwargs::<_, (), (Option<Option<String>>,), ()>(args.keywords, &[], &["actor_id"])?;
        let mut doc = AutoCommit::new_with_encoding(ENCODING);
        if let (Some(Some(actor_id)),) = kwargs.optional {
            let actor = ActorId::from_str(&actor_id)
                .map_err(|e| error(ruby, format!("invalid actor id: {e}")))?;
            doc.set_actor(actor);
        }
        Ok(Self {
            inner: RefCell::new(doc),
        })
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

    /// `doc.keys(path = [])`
    pub fn keys(ruby: &Ruby, rb_self: &Self, args: &[Value]) -> Result<RArray, Error> {
        let segments = optional_path(ruby, args)?;
        let doc = rb_self.doc(ruby)?;
        let (obj, obj_type) = path::resolve_existing(ruby, &doc, &segments)?;
        if obj_type != ObjType::Map {
            return Err(error(ruby, "keys target must be an Automerge map"));
        }
        Ok(ruby.ary_from_iter(doc.keys(&obj)))
    }

    /// `doc.length(path = [])`
    pub fn length(ruby: &Ruby, rb_self: &Self, args: &[Value]) -> Result<usize, Error> {
        let segments = optional_path(ruby, args)?;
        let doc = rb_self.doc(ruby)?;
        let (obj, _) = path::resolve_existing(ruby, &doc, &segments)?;
        Ok(doc.length(&obj))
    }

    /// `doc.put(path, value)`
    pub fn put(
        ruby: &Ruby,
        rb_self: Obj<Self>,
        path: Value,
        value: Value,
    ) -> Result<Obj<Self>, Error> {
        let segments = path::segments(path)?;
        let Some((last, parents)) = segments.split_last() else {
            return Err(arg_error(ruby, "path must not be empty"));
        };
        {
            let mut doc = rb_self.doc_mut(ruby)?;
            let (parent, parent_type) = path::resolve_existing(ruby, &doc, parents)?;
            let prop = path::write_prop(ruby, &doc, &parent, parent_type, *last)?;
            write::write(ruby, &mut doc, &parent, write::Slot::Put(prop), value)?;
        }
        Ok(rb_self)
    }

    /// `doc.delete(path)`
    pub fn delete(ruby: &Ruby, rb_self: Obj<Self>, path: Value) -> Result<Obj<Self>, Error> {
        let segments = path::segments(path)?;
        let Some((last, parents)) = segments.split_last() else {
            return Err(arg_error(ruby, "path must not be empty"));
        };
        {
            let mut doc = rb_self.doc_mut(ruby)?;
            let (parent, parent_type) = path::resolve_existing(ruby, &doc, parents)?;
            let prop = path::write_prop(ruby, &doc, &parent, parent_type, *last)?;
            doc.delete(&parent, prop)
                .map_err(|e| automerge_error(ruby, e))?;
        }
        Ok(rb_self)
    }

    /// `doc.save`: the document as a binary String.
    pub fn save(ruby: &Ruby, rb_self: &Self) -> Result<RString, Error> {
        let bytes = rb_self.doc_mut(ruby)?.save();
        Ok(ruby.str_from_slice(&bytes))
    }
}

fn optional_path(ruby: &Ruby, args: &[Value]) -> Result<Vec<Value>, Error> {
    let args = scan_args::<(), (Option<Value>,), (), (), (), ()>(args)?;
    path::segments(args.optional.0.unwrap_or_else(|| ruby.qnil().as_value()))
}
