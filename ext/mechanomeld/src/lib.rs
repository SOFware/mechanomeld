mod classes;
mod document;
mod errors;
mod path;
mod read;
mod write;

use magnus::{function, method, prelude::*, Error, Ruby};

use crate::document::Document;

#[magnus::init]
fn init(ruby: &Ruby) -> Result<(), Error> {
    let module = ruby.define_module("Mechanomeld")?;
    let class = module.define_class("Document", ruby.class_object())?;
    class.define_singleton_method("new", function!(Document::new, -1))?;
    class.define_singleton_method("load", function!(Document::load, 1))?;
    class.define_method("get", method!(Document::get, 1))?;
    class.define_method("keys", method!(Document::keys, -1))?;
    class.define_method("length", method!(Document::length, -1))?;
    class.define_method("put", method!(Document::put, 2))?;
    class.define_method("delete", method!(Document::delete, 1))?;
    class.define_method("save", method!(Document::save, 0))?;
    Ok(())
}
