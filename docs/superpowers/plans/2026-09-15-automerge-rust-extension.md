# Mechanomeld Automerge Extension Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** A Ruby gem, `mechanomeld`, that loads and reads Automerge documents written by JavaScript, then creates and saves documents JavaScript can load.

**Architecture:** A Rust native extension (magnus + rb-sys) wraps `automerge::AutoCommit` in `Mechanomeld::Document`. Value conversion and path walking live in Rust; scalar wrapper classes and conveniences (`[]`, `to_h`, `change`, `from`) are plain Ruby. mise pins the toolchain and runs tasks; reissue plus a SOFware shared workflow releases the gem, and oxidize-rb actions build precompiled platform gems.

**Tech Stack:** Ruby ≥ 3.2 (4.0 for development), Rust 1.90, magnus 0.8, rb-sys 0.9.130, automerge 0.11, Minitest 5, Node 24 with `@automerge/automerge` 3.4.1 (fixtures and interop only), mise, reissue, GitHub Actions.

**Spec:** `docs/superpowers/specs/2026-09-15-automerge-rust-extension-design.md`

The final state of all code below was compiled and its tests run (70 runs, 98 assertions, 0 failures; clippy clean; JS interop passing) in a throwaway prototype on arm64-darwin / Ruby 4.0.6 before this plan was written. The intermediate states between tasks were not compiled separately. If a task's build fails on an import, compare the file with the prototype-verified snippets in later tasks. The GitHub Actions workflows were not run.

## Global Constraints

- Ruby namespace is `Mechanomeld`; library errors are `Mechanomeld::Error < StandardError`.
- `required_ruby_version >= 3.2.0`. CI covers Ruby 3.2, 3.3, 3.4, 4.0.
- Crates: `automerge = "0.11"`, `magnus = "0.8"`, `rb-sys = "0.9.130"`. Gems: `rb_sys ~> 0.9.130` (runtime), `rake-compiler ~> 1.3`, `reissue ~> 0.5`, `minitest ~> 5.16`. JS: `@automerge/automerge` exactly `3.4.1`.
- Text objects read as `Mechanomeld::Text`; Automerge scalar strings read as `String`. Writing a Ruby `String` stores a scalar string; `Mechanomeld::Text` creates a text object.
- `Timestamp` values are **milliseconds**; `commit(timestamp:)` is **seconds**. Both follow Automerge — do not "fix" the difference.
- Documents use `TextEncoding::UnicodeCodePoint` so text lengths match Ruby `String#length`.
- Scalar wrappers are built in Rust by `obj_alloc` + `ivar_set("@value", ...)`, never by calling `new`; wrapper values are read with `ivar_get`, never by calling methods.
- magnus 0.8 cannot copy an `RArray` into `Vec<Value>` (`to_vec::<Value>()` fails to compile: `Value: TryConvertOwned` not satisfied). Read elements with `array.entry::<Value>(index as isize)`.
- Test files follow the skeleton convention: `test/test_*.rb`.
- Tools come from `mise.toml`. Run `mise trust` once in the repo before any `mise run`.
- Never run `rake build` or `rake release` locally: reissue hooks `build` to bump the version and commit. Verify packaging with `gem build mechanomeld.gemspec`.
- The gemspec lists files with `git ls-files`: `git add` new files before `gem build`.
- GitHub home: `https://github.com/SOFware/mechanomeld`. Release commits use `SOFware <gems@sofwarellc.com>`.
- Commits that change user-facing behavior carry reissue changelog trailers (`Added:`, `Changed:`, `Fixed:`, ...) in the final trailer block, alongside `Co-Authored-By`.
- Work on a feature branch (`automerge-extension`), not `main`.

## File Map

```
mise.toml                              tools + tasks (Task 1, extended in Tasks 3, 7)
Cargo.toml                             Cargo workspace (Task 1)
Cargo.lock                             generated, committed (Task 1)
ext/mechanomeld/Cargo.toml             crate manifest (Task 1)
ext/mechanomeld/extconf.rb             rb_sys makefile (Task 1)
ext/mechanomeld/src/lib.rs             method registration (Tasks 1, 3, 4, 5, 6)
ext/mechanomeld/src/classes.rs         lazy Ruby class lookups (Task 3)
ext/mechanomeld/src/errors.rs          Ruby exception constructors (Task 3)
ext/mechanomeld/src/path.rs            path segments -> ObjId/Prop (Tasks 3, 4, 5)
ext/mechanomeld/src/read.rs            automerge -> Ruby values (Task 3)
ext/mechanomeld/src/write.rs           Ruby values -> automerge ops (Task 5)
ext/mechanomeld/src/document.rs        Mechanomeld::Document native methods (Tasks 3, 4, 5, 6)
lib/mechanomeld.rb                     load order (Tasks 1, 2, 3)
lib/mechanomeld/error.rb               Mechanomeld::Error (Task 1)
lib/mechanomeld/scalars.rb             Scalar, Counter, Timestamp, Uint, Bytes, Text (Task 2)
lib/mechanomeld/document.rb            Ruby conveniences (Tasks 3, 5, 6)
test/test_helper.rb                    fixture loader (Task 3)
test/test_mechanomeld.rb               (Task 1)
test/test_scalars.rb                   (Task 2)
test/test_document_read.rb             (Task 3)
test/test_document_keys.rb             (Task 4)
test/test_document_write.rb            (Task 5)
test/test_document_change.rb           (Task 6)
test/fixtures/package.json             JS dependency pin (Task 3)
test/fixtures/package-lock.json        generated, committed (Task 3)
test/fixtures/generate.mjs             writes *.automerge fixtures (Task 3)
test/fixtures/types.automerge          generated, committed (Task 3)
test/fixtures/conflict.automerge       generated, committed (Task 3)
test/fixtures/interop.mjs              JS reads a Ruby-written doc (Task 7)
test/interop/write_fixtures.rb         Ruby writes a doc for JS (Task 7)
.github/workflows/test.yml             CI (Task 8)
.github/workflows/release.yml          releases (Task 9)
CHANGELOG.md                           reissue (Task 9)
Gemfile, mechanomeld.gemspec, Rakefile, .gitignore, README.md   (Tasks 1, 8, 9)
```

---

### Task 1: Toolchain and native extension skeleton

Replaces the skeleton's failing placeholder test with a test that proves the compiled extension loads.

**Files:**
- Create: `mise.toml`, `Cargo.toml`, `ext/mechanomeld/Cargo.toml`, `ext/mechanomeld/extconf.rb`, `ext/mechanomeld/src/lib.rs`, `lib/mechanomeld/error.rb`
- Modify: `Gemfile`, `mechanomeld.gemspec`, `Rakefile`, `.gitignore`, `lib/mechanomeld.rb`
- Test: `test/test_mechanomeld.rb`

**Interfaces:**
- Produces: `Mechanomeld::Error < StandardError`; class `Mechanomeld::Document` defined by the native extension; `lib/mechanomeld/mechanomeld.bundle` (or `.so`) built by `bundle exec rake compile`; `rake test` depends on `compile`; mise tasks `setup`, `compile`, `test`.

- [ ] **Step 1: Create a feature branch**

```bash
git switch -c automerge-extension
```

- [ ] **Step 2: Pin tools with mise**

Create `mise.toml`:

```toml
[tools]
ruby = "4.0"
rust = "1.90"
node = "24"

[tasks.setup]
run = "bundle install"

[tasks.compile]
run = "bundle exec rake compile"

[tasks.test]
run = "bundle exec rake test"
```

Run: `mise trust && mise install`
Expected: ruby 4.0.x, rust 1.90.x, node 24.x reported installed (or already installed).

- [ ] **Step 3: Write the failing test**

Replace `test/test_mechanomeld.rb`:

```ruby
# frozen_string_literal: true

require "test_helper"

class TestMechanomeld < Minitest::Test
  def test_that_it_has_a_version_number
    refute_nil ::Mechanomeld::VERSION
  end

  def test_native_extension_defines_document
    assert_kind_of Class, Mechanomeld::Document
  end
end
```

