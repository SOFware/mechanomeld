mod classes;
mod diff;
mod document;
mod errors;
mod path;
mod read;
mod sync_state;
mod write;

use magnus::{function, method, prelude::*, Error, Ruby};

use crate::document::Document;
use crate::sync_state::SyncState;

#[magnus::init]
fn init(ruby: &Ruby) -> Result<(), Error> {
    let module = ruby.define_module("Mechanomeld")?;
    let class = module.define_class("Document", ruby.class_object())?;
    class.define_singleton_method("new", function!(Document::new, -1))?;
    class.define_singleton_method("load", function!(Document::load, 1))?;
    class.define_method("load_incremental", method!(Document::load_incremental, 1))?;
    class.define_method("get", method!(Document::get, 1))?;
    class.define_method("keys", method!(Document::keys, -1))?;
    class.define_method("length", method!(Document::length, -1))?;
    class.define_method("put", method!(Document::put, 2))?;
    class.define_method("delete", method!(Document::delete, 1))?;
    class.define_method("commit", method!(Document::commit, -1))?;
    class.define_method("rollback", method!(Document::rollback, 0))?;
    class.define_method("save", method!(Document::save, 0))?;
    class.define_method("heads", method!(Document::heads, 0))?;
    class.define_method("includes_heads?", method!(Document::includes_heads, 1))?;
    class.define_method("diff", method!(Document::diff, -1))?;
    class.define_method(
        "generate_sync_message",
        method!(Document::generate_sync_message, 1),
    )?;
    class.define_method(
        "receive_sync_message",
        method!(Document::receive_sync_message, 2),
    )?;
    let sync_state = module.define_class("SyncState", ruby.class_object())?;
    sync_state.define_singleton_method("new", function!(SyncState::new, 0))?;
    sync_state.define_singleton_method("decode", function!(SyncState::decode, 1))?;
    sync_state.define_method("encode", method!(SyncState::encode, 0))?;
    Ok(())
}
