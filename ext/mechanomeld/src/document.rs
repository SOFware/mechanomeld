use std::cell::{Ref, RefCell, RefMut};
use std::str::FromStr;

use automerge::{
    transaction::{CommitOptions, Transactable},
    ActorId, AutoCommit, ChangeHash, LoadOptions, ObjType, ReadDoc, TextEncoding, ROOT,
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

    /// `doc.load_incremental(bytes)`: applies a full save, snapshot, or incremental
    /// change chunk into this document.
    pub fn load_incremental(
        ruby: &Ruby,
        rb_self: Obj<Self>,
        bytes: RString,
    ) -> Result<Obj<Self>, Error> {
        let data = unsafe { bytes.as_slice() }.to_vec();
        rb_self
            .doc_mut(ruby)?
            .load_incremental(&data)
            .map_err(|e| error(ruby, format!("could not load Automerge document: {e}")))?;
        Ok(rb_self)
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

    /// `doc.commit(message: nil, timestamp: nil)`: the binary change hash, or nil.
    /// `timestamp` is Unix seconds (automerge's commit time), unlike Timestamp values.
    pub fn commit(ruby: &Ruby, rb_self: &Self, args: &[Value]) -> Result<Option<RString>, Error> {
        let args = scan_args::<(), (), (), (), RHash, ()>(args)?;
        let kwargs = get_kwargs::<_, (), (Option<Option<String>>, Option<Option<i64>>), ()>(
            args.keywords,
            &[],
            &["message", "timestamp"],
        )?;
        let (message, timestamp) = kwargs.optional;
        let mut options = CommitOptions::default();
        if let Some(Some(message)) = message {
            options = options.with_message(message);
        }
        if let Some(Some(seconds)) = timestamp {
            options = options.with_time(seconds);
        }
        let hash = rb_self.doc_mut(ruby)?.commit_with(options);
        Ok(hash.map(|hash| ruby.str_from_slice(&hash.0)))
    }

    /// `doc.rollback`: the number of pending operations discarded.
    pub fn rollback(ruby: &Ruby, rb_self: &Self) -> Result<usize, Error> {
        Ok(rb_self.doc_mut(ruby)?.rollback())
    }

    /// `doc.save`: the document as a binary String.
    pub fn save(ruby: &Ruby, rb_self: &Self) -> Result<RString, Error> {
        let bytes = rb_self.doc_mut(ruby)?.save();
        Ok(ruby.str_from_slice(&bytes))
    }

    /// `doc.heads`: the current heads as lowercase hex change hashes, the format
    /// JavaScript's `Automerge.getHeads` returns.
    pub fn heads(ruby: &Ruby, rb_self: &Self) -> Result<RArray, Error> {
        let heads = rb_self.doc_mut(ruby)?.get_heads();
        Ok(ruby.ary_from_iter(heads.iter().map(ChangeHash::to_string)))
    }

    /// `doc.includes_heads?(heads)`: whether every hex change hash in `heads` is a
    /// change this document already contains.
    pub fn includes_heads(ruby: &Ruby, rb_self: &Self, heads: Vec<String>) -> Result<bool, Error> {
        let hashes = heads
            .iter()
            .map(|hex| change_hash(ruby, hex))
            .collect::<Result<Vec<_>, _>>()?;
        let mut doc = rb_self.doc_mut(ruby)?;
        Ok(hashes
            .iter()
            .all(|hash| doc.get_change_meta_by_hash(hash).is_some()))
    }
}

fn change_hash(ruby: &Ruby, hex: &str) -> Result<ChangeHash, Error> {
    ChangeHash::from_str(hex).map_err(|e| error(ruby, format!("invalid change hash {hex:?}: {e}")))
}

fn optional_path(ruby: &Ruby, args: &[Value]) -> Result<Vec<Value>, Error> {
    let args = scan_args::<(), (Option<Value>,), (), (), (), ()>(args)?;
    path::segments(args.optional.0.unwrap_or_else(|| ruby.qnil().as_value()))
}