- [ ] **Step 4: Run it to verify it fails**

Run: `bundle exec rake test`
Expected: 1 error, `NameError: uninitialized constant Mechanomeld::Document`.

- [ ] **Step 5: Add build dependencies**

Replace `Gemfile`:

```ruby
# frozen_string_literal: true

source "https://rubygems.org"

# Specify your gem's dependencies in mechanomeld.gemspec
gemspec

gem "irb"
gem "rake", "~> 13.0"
gem "rake-compiler", "~> 1.3"

gem "minitest", "~> 5.16"
```

Replace `mechanomeld.gemspec`:

```ruby
# frozen_string_literal: true

require_relative "lib/mechanomeld/version"

Gem::Specification.new do |spec|
  spec.name = "mechanomeld"
  spec.version = Mechanomeld::VERSION
  spec.authors = ["Jim Gay"]
  spec.email = ["jim@saturnflyer.com"]

  spec.summary = "Read and write Automerge documents from Ruby"
  spec.description = "A native extension wrapping the Rust automerge crate so Ruby can load, read, create, and save Automerge documents shared with JavaScript."
  spec.homepage = "https://github.com/SOFware/mechanomeld"
  spec.required_ruby_version = ">= 3.2.0"
  spec.metadata["allowed_push_host"] = "https://rubygems.org"
  spec.metadata["homepage_uri"] = spec.homepage
  spec.metadata["source_code_uri"] = spec.homepage
  spec.metadata["changelog_uri"] = "#{spec.homepage}/blob/main/CHANGELOG.md"

  # Specify which files should be added to the gem when it is released.
  # The `git ls-files -z` loads the files in the RubyGem that have been added into git.
  gemspec = File.basename(__FILE__)
  spec.files = IO.popen(%w[git ls-files -z], chdir: __dir__, err: IO::NULL) do |ls|
    ls.readlines("\x0", chomp: true).reject do |f|
      (f == gemspec) ||
        f.start_with?(*%w[bin/ Gemfile .gitignore test/ docs/ .github/ mise.toml])
    end
  end
  spec.bindir = "exe"
  spec.executables = spec.files.grep(%r{\Aexe/}) { |f| File.basename(f) }
  spec.require_paths = ["lib"]
  spec.extensions = ["ext/mechanomeld/extconf.rb"]

  spec.add_dependency "rb_sys", "~> 0.9.130"
end
```

Replace `Rakefile`:

```ruby
# frozen_string_literal: true

require "bundler/gem_tasks"
require "minitest/test_task"
require "rb_sys/extensiontask"

GEMSPEC = Gem::Specification.load("mechanomeld.gemspec")

# No platform list: cross-gem builds set RUBY_TARGET, which rb_sys reads.
RbSys::ExtensionTask.new("mechanomeld", GEMSPEC) do |ext|
  ext.lib_dir = "lib/mechanomeld"
end

Minitest::TestTask.create
task test: :compile

task default: :test
```

Append to `.gitignore`:

```
/target/
/lib/mechanomeld/*.bundle
/lib/mechanomeld/*.so
/lib/mechanomeld/[0-9]*/
/test/fixtures/node_modules/
/test/interop/out/
*.gem
```

- [ ] **Step 6: Create the Rust crate**

Create `Cargo.toml`:

```toml
[workspace]
members = ["ext/mechanomeld"]
resolver = "2"
```

Create `ext/mechanomeld/Cargo.toml`:

```toml
[package]
name = "mechanomeld"
version = "0.1.0"
edition = "2021"
publish = false

[lib]
crate-type = ["cdylib"]

[dependencies]
automerge = "0.11"
magnus = "0.8"
rb-sys = "0.9.130"
```

Create `ext/mechanomeld/extconf.rb`:

```ruby
# frozen_string_literal: true

require "mkmf"
require "rb_sys/mkmf"

create_rust_makefile("mechanomeld/mechanomeld")
```

Create `ext/mechanomeld/src/lib.rs`:

```rust
use magnus::{prelude::*, Error, Ruby};

#[magnus::init]
fn init(ruby: &Ruby) -> Result<(), Error> {
    let module = ruby.define_module("Mechanomeld")?;
    module.define_class("Document", ruby.class_object())?;
    Ok(())
}
```

- [ ] **Step 7: Load the extension from Ruby**

Create `lib/mechanomeld/error.rb`:

```ruby
# frozen_string_literal: true

module Mechanomeld
  class Error < StandardError; end
end
```

Replace `lib/mechanomeld.rb`:

```ruby
# frozen_string_literal: true

require_relative "mechanomeld/version"
require_relative "mechanomeld/error"

# Precompiled gems ship one binary per Ruby minor version; source builds put it in lib/mechanomeld.
begin
  RUBY_VERSION =~ /(\d+\.\d+)/
  require_relative "mechanomeld/#{Regexp.last_match(1)}/mechanomeld"
rescue LoadError
  require_relative "mechanomeld/mechanomeld"
end
```

- [ ] **Step 8: Run the tests to verify they pass**

Run: `mise run setup && mise run test`
Expected: the extension compiles (first build downloads crates, a few minutes), then `2 runs, 2 assertions, 0 failures, 0 errors`. A `Cargo.lock` appears at the repo root.

- [ ] **Step 9: Verify the gem packages the extension sources**

```bash
git add -A
gem build mechanomeld.gemspec
tar -xOf mechanomeld-0.1.0.gem metadata.gz | gunzip | grep -A 12 '^files:'
rm mechanomeld-0.1.0.gem
```

Expected: `Successfully built RubyGem`; `files:` lists `Cargo.lock`, `Cargo.toml`, `ext/mechanomeld/Cargo.toml`, `ext/mechanomeld/extconf.rb`, `ext/mechanomeld/src/lib.rs`, `lib/mechanomeld.rb`, `lib/mechanomeld/error.rb`. No `mise.toml`, `docs/`, or `test/`.

- [ ] **Step 10: Commit**

```bash
git add -A
git commit -F - <<'EOF'
Build a Rust native extension with magnus and rb_sys

Co-Authored-By: Claude Opus 5 (1M context) <noreply@anthropic.com>
EOF
```

---

### Task 2: Scalar value classes

**Files:**
- Create: `lib/mechanomeld/scalars.rb`
- Modify: `lib/mechanomeld.rb`
- Test: `test/test_scalars.rb`

**Interfaces:**
- Consumes: `lib/mechanomeld.rb` load order from Task 1.
- Produces: `Mechanomeld::Scalar#value`, `#==`, `#eql?`, `#hash`, `#inspect`; subclasses `Counter`, `Timestamp`, `Uint`, `Bytes` (value coerced with `String(value).b`), `Text` (value coerced with `String(value)`, default `""`, `#to_s`). Every instance stores its value in `@value` — Rust reads and writes that ivar directly.

- [ ] **Step 1: Write the failing test**

Create `test/test_scalars.rb`:

```ruby
# frozen_string_literal: true

require "test_helper"

class TestScalars < Minitest::Test
  def test_same_class_and_value_are_equal
    assert_equal Mechanomeld::Counter.new(1), Mechanomeld::Counter.new(1)
  end

  def test_different_classes_are_not_equal
    refute_equal Mechanomeld::Counter.new(1), Mechanomeld::Timestamp.new(1)
  end

  def test_equal_scalars_are_the_same_hash_key
    assert_equal 1, [Mechanomeld::Uint.new(1), Mechanomeld::Uint.new(1)].uniq.size
  end

  def test_bytes_value_is_binary
    assert_equal Encoding::BINARY, Mechanomeld::Bytes.new("abc").value.encoding
  end

  def test_text_defaults_to_empty_string
    assert_equal "", Mechanomeld::Text.new.value
  end

  def test_text_to_s_is_its_value
    assert_equal "hi", Mechanomeld::Text.new("hi").to_s
  end

  def test_inspect_shows_class_and_value
    assert_equal "#<Mechanomeld::Counter 3>", Mechanomeld::Counter.new(3).inspect
  end
end
```

- [ ] **Step 2: Run it to verify it fails**

Run: `mise run test`
Expected: 7 errors, `NameError: uninitialized constant Mechanomeld::Counter` (and similar).

- [ ] **Step 3: Implement the classes**

Create `lib/mechanomeld/scalars.rb`:

```ruby
# frozen_string_literal: true

module Mechanomeld
  # A typed Automerge value that has no natural Ruby equivalent.
  class Scalar
    attr_reader :value

    def initialize(value)
      @value = value
    end

    def ==(other)
      other.class == self.class && other.value == value
    end
    alias_method :eql?, :==

    def hash
      [self.class, value].hash
    end

    def inspect
      "#<#{self.class.name} #{value.inspect}>"
    end
  end

  class Counter < Scalar; end

  # Milliseconds since the Unix epoch.
  class Timestamp < Scalar; end

  class Uint < Scalar; end

  class Bytes < Scalar
    def initialize(value)
      super(String(value).b)
    end
  end

  # A collaborative text object, as opposed to a plain String value.
  class Text < Scalar
    def initialize(value = "")
      super(String(value))
    end

    def to_s
      value
    end
  end
end
```

In `lib/mechanomeld.rb`, add the require after `require_relative "mechanomeld/error"`. The scalars must load **before** the native extension:

```ruby
require_relative "mechanomeld/scalars"
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `mise run test`
Expected: `9 runs, 9 assertions, 0 failures, 0 errors`.

- [ ] **Step 5: Commit**

```bash
git add lib/mechanomeld/scalars.rb lib/mechanomeld.rb test/test_scalars.rb
git commit -F - <<'EOF'
Add Automerge scalar value classes

Added: Mechanomeld::Counter, Timestamp, Uint, Bytes, and Text value classes
Co-Authored-By: Claude Opus 5 (1M context) <noreply@anthropic.com>
EOF
```

---

### Task 3: Load and read documents

**Files:**
- Create: `test/fixtures/package.json`, `test/fixtures/generate.mjs`, `ext/mechanomeld/src/classes.rs`, `ext/mechanomeld/src/errors.rs`, `ext/mechanomeld/src/path.rs`, `ext/mechanomeld/src/read.rs`, `ext/mechanomeld/src/document.rs`, `lib/mechanomeld/document.rb`
- Generate and commit: `test/fixtures/package-lock.json`, `test/fixtures/types.automerge`, `test/fixtures/conflict.automerge`
- Modify: `ext/mechanomeld/src/lib.rs`, `lib/mechanomeld.rb`, `test/test_helper.rb`, `mise.toml`
- Test: `test/test_document_read.rb`

**Interfaces:**
- Consumes: scalar classes and `@value` convention (Task 2); `Mechanomeld::Error` (Task 1).
- Produces:
  - Ruby: `Mechanomeld::Document.load(String) -> Document`, `#get(path) -> value`, `#[](path)`, `#to_h`, `#to_hash`.
  - Rust `errors`: `error(ruby, msg) -> Error` (Mechanomeld::Error), `automerge_error(ruby, impl Display)`, `type_error(ruby, msg)`, `arg_error(ruby, msg)`.
  - Rust `classes`: `static MECHANOMELD, ERROR, TEXT, COUNTER, TIMESTAMP, UINT, BYTES: Lazy<...>`.
  - Rust `path`: `segments(Value) -> Result<Vec<Value>>`, `key(&Ruby, Value) -> Result<String>`, `index(&Ruby, Value) -> Result<usize>`, `class_name(Value) -> String`, `prop(&Ruby, ObjType, Value) -> Result<Prop>`, `resolve(&Ruby, &AutoCommit, &[Value]) -> Result<Option<(ObjId, ObjType)>>`.
  - Rust `read`: `value(&Ruby, &AutoCommit, AmValue, ObjId) -> Result<Value>`, `object(&Ruby, &AutoCommit, &ObjId, ObjType) -> Result<Value>`, `scalar(&Ruby, &ScalarValue) -> Result<Value>`.
  - Rust `document`: `struct Document { inner: RefCell<AutoCommit> }`, `const ENCODING`, `Document::doc(&self, &Ruby) -> Result<Ref<AutoCommit>>`.
  - Test helper: `fixture(name) -> String` (binary bytes of `test/fixtures/<name>.automerge`).
  - mise task `fixtures`; `setup` also runs `npm ci --prefix test/fixtures`.

- [ ] **Step 1: Generate JS fixtures**

Create `test/fixtures/package.json`:

```json
{
  "name": "mechanomeld-fixtures",
  "private": true,
  "dependencies": {
    "@automerge/automerge": "3.4.1"
  }
}
```

Create `test/fixtures/generate.mjs`:

```js
// Writes the .automerge fixtures read by the Ruby tests. Run with `mise run fixtures`.
import * as A from "@automerge/automerge";
import { writeFileSync } from "node:fs";

const save = (name, doc) =>
  writeFileSync(new URL(`${name}.automerge`, import.meta.url), A.save(doc));

const types = A.change(A.init(), (doc) => {
  doc.text = "héllo 😀";
  doc.string = new A.ImmutableString("fixed");
  doc.int = 42;
  doc.negative = new A.Int(-3);
  doc.float = 1.5;
  doc.whole_float = new A.Float64(2.0);
  doc.bool_true = true;
  doc.bool_false = false;
  doc.nothing = null;
  doc.counter = new A.Counter(5);
  doc.timestamp = new Date(1700000000123);
  doc.uint = new A.Uint(7);
  doc.bytes = new Uint8Array([0, 1, 255]);
  doc.nested = { list: [1, "two", { three: 3 }], empty_map: {}, empty_list: [] };
});
save("types", types);

// Two actors set the same key concurrently; actor "bbbb" wins.
let a = A.from({ k: new A.ImmutableString("base") }, { actor: "aaaa" });
let b = A.clone(a, { actor: "bbbb" });
a = A.change(a, (doc) => {
  doc.k = new A.ImmutableString("from-a");
});
b = A.change(b, (doc) => {
  doc.k = new A.ImmutableString("from-b");
});
save("conflict", A.merge(a, b));
```

Note: `Uint`, `Int`, and `Float64` must be assigned inside `A.change`. Passed to `A.from` they are stored as maps (`{value: 7}`).

In `mise.toml`, replace the `setup` task and add `fixtures`:

```toml
[tasks.setup]
run = ["bundle install", "npm ci --prefix test/fixtures"]

[tasks.fixtures]
run = "node test/fixtures/generate.mjs"
```

Run:

```bash
npm install --prefix test/fixtures
mise run fixtures
ls -l test/fixtures/*.automerge
```

Expected: `test/fixtures/package-lock.json` created; `types.automerge` (~450 bytes) and `conflict.automerge` (~200 bytes) exist.

- [ ] **Step 2: Add the fixture helper**

Replace `test/test_helper.rb`:

```ruby
# frozen_string_literal: true

$LOAD_PATH.unshift File.expand_path("../lib", __dir__)
require "mechanomeld"

require "minitest/autorun"

module FixtureHelper
  # Bytes of a document written by test/fixtures/generate.mjs.
  def fixture(name)
    File.binread(File.expand_path("fixtures/#{name}.automerge", __dir__))
  end
end

Minitest::Test.include(FixtureHelper)
```

- [ ] **Step 3: Write the failing test**

Create `test/test_document_read.rb`:

```ruby
# frozen_string_literal: true

require "test_helper"

class TestDocumentRead < Minitest::Test
  def setup
    @doc = Mechanomeld::Document.load(fixture("types"))
  end

  def test_load_rejects_invalid_bytes
    error = assert_raises(Mechanomeld::Error) { Mechanomeld::Document.load("not automerge".b) }
    assert_match(/could not load Automerge document/, error.message)
  end

  def test_js_strings_read_as_text
    assert_equal Mechanomeld::Text.new("héllo 😀"), @doc.get("text")
  end

  def test_immutable_strings_read_as_utf8_strings
    value = @doc.get("string")
    assert_equal "fixed", value
    assert_equal Encoding::UTF_8, value.encoding
  end

  def test_integers
    assert_equal 42, @doc.get("int")
    assert_equal(-3, @doc.get("negative"))
  end

  def test_floats
    assert_equal 1.5, @doc.get("float")
    assert_kind_of Float, @doc.get("whole_float")
  end

  def test_booleans_and_null
    assert_equal true, @doc.get("bool_true")
    assert_equal false, @doc.get("bool_false")
    assert_nil @doc.get("nothing")
  end

  def test_counter
    assert_equal Mechanomeld::Counter.new(5), @doc.get("counter")
  end

  def test_timestamp_is_milliseconds
    assert_equal Mechanomeld::Timestamp.new(1_700_000_000_123), @doc.get("timestamp")
  end

  def test_uint
    assert_equal Mechanomeld::Uint.new(7), @doc.get("uint")
  end

  def test_bytes_are_binary
    value = @doc.get("bytes")
    assert_equal Mechanomeld::Bytes.new("\x00\x01\xFF"), value
    assert_equal Encoding::BINARY, value.value.encoding
  end

  def test_nested_path
    assert_equal 3, @doc.get(["nested", "list", 2, "three"])
  end

  def test_symbol_segments
    assert_equal 3, @doc.get([:nested, :list, 2, :three])
  end

  def test_maps_read_as_hashes_with_string_keys
    expected = {
      "list" => [1, Mechanomeld::Text.new("two"), {"three" => 3}],
      "empty_map" => {},
      "empty_list" => []
    }
    assert_equal expected, @doc.get("nested")
  end

  def test_missing_keys_and_indexes_are_nil
    assert_nil @doc.get("missing")
    assert_nil @doc.get(["missing", "deeper"])
    assert_nil @doc.get(["nested", "list", 99])
  end

  def test_brackets_read_a_path
    assert_equal 1, @doc[["nested", "list", 0]]
  end

  def test_to_h_is_the_whole_document
    expected = {
      "text" => Mechanomeld::Text.new("héllo 😀"),
      "string" => "fixed",
      "int" => 42,
      "negative" => -3,
      "float" => 1.5,
      "whole_float" => 2.0,
      "bool_true" => true,
      "bool_false" => false,
      "nothing" => nil,
      "counter" => Mechanomeld::Counter.new(5),
      "timestamp" => Mechanomeld::Timestamp.new(1_700_000_000_123),
      "uint" => Mechanomeld::Uint.new(7),
      "bytes" => Mechanomeld::Bytes.new("\x00\x01\xFF"),
      "nested" => {
        "list" => [1, Mechanomeld::Text.new("two"), {"three" => 3}],
        "empty_map" => {},
        "empty_list" => []
      }
    }
    assert_equal expected, @doc.to_h
  end

  def test_string_segment_into_list_raises_type_error
    assert_raises(TypeError) { @doc.get(["nested", "list", "0"]) }
  end

  def test_integer_segment_into_map_raises_type_error
    assert_raises(TypeError) { @doc.get(["nested", 0]) }
  end

  def test_negative_index_raises_argument_error
    assert_raises(ArgumentError) { @doc.get(["nested", "list", -1]) }
  end

  def test_descending_into_a_scalar_raises
    assert_raises(Mechanomeld::Error) { @doc.get(["int", "deeper"]) }
  end

  def test_descending_into_text_raises
    assert_raises(Mechanomeld::Error) { @doc.get(["text", 0]) }
  end

  def test_conflicting_values_read_the_winner
    doc = Mechanomeld::Document.load(fixture("conflict"))
    assert_equal "from-b", doc.get("k")
  end
end
```

- [ ] **Step 4: Run it to verify it fails**

Run: `mise run test`
Expected: 22 errors, `NoMethodError: undefined method 'load' for class Mechanomeld::Document`.

- [ ] **Step 5: Add class lookups and error constructors**

Create `ext/mechanomeld/src/classes.rs`:

```rust
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
```

Create `ext/mechanomeld/src/errors.rs`:

```rust
use std::borrow::Cow;
use std::fmt::Display;

use magnus::{Error, Ruby};

use crate::classes::ERROR;

/// A `Mechanomeld::Error`.
pub fn error<T: Into<Cow<'static, str>>>(ruby: &Ruby, message: T) -> Error {
    Error::new(ruby.get_inner(&ERROR), message)
}

/// A `Mechanomeld::Error` carrying an automerge error's message.
pub fn automerge_error(ruby: &Ruby, err: impl Display) -> Error {
    error(ruby, err.to_string())
}

pub fn type_error<T: Into<Cow<'static, str>>>(ruby: &Ruby, message: T) -> Error {
    Error::new(ruby.exception_type_error(), message)
}

pub fn arg_error<T: Into<Cow<'static, str>>>(ruby: &Ruby, message: T) -> Error {
    Error::new(ruby.exception_arg_error(), message)
}
```

- [ ] **Step 6: Add path resolution**

Create `ext/mechanomeld/src/path.rs`:

```rust
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
```

- [ ] **Step 7: Add value conversion**

Create `ext/mechanomeld/src/read.rs`:

```rust
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
```

- [ ] **Step 8: Add the native Document**

Create `ext/mechanomeld/src/document.rs`:

```rust
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
```

Replace `ext/mechanomeld/src/lib.rs`:

```rust
mod classes;
mod document;
mod errors;
mod path;
mod read;

use magnus::{function, method, prelude::*, Error, Ruby};

use crate::document::Document;

#[magnus::init]
fn init(ruby: &Ruby) -> Result<(), Error> {
    let module = ruby.define_module("Mechanomeld")?;
    let class = module.define_class("Document", ruby.class_object())?;
    class.define_singleton_method("load", function!(Document::load, 1))?;
    class.define_method("get", method!(Document::get, 1))?;
    Ok(())
}
```

- [ ] **Step 9: Add Ruby conveniences**

Create `lib/mechanomeld/document.rb`:

```ruby
# frozen_string_literal: true

module Mechanomeld
  # Native methods are defined in ext/mechanomeld/src/document.rs.
  class Document
    def [](path)
      get(path)
    end

    def to_h
      get([])
    end
    alias_method :to_hash, :to_h
  end
end
```

Append to the end of `lib/mechanomeld.rb` (after the native `begin`/`rescue` block — it reopens the natively defined class):

```ruby

require_relative "mechanomeld/document"
```

- [ ] **Step 10: Run the tests to verify they pass**

Run: `mise run test`
Expected: compiles with no warnings; `31 runs, ... 0 failures, 0 errors`.

Run: `cargo clippy --release && cargo fmt --check`
Expected: no warnings, no diff.

- [ ] **Step 11: Commit**

```bash
git add -A
git commit -F - <<'EOF'
Load Automerge documents and read values

Added: Mechanomeld::Document.load reads documents saved by JavaScript Automerge
Added: Document#get, #[], and #to_h read values by path
Co-Authored-By: Claude Opus 5 (1M context) <noreply@anthropic.com>
EOF
```

---

### Task 4: Keys and length

**Files:**
- Modify: `ext/mechanomeld/src/path.rs`, `ext/mechanomeld/src/document.rs`, `ext/mechanomeld/src/lib.rs`
- Test: `test/test_document_keys.rb`

**Interfaces:**
- Consumes: `path::resolve`, `Document::doc` (Task 3).
- Produces: Ruby `#keys(path = []) -> Array<String>`, `#length(path = []) -> Integer`. Rust `path::resolve_existing(&Ruby, &AutoCommit, &[Value]) -> Result<(ObjId, ObjType)>` (missing path raises `Mechanomeld::Error "path does not exist"`); private `document::optional_path(&Ruby, &[Value]) -> Result<Vec<Value>>`.

- [ ] **Step 1: Write the failing test**

Create `test/test_document_keys.rb`:

```ruby
# frozen_string_literal: true

require "test_helper"

class TestDocumentKeys < Minitest::Test
  def setup
    @doc = Mechanomeld::Document.load(fixture("types"))
  end

  def test_keys_of_root_are_sorted_strings
    expected = %w[bool_false bool_true bytes counter float int negative nested nothing string text timestamp uint whole_float]
    assert_equal expected, @doc.keys
  end

  def test_keys_at_a_path
    assert_equal %w[empty_list empty_map list], @doc.keys("nested")
  end

  def test_keys_of_a_list_raise
    assert_raises(Mechanomeld::Error) { @doc.keys(["nested", "list"]) }
  end

  def test_keys_of_a_missing_path_raise
    assert_raises(Mechanomeld::Error) { @doc.keys("missing") }
  end

  def test_length_of_map
    assert_equal 14, @doc.length
  end

  def test_length_of_list
    assert_equal 3, @doc.length(["nested", "list"])
  end

  def test_length_of_text_counts_code_points
    assert_equal 7, @doc.length("text")
  end
end
```

- [ ] **Step 2: Run it to verify it fails**

Run: `mise run test`
Expected: 7 errors, `NoMethodError: undefined method 'keys'` / `'length'`.

- [ ] **Step 3: Implement**

Append to `ext/mechanomeld/src/path.rs`:

```rust

/// Like [`resolve`], but a missing key or index is an error.
pub fn resolve_existing(
    ruby: &Ruby,
    doc: &AutoCommit,
    segments: &[Value],
) -> Result<(ObjId, ObjType), Error> {
    resolve(ruby, doc, segments)?.ok_or_else(|| error(ruby, "path does not exist"))
}
```

In `ext/mechanomeld/src/document.rs`, replace the `use magnus::...` line with:

```rust
use magnus::{prelude::*, scan_args::scan_args, Error, RArray, RString, Ruby, Value};
```

Add these methods inside `impl Document`, after `get`:

```rust

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
```

Append to the end of `ext/mechanomeld/src/document.rs` (outside `impl`):

```rust

fn optional_path(ruby: &Ruby, args: &[Value]) -> Result<Vec<Value>, Error> {
    let args = scan_args::<(), (Option<Value>,), (), (), (), ()>(args)?;
    path::segments(args.optional.0.unwrap_or_else(|| ruby.qnil().as_value()))
}
```

In `ext/mechanomeld/src/lib.rs`, register after `get`:

```rust
    class.define_method("keys", method!(Document::keys, -1))?;
    class.define_method("length", method!(Document::length, -1))?;
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `mise run test && cargo clippy --release && cargo fmt --check`
Expected: `38 runs, ... 0 failures, 0 errors`; no clippy warnings; no fmt diff.

- [ ] **Step 5: Commit**

```bash
git add -A
git commit -F - <<'EOF'
List map keys and object lengths

Added: Document#keys and #length for maps, lists, and text
Co-Authored-By: Claude Opus 5 (1M context) <noreply@anthropic.com>
EOF
```

---

### Task 5: Create documents, put, delete, and save

**Files:**
- Create: `ext/mechanomeld/src/write.rs`
- Modify: `ext/mechanomeld/src/path.rs`, `ext/mechanomeld/src/document.rs`, `ext/mechanomeld/src/lib.rs`, `lib/mechanomeld/document.rb`
- Test: `test/test_document_write.rb`

**Interfaces:**
- Consumes: `path::{segments, key, index, prop, resolve_existing, class_name}`, classes, errors (Tasks 3–4).
- Produces: Ruby `Document.new(actor_id: nil)`, `#put(path, value) -> self`, `#[]=(path, value)`, `#delete(path) -> self`, `#save -> String (BINARY)`. Rust `path::write_prop(&Ruby, &AutoCommit, &ObjId, ObjType, Value) -> Result<Prop>` (list index must be `< length`); `write::Slot { Put(Prop), Insert(usize) }`; `write::write(&Ruby, &mut AutoCommit, &ObjId, Slot, Value) -> Result<()>`; `Document::doc_mut(&self, &Ruby) -> Result<RefMut<AutoCommit>>`.

- [ ] **Step 1: Write the failing test**

Create `test/test_document_write.rb`:

```ruby
# frozen_string_literal: true

require "test_helper"

class TestDocumentWrite < Minitest::Test
  SCALARS = {
    "true" => true,
    "false" => false,
    "int" => -3,
    "float" => 2.5,
    "string" => "plain",
    "counter" => Mechanomeld::Counter.new(5),
    "timestamp" => Mechanomeld::Timestamp.new(1_700_000_000_123),
    "uint" => Mechanomeld::Uint.new(7),
    "bytes" => Mechanomeld::Bytes.new("\x00\xFF")
  }.freeze

  def setup
    @doc = Mechanomeld::Document.new
  end

  def test_new_document_is_empty
    assert_equal({}, @doc.to_h)
  end

  def test_new_accepts_a_hex_actor_id
    assert_instance_of Mechanomeld::Document, Mechanomeld::Document.new(actor_id: "0123456789abcdef")
  end

  def test_new_rejects_an_invalid_actor_id
    assert_raises(Mechanomeld::Error) { Mechanomeld::Document.new(actor_id: "not hex") }
  end

  def test_put_scalars_reads_them_back
    SCALARS.each { |key, value| @doc.put(key, value) }
    SCALARS.each { |key, value| assert_equal value, @doc.get(key), key }
  end

  def test_put_nil
    @doc.put("nothing", nil)
    assert_includes @doc.keys, "nothing"
    assert_nil @doc.get("nothing")
  end

  def test_put_string_stays_a_string
    @doc.put("s", "plain")
    assert_instance_of String, @doc.get("s")
  end

  def test_put_text_creates_a_text_object
    @doc.put("t", Mechanomeld::Text.new("héllo"))
    assert_equal Mechanomeld::Text.new("héllo"), @doc.get("t")
    assert_equal 5, @doc.length("t")
  end

  def test_put_returns_self
    assert_same @doc, @doc.put("k", 1)
  end

  def test_bracket_assignment
    @doc["k"] = 1
    assert_equal 1, @doc["k"]
  end

  def test_put_nested_hashes_and_arrays
    @doc.put("nested", {"list" => [1, "two", {three: 3}], "empty" => {}})
    assert_equal({"list" => [1, "two", {"three" => 3}], "empty" => {}}, @doc.get("nested"))
  end

  def test_put_overwrites_an_existing_list_index
    @doc.put("list", [1, 2, 3])
    @doc.put(["list", 1], "b")
    assert_equal [1, "b", 3], @doc.get("list")
  end

  def test_put_past_the_end_of_a_list_raises
    @doc.put("list", [1, 2, 3])
    assert_raises(Mechanomeld::Error) { @doc.put(["list", 3], 4) }
  end

  def test_put_into_a_missing_parent_raises
    assert_raises(Mechanomeld::Error) { @doc.put(["missing", "k"], 1) }
  end

  def test_put_with_an_empty_path_raises_argument_error
    assert_raises(ArgumentError) { @doc.put([], 1) }
  end

  def test_integer_outside_i64_raises_range_error
    assert_raises(RangeError) { @doc.put("big", 2**64) }
  end

  def test_negative_uint_raises_range_error
    assert_raises(RangeError) { @doc.put("u", Mechanomeld::Uint.new(-1)) }
  end

  def test_symbol_value_raises_type_error
    assert_raises(TypeError) { @doc.put("s", :symbol) }
  end

  def test_unsupported_object_raises_type_error
    assert_raises(TypeError) { @doc.put("o", Object.new) }
  end

  def test_invalid_utf8_string_raises_argument_error
    assert_raises(ArgumentError) { @doc.put("s", "\xFF".b) }
  end

  def test_non_string_hash_key_raises_type_error
    assert_raises(TypeError) { @doc.put("h", {1 => "a"}) }
  end

  def test_delete_a_map_key
    @doc.put("k", 1)
    assert_same @doc, @doc.delete("k")
    assert_equal({}, @doc.to_h)
  end

  def test_delete_a_list_index
    @doc.put("list", [1, 2, 3])
    @doc.delete(["list", 0])
    assert_equal [2, 3], @doc.get("list")
  end

  def test_save_returns_binary_that_loads_back
    @doc.put("nested", {"list" => [1, Mechanomeld::Text.new("two")]})
    @doc.put("count", Mechanomeld::Counter.new(1))
    bytes = @doc.save
    assert_equal Encoding::BINARY, bytes.encoding
    assert_equal @doc.to_h, Mechanomeld::Document.load(bytes).to_h
  end
end
```

- [ ] **Step 2: Run it to verify it fails**

Run: `mise run test`
Expected: 23 errors. Without a native `new`, Ruby's default `Class#new` builds a plain object, so the failures are `NoMethodError` (`put`, `delete`, `save`), `TypeError` (native `get` called on an object that wraps no document), and `ArgumentError` (`new(actor_id:)`).

- [ ] **Step 3: Add write-side path handling**

Append to `ext/mechanomeld/src/path.rs`:

```rust

/// The prop for writing `segment` into `obj`. List indexes must already exist.
pub fn write_prop(
    ruby: &Ruby,
    doc: &AutoCommit,
    obj: &ObjId,
    obj_type: ObjType,
    segment: Value,
) -> Result<Prop, Error> {
    if obj_type != ObjType::List {
        return prop(ruby, obj_type, segment);
    }
    let index = index(ruby, segment)?;
    let length = doc.length(obj);
    if index >= length {
        return Err(error(
            ruby,
            format!("list index {index} is out of bounds (length {length})"),
        ));
    }
    Ok(Prop::Seq(index))
}
```

- [ ] **Step 4: Add Ruby-to-Automerge conversion**

Create `ext/mechanomeld/src/write.rs`:

```rust
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
```

- [ ] **Step 5: Add the native write methods**

In `ext/mechanomeld/src/document.rs`, replace every `use` line at the top of the file with:

```rust
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
```

Inside `impl Document`, add after `fn doc`:

```rust

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
```

Inside `impl Document`, add after `length`:

```rust

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
```

Replace `ext/mechanomeld/src/lib.rs`:

```rust
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
```

In `lib/mechanomeld/document.rb`, add after `def [](path) ... end`:

```ruby

    def []=(path, value)
      put(path, value)
    end
```

- [ ] **Step 6: Run the tests to verify they pass**

Run: `mise run test && cargo clippy --release && cargo fmt --check`
Expected: `61 runs, ... 0 failures, 0 errors`; no clippy warnings; no fmt diff.

- [ ] **Step 7: Commit**

```bash
git add -A
git commit -F - <<'EOF'
Create documents, write values, and save

Added: Mechanomeld::Document.new(actor_id:) creates an empty document
Added: Document#put, #[]=, #delete, and #save
Co-Authored-By: Claude Opus 5 (1M context) <noreply@anthropic.com>
EOF
```

---

### Task 6: Commit, rollback, change, and from

**Files:**
- Modify: `ext/mechanomeld/src/document.rs`, `ext/mechanomeld/src/lib.rs`, `lib/mechanomeld/document.rb`
- Test: `test/test_document_change.rb`

**Interfaces:**
- Consumes: `Document::doc_mut`, `Document.new`, `#put` (Task 5).
- Produces: Ruby `#commit(message: nil, timestamp: nil) -> String (32 bytes, BINARY) | nil`, `#rollback -> Integer`, `#change(message: nil, timestamp: nil) { |doc| } -> self`, `Document.from(hash, actor_id: nil) -> Document`.

- [ ] **Step 1: Write the failing test**

Create `test/test_document_change.rb`:

```ruby
# frozen_string_literal: true

require "test_helper"

class TestDocumentChange < Minitest::Test
  def setup
    @doc = Mechanomeld::Document.new
  end

  def test_commit_returns_a_binary_change_hash
    @doc.put("k", 1)
    hash = @doc.commit(message: "set k", timestamp: 1_700_000_000)
    assert_equal 32, hash.bytesize
    assert_equal Encoding::BINARY, hash.encoding
  end

  def test_commit_with_nothing_pending_returns_nil
    assert_nil @doc.commit
  end

  def test_rollback_discards_pending_operations
    @doc.put("k", 1)
    @doc.put("j", 2)
    assert_equal 2, @doc.rollback
    assert_equal({}, @doc.to_h)
  end

  def test_change_commits_the_block
    @doc.change(message: "set k") { |doc| doc["k"] = 1 }
    assert_equal 1, @doc["k"]
    assert_nil @doc.commit
  end

  def test_change_returns_self
    assert_same @doc, @doc.change { |doc| doc["k"] = 1 }
  end

  def test_change_rolls_back_when_the_block_raises
    @doc.change { |doc| doc["kept"] = 1 }
    assert_raises(RuntimeError) do
      @doc.change do |doc|
        doc["lost"] = 2
        raise "boom"
      end
    end
    assert_equal({"kept" => 1}, @doc.to_h)
  end

  def test_from_builds_a_committed_document
    doc = Mechanomeld::Document.from({"a" => 1, :b => [true]})
    assert_equal({"a" => 1, "b" => [true]}, doc.to_h)
    assert_nil doc.commit
  end

  def test_from_accepts_an_actor_id
    doc = Mechanomeld::Document.from({"a" => 1}, actor_id: "0123456789abcdef")
    assert_equal({"a" => 1}, doc.to_h)
  end

  def test_from_rejects_a_non_hash
    assert_raises(ArgumentError) { Mechanomeld::Document.from([1]) }
  end
end
```

- [ ] **Step 2: Run it to verify it fails**

Run: `mise run test`
Expected: 9 errors, `NoMethodError: undefined method 'commit'` / `'change'` / `'from'`.

- [ ] **Step 3: Implement native commit and rollback**

In `ext/mechanomeld/src/document.rs`, change the automerge `use` to import `CommitOptions`:

```rust
use automerge::{
    transaction::{CommitOptions, Transactable},
    ActorId, AutoCommit, LoadOptions, ObjType, ReadDoc, TextEncoding, ROOT,
};
```

Inside `impl Document`, add before `save`:

```rust
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

```

In `ext/mechanomeld/src/lib.rs`, register after `delete`:

```rust
    class.define_method("commit", method!(Document::commit, -1))?;
    class.define_method("rollback", method!(Document::rollback, 0))?;
```

- [ ] **Step 4: Implement Ruby `from` and `change`**

Replace `lib/mechanomeld/document.rb`:

```ruby
# frozen_string_literal: true

module Mechanomeld
  # Native methods (new, load, get, keys, length, put, delete, commit, rollback, save)
  # are defined in ext/mechanomeld/src/document.rs.
  class Document
    def self.from(hash, actor_id: nil)
      raise ArgumentError, "Automerge document root must be a Hash" unless hash.is_a?(Hash)

      doc = new(actor_id: actor_id)
      hash.each { |key, value| doc.put([key], value) }
      doc.commit
      doc
    end

    def [](path)
      get(path)
    end

    def []=(path, value)
      put(path, value)
    end

    def to_h
      get([])
    end
    alias_method :to_hash, :to_h

    def change(message: nil, timestamp: nil)
      yield self
      commit(message: message, timestamp: timestamp)
      self
    rescue Exception
      rollback
      raise
    end
  end
end
```

- [ ] **Step 5: Run the tests to verify they pass**

Run: `mise run test && cargo clippy --release && cargo fmt --check`
Expected: `70 runs, 98 assertions, 0 failures, 0 errors`; no clippy warnings; no fmt diff.

- [ ] **Step 6: Commit**

```bash
git add -A
git commit -F - <<'EOF'
Commit, roll back, and build documents from hashes

Added: Document#commit(message:, timestamp:) and #rollback
Added: Document#change commits a block and rolls back if it raises
Added: Mechanomeld::Document.from builds a committed document from a Hash
Co-Authored-By: Claude Opus 5 (1M context) <noreply@anthropic.com>
EOF
```

---

### Task 7: JavaScript interop check

**Files:**
- Create: `test/fixtures/interop.mjs`, `test/interop/write_fixtures.rb`
- Modify: `mise.toml`

**Interfaces:**
- Consumes: `Document.from`, `#save`, scalar classes (Tasks 2–6); `@automerge/automerge` from `test/fixtures/node_modules` (Task 3).
- Produces: mise task `test:interop`, which prints `interop ok` on success and exits non-zero on any mismatch.

- [ ] **Step 1: Write the failing JS check**

Create `test/fixtures/interop.mjs`:

```js
// Loads the document written by test/interop/write_fixtures.rb and checks JS sees the right values.
import * as A from "@automerge/automerge";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

const doc = A.load(readFileSync(new URL("../interop/out/ruby.automerge", import.meta.url)));

assert.ok(doc.string instanceof A.ImmutableString, "Ruby String is an ImmutableString");
assert.equal(String(doc.string), "plain");
assert.equal(doc.text, "héllo 😀");
assert.equal(doc.int, -3);
assert.equal(doc.float, 2.5);
assert.equal(doc.bool, true);
assert.equal(doc.nothing, null);
assert.ok(doc.counter instanceof A.Counter, "Counter");
assert.equal(doc.counter.value, 5);
assert.ok(doc.timestamp instanceof Date, "Timestamp is a Date");
assert.equal(doc.timestamp.getTime(), 1700000000123);
assert.equal(doc.uint, 7n);
assert.deepEqual(Array.from(doc.bytes), [0, 1, 255]);
assert.equal(doc.nested.list[0], 1);
assert.equal(String(doc.nested.list[1]), "two");
assert.equal(doc.nested.list[2].three, 3);

console.log("interop ok");
```

- [ ] **Step 2: Run it to verify it fails**

Run: `node test/fixtures/interop.mjs`
Expected: `Error: ENOENT: no such file or directory` for `test/interop/out/ruby.automerge`.

- [ ] **Step 3: Write the Ruby side and the mise task**

Create `test/interop/write_fixtures.rb`:

```ruby
# frozen_string_literal: true

# Writes a Ruby-created document for test/fixtures/interop.mjs. Run with `mise run test:interop`.
$LOAD_PATH.unshift File.expand_path("../../lib", __dir__)
require "mechanomeld"
require "fileutils"

out = File.expand_path("out", __dir__)
FileUtils.mkdir_p(out)

doc = Mechanomeld::Document.from({
  "string" => "plain",
  "text" => Mechanomeld::Text.new("héllo 😀"),
  "int" => -3,
  "float" => 2.5,
  "bool" => true,
  "nothing" => nil,
  "counter" => Mechanomeld::Counter.new(5),
  "timestamp" => Mechanomeld::Timestamp.new(1_700_000_000_123),
  "uint" => Mechanomeld::Uint.new(7),
  "bytes" => Mechanomeld::Bytes.new("\x00\x01\xFF"),
  "nested" => {"list" => [1, "two", {"three" => 3}]}
})

File.binwrite(File.join(out, "ruby.automerge"), doc.save)
```

The hash argument needs its braces: without them Ruby treats the String keys as keyword arguments.

Append to `mise.toml`:

```toml

[tasks."test:interop"]
depends = ["compile"]
run = ["bundle exec ruby test/interop/write_fixtures.rb", "node test/fixtures/interop.mjs"]
```

- [ ] **Step 4: Run it to verify it passes**

Run: `mise run test:interop`
Expected: last line `interop ok`.

- [ ] **Step 5: Commit**

```bash
git add test/fixtures/interop.mjs test/interop/write_fixtures.rb mise.toml
git commit -F - <<'EOF'
Check that JavaScript Automerge reads Ruby-written documents

Co-Authored-By: Claude Opus 5 (1M context) <noreply@anthropic.com>
EOF
```

---

### Task 8: Continuous integration

**Files:**
- Create: `.github/workflows/test.yml`
- Modify: `Gemfile.lock` (add Linux platforms)

**Interfaces:**
- Consumes: mise tasks `setup`, `test`, `test:interop` (Tasks 1, 3, 7).
- Produces: `Test` workflow on pushes to `main` and on pull requests. `Gemfile.lock` resolvable on Linux runners (the release workflow in Task 9 also installs with `bundler-cache: true`, which freezes the lockfile).

- [ ] **Step 1: Make the lockfile resolvable on Linux**

Run:

```bash
bundle lock --add-platform x86_64-linux aarch64-linux
grep -A 4 '^PLATFORMS' Gemfile.lock
```

Expected: `PLATFORMS` lists `aarch64-linux`, `arm64-darwin-*`, and `x86_64-linux`.

- [ ] **Step 2: Write the workflow**

Create `.github/workflows/test.yml`:

```yaml
name: Test

on:
  push:
    branches: [main]
  pull_request:

env:
  MISE_TRUSTED_CONFIG_PATHS: ${{ github.workspace }}

jobs:
  test:
    name: Ruby ${{ matrix.ruby }} on ${{ matrix.os }}
    runs-on: ${{ matrix.os }}
    strategy:
      fail-fast: false
      matrix:
        os: [ubuntu-latest, macos-latest]
        ruby: ["3.2", "3.3", "3.4", "4.0"]
    env:
      MISE_RUBY_VERSION: ${{ matrix.ruby }}
    steps:
      - uses: actions/checkout@v7
      - uses: jdx/mise-action@v4
      - run: mise run setup
      - run: mise run test

  interop:
    name: JavaScript interop
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v7
      - uses: jdx/mise-action@v4
      - run: mise run setup
      - run: mise run test:interop
```

`MISE_RUBY_VERSION` overrides the `ruby` pinned in `mise.toml` for each matrix cell. mise installs precompiled Rubies (`ruby.precompiled_url` defaults to `jdx/ruby`), so the matrix does not compile Ruby.

- [ ] **Step 3: Validate the workflow syntax**

Run: `action-validator .github/workflows/test.yml` (installed through `~/.tool-versions`; otherwise `mise exec action-validator@0.5.1 -- action-validator .github/workflows/test.yml`)
Expected: no output, exit status 0.

- [ ] **Step 4: Commit**

```bash
git add .github/workflows/test.yml Gemfile.lock
git commit -F - <<'EOF'
Run tests and JavaScript interop in GitHub Actions

Co-Authored-By: Claude Opus 5 (1M context) <noreply@anthropic.com>
EOF
```

The workflow is verified for real when the branch is pushed and a pull request runs it. That happens outside this plan.

---

### Task 9: Releases and documentation

**Files:**
- Create: `.github/workflows/release.yml`, `CHANGELOG.md` (generated)
- Modify: `Gemfile`, `Rakefile`, `README.md`

**Interfaces:**
- Consumes: gemspec and Rakefile from Task 1; trailered commits from Tasks 2–6.
- Produces: `rake reissue:*` tasks; the `Release gem to RubyGems.org` workflow (source gem via the SOFware shared workflow, then platform gems for `arm64-darwin`, `x86_64-darwin`, `x86_64-linux`, `aarch64-linux` × Ruby 3.2–4.0).

- [ ] **Step 1: Add reissue**

In `Gemfile`, add after `gem "rake-compiler", "~> 1.3"`:

```ruby
gem "reissue", "~> 0.5"
```

In `Rakefile`, add `require "reissue/gem"` after `require "rb_sys/extensiontask"` (it must come after `bundler/gem_tasks`, whose `build` task it enhances), and add before `task default: :test`:

```ruby
Reissue::Task.create :reissue do |task|
  task.version_file = "lib/mechanomeld/version.rb"
  task.fragment = :git
  # The post-release bump goes on a pushed branch; the shared release workflow opens its PR.
  # (0.5.1's default, stated explicitly: it is the "Option A" the shared workflow expects.)
  task.push_reissue = :branch
end

```

reissue's hooks attach to bundler's `build` and `release` tasks only. rb-sys-dock runs `rake native:<platform> gem` inside its container, so native builds never trigger a bump or commit.

Run:

```bash
bundle install
bundle exec rake reissue:initialize
cat CHANGELOG.md
```

Expected: `✓ Created CHANGELOG.md`; the file has a `## [0.1.0] - Unreleased` heading.

- [ ] **Step 2: Verify trailers reach the changelog**

Run: `bundle exec rake reissue:preview`
Expected: the `Added` entries from the Task 2–6 commit trailers (for example `Mechanomeld::Document.load reads documents saved by JavaScript Automerge`). If they are missing, check that each commit's `Added:` line sits in the final trailer block with `Co-Authored-By` (`git log -5 --format=%B`).

- [ ] **Step 3: Write the release workflow**

Create `.github/workflows/release.yml`:

```yaml
name: Release gem to RubyGems.org

on:
  workflow_dispatch:
    inputs:
      dry_run:
        description: "Test the release without publishing to RubyGems"
        required: false
        type: boolean
        default: false

jobs:
  release:
    uses: SOFware/reissue/.github/workflows/shared-ruby-gem-release.yml@main
    permissions:
      id-token: write
      contents: write
      pull-requests: write
    with:
      git_user_email: "gems@sofwarellc.com"
      git_user_name: "SOFware"
      # ruby/setup-ruby in the shared workflow does not read mise.toml.
      ruby_version: "4.0"
      dry_run: ${{ inputs.dry_run }}

  native:
    name: Build ${{ matrix.platform }} gem
    needs: release
    if: ${{ !inputs.dry_run }}
    runs-on: ubuntu-latest
    strategy:
      fail-fast: false
      matrix:
        platform: [arm64-darwin, x86_64-darwin, x86_64-linux, aarch64-linux]
    steps:
      - uses: actions/checkout@v7
        with:
          ref: v${{ needs.release.outputs.version }}
      - uses: oxidize-rb/actions/setup-ruby-and-rust@v1
        with:
          ruby-version: "4.0"
          bundler-cache: true
          cargo-cache: true
      - id: cross-gem
        uses: oxidize-rb/actions/cross-gem@v1
        with:
          platform: ${{ matrix.platform }}
          ruby-versions: "3.2,3.3,3.4,4.0"
      - uses: actions/upload-artifact@v7
        with:
          name: gem-${{ matrix.platform }}
          path: ${{ steps.cross-gem.outputs.gem-path }}

  publish-native:
    name: Publish platform gems
    needs: native
    runs-on: ubuntu-latest
    permissions:
      id-token: write
      contents: read
    steps:
      - uses: actions/download-artifact@v8
        with:
          pattern: gem-*
          merge-multiple: true
          path: pkg
      - uses: ruby/setup-ruby@v1
        with:
          ruby-version: "4.0"
      - uses: rubygems/configure-rubygems-credentials@v2.1.0
      - name: Push platform gems
        run: |
          for gem in pkg/*.gem; do
            gem push "$gem"
          done
```

Run: `action-validator .github/workflows/release.yml`
Expected: no output, exit status 0.

- [ ] **Step 4: Document installation and usage**

Replace `README.md`:

````markdown
# Mechanomeld

Read and write [Automerge](https://automerge.org) documents from Ruby. Mechanomeld is a native extension around the Rust `automerge` crate, so documents round-trip with JavaScript's `@automerge/automerge`.

## Installation

```bash
bundle add mechanomeld
```

Precompiled gems are published for arm64/x86_64 macOS and Linux on Ruby 3.2–4.0. Other platforms build from source and need a Rust toolchain.

## Usage

```ruby
require "mechanomeld"

doc = Mechanomeld::Document.load(File.binread("todos.automerge"))
doc.get(["todos", 0, "title"]) # => #<Mechanomeld::Text "Buy milk">
doc.to_h                       # => {"todos" => [{"title" => #<Mechanomeld::Text "Buy milk">, ...}]}
doc.keys                       # => ["todos"]
doc.length("todos")            # => 1

doc = Mechanomeld::Document.from({"title" => Mechanomeld::Text.new("Groceries"), "items" => []})
doc.change(message: "Add milk") do |d|
  d["count"] = Mechanomeld::Counter.new(1)
end
File.binwrite("groceries.automerge", doc.save)
```

### Values

| Automerge | Ruby |
|---|---|
| map / list | `Hash` (String keys) / `Array` |
| text object | `Mechanomeld::Text` |
| string | `String` |
| int / f64 / boolean / null | `Integer` / `Float` / `true`, `false` / `nil` |
| counter / uint | `Mechanomeld::Counter` / `Mechanomeld::Uint` |
| timestamp | `Mechanomeld::Timestamp` (milliseconds since the epoch) |
| bytes | `Mechanomeld::Bytes` |

JavaScript stores strings as text objects by default, so they read as `Mechanomeld::Text`. A Ruby `String` is written as an Automerge string, which JavaScript reads as an `ImmutableString`; write `Mechanomeld::Text.new("...")` to create text JavaScript reads as a plain string.

Errors from Automerge raise `Mechanomeld::Error`.

## Development

Tools are pinned in `mise.toml`.

```bash
mise trust
mise install
mise run setup         # bundle install, npm ci for fixtures
mise run test          # compile the extension and run the tests
mise run test:interop  # check JavaScript reads Ruby-written documents
mise run fixtures      # regenerate test/fixtures/*.automerge
```

Check packaging with `gem build mechanomeld.gemspec`. Do not run `rake build` or `rake release` locally: reissue bumps the version and commits during `build`.

## Releasing

Releases run from the **Release gem to RubyGems.org** GitHub Actions workflow. Changelog entries and version bumps come from commit trailers (`Added:`, `Changed:`, `Fixed:`, `Version: minor`, ...). Run it with `dry_run` first.

## Contributing

Bug reports and pull requests are welcome on GitHub at https://github.com/SOFware/mechanomeld.
````

- [ ] **Step 5: Verify packaging and tests still pass**

```bash
git add -A
mise run test
gem build mechanomeld.gemspec
tar -xOf mechanomeld-0.1.0.gem metadata.gz | gunzip | grep -A 20 '^files:'
rm mechanomeld-0.1.0.gem
```

Expected: `70 runs, 98 assertions, 0 failures, 0 errors`; `Successfully built RubyGem`; `files:` lists exactly `CHANGELOG.md`, `Cargo.lock`, `Cargo.toml`, `README.md`, `Rakefile`, the nine `ext/mechanomeld/` files (`Cargo.toml`, `extconf.rb`, `src/classes.rs`, `src/document.rs`, `src/errors.rs`, `src/lib.rs`, `src/path.rs`, `src/read.rs`, `src/write.rs`), the five `lib/` files, and `sig/mechanomeld.rbs`. Nothing from `.github/`, `docs/`, `test/`, `mise.toml`, or `Gemfile.lock`.

- [ ] **Step 6: Commit**

```bash
git add -A
git commit -F - <<'EOF'
Release with reissue and publish precompiled platform gems

Co-Authored-By: Claude Opus 5 (1M context) <noreply@anthropic.com>
EOF
```

- [ ] **Step 7: Hand off manual release prerequisites**

These cannot be done from the repository. Report them to the user rather than attempting them:

1. Create `SOFware/mechanomeld` on GitHub and push `main` and the branch.
2. On rubygems.org, add a pending trusted publisher for gem `mechanomeld`: repository `SOFware/mechanomeld`, workflow `release.yml`.
3. Run the release workflow once with `dry_run: true` and check the `release` job's output.
