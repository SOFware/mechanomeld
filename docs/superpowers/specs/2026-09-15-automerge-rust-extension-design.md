# Mechanomeld: Automerge for Ruby via a Rust extension

Date: 2026-09-15
Status: Draft for review

## Goal

Read, create, and save [Automerge](https://github.com/automerge/automerge) documents from Ruby,
interoperating with documents produced by JavaScript `@automerge/automerge`.

Delivered in two phases:

1. **Read** — load bytes produced by JS and read values.
2. **Write** — create documents, put/delete values, commit, save bytes that JS can load.

### Non-goals (for now)

Sync (`SyncState`, sync messages), cursors, marks, history (`heads:` reads, `get_changes`,
`apply_changes`, `fork`/`merge`), conflict listing (`get_all`), list insert/splice, text splice,
counter increment, JRuby/TruffleRuby.

## Decisions

| Decision | Choice | Why |
|---|---|---|
| Binding | Rust extension (magnus 0.8 + rb-sys 0.9.130) on the `automerge` 0.11 crate | automerge-c also needs a Rust build and ships no prebuilt binaries; the Rust crate is the primary, actively maintained API; Rust owns memory instead of manual `AMresultFree` bookkeeping. Spike built and ran on Ruby 4.0.6/arm64-darwin. |
| Distribution | Precompiled platform gems + source gem fallback | `gem install` without a Rust toolchain on common platforms |
| Namespace | `Mechanomeld` | Matches gem name; no clash with other automerge gems |
| Text objects on read | `Mechanomeld::Text` | Lossless; mirrors the reference implementation |
| Ruby `String` on write | Automerge scalar string; `Text.new(...)` creates a text object | Symmetric with reading (`String` in, `String` out). JS readers see these as `ImmutableString`. |
| Toolchain | `mise.toml` pins Ruby, Rust, Node and defines dev tasks | Project standard |
| Releases | `reissue` with git trailers + SOFware shared release workflow | Project standard |
| Ruby floor | `required_ruby_version >= 3.2` | Unchanged from skeleton |

## Architecture

A thin native core does document access and value conversion. Scalar wrapper classes and
convenience methods stay in plain Ruby so the native layer stays small and swappable.

```
Cargo.toml                      workspace: members = ["ext/mechanomeld"]
Cargo.lock
ext/mechanomeld/
  Cargo.toml                    magnus, rb-sys, automerge (linker flags come from rb_sys mkmf)
  extconf.rb                    require "rb_sys/mkmf"; create_rust_makefile("mechanomeld/mechanomeld")
  src/lib.rs                    #[magnus::init]: defines Mechanomeld::Document methods
  src/classes.rs                lazy lookups of Mechanomeld::Error and the scalar classes
  src/errors.rs                 Mechanomeld::Error / TypeError / ArgumentError constructors
  src/document.rs               Document wrapper and methods
  src/read.rs                   Automerge value -> Ruby value
  src/write.rs                  Ruby value -> Automerge put operations
  src/path.rs                   Ruby path -> (parent ObjId, Prop) resolution
lib/mechanomeld.rb              requires version, error, scalars, native ext, document
lib/mechanomeld/error.rb        Mechanomeld::Error < StandardError
lib/mechanomeld/scalars.rb      Scalar, Text, Counter, Timestamp, Uint, Bytes
lib/mechanomeld/document.rb     Ruby conveniences on Document
test/fixtures/package.json      pins @automerge/automerge 3.4.1
test/fixtures/generate.mjs      writes *.automerge fixtures (committed)
test/fixtures/interop.mjs       loads Ruby-saved bytes and asserts contents
test/interop/write_fixtures.rb  writes Ruby-created documents to test/interop/out/ for interop.mjs
test/test_*.rb                  Minitest (the skeleton's naming convention)
mise.toml
CHANGELOG.md
.github/workflows/test.yml
.github/workflows/release.yml
```

### Load order

`lib/mechanomeld.rb` defines `Error` and the scalar classes **before** loading the native
extension, because the native code constructs those classes. Native loading prefers the
fat-binary per-Ruby-version path:

```ruby
begin
  RUBY_VERSION =~ /(\d+\.\d+)/
  require_relative "mechanomeld/#{Regexp.last_match(1)}/mechanomeld"
rescue LoadError
  require_relative "mechanomeld/mechanomeld"
end
```

`lib/mechanomeld/document.rb` is required last and reopens the natively defined `Document`.

### Native Document

- `#[magnus::wrap(class = "Mechanomeld::Document", free_immediately, size)]` around
  `RefCell<automerge::AutoCommit>`. `RefCell` gives `&mut` for writes and reads that need it
  (e.g. `save`); a borrow conflict raises `Mechanomeld::Error` rather than panicking.
- Documents are created and loaded with `TextEncoding::UnicodeCodePoint` so text lengths match
  Ruby `String#length`. Encoding affects index-based operations only, not stored data, so JS
  interop is unaffected.
- All `AutomergeError`s map to `Mechanomeld::Error` with the Rust error message.
- Value conversion happens entirely in Rust; `to_h` is one native call.
- Scalar wrappers are built by allocating the class and setting `@value` directly (as the
  reference C extension did), not by calling `new`, so no Ruby code runs while the `RefCell` is
  borrowed.

## Scalar classes (Ruby)

As in the reference implementation:

```ruby
class Scalar          # attr_reader :value; == compares class and value
class Counter < Scalar
class Timestamp < Scalar   # value: Integer milliseconds since epoch
class Uint < Scalar
class Bytes < Scalar       # value coerced to binary String (String(value).b)
class Text < Scalar        # value coerced to String; default ""
```

Add `Text#to_s` returning `value`. Scalars get `hash`/`eql?` consistent with `==`.

## Paths

Used by every read and write method.

- `nil` or `[]` is the document root. A non-Array is treated as a one-element path.
- A segment into a map must be a `String` or `Symbol` (Symbols converted to String).
- A segment into a list must be a non-negative `Integer`.
- Wrong segment type raises `TypeError`; a negative index raises `ArgumentError`.
- Descending into a scalar or a text object raises `Mechanomeld::Error`.

## Phase 1: Read

### API

| Method | Returns | Notes |
|---|---|---|
| `Document.load(bytes)` | `Document` | Invalid bytes raise `Mechanomeld::Error` |
| `doc.get(path)` / `doc[path]` | value | Missing key/index anywhere along the path returns `nil` |
| `doc.to_h` (alias `to_hash`) | `Hash` | `get([])` |
| `doc.keys(path = [])` | `Array<String>` | Target must be an existing map, else `Mechanomeld::Error` |
| `doc.length(path = [])` | `Integer` | Map: key count; list: element count; text: code points. Missing path raises `Mechanomeld::Error` |

### Automerge to Ruby

| Automerge | Ruby |
|---|---|
| map | `Hash` with `String` keys |
| list | `Array` |
| text object | `Mechanomeld::Text` |
| `Str` | `String` (UTF-8) |
| `Int` | `Integer` |
| `F64` | `Float` |
| `Boolean` | `true` / `false` |
| `Null` | `nil` |
| `Counter` | `Mechanomeld::Counter` |
| `Timestamp` | `Mechanomeld::Timestamp` |
| `Uint` | `Mechanomeld::Uint` |
| `Bytes` | `Mechanomeld::Bytes` (binary `String`) |
| `Unknown` | raises `Mechanomeld::Error` |
| table (legacy) | raises `Mechanomeld::Error` |

When a key has conflicting concurrent values, `get` returns Automerge's winning value.

## Phase 2: Create and save

### API

| Method | Returns | Notes |
|---|---|---|
| `Document.new(actor_id: nil)` | `Document` | `actor_id` is a hex String; invalid raises `Mechanomeld::Error` |
| `Document.from(hash, actor_id: nil)` | `Document` | Ruby: `put` each key, then `commit`. Non-Hash raises `ArgumentError` |
| `doc.put(path, value)` / `doc[path] = value` | `self` | Parent must exist, else `Mechanomeld::Error`. Into a list, index must be `< length` (overwrite) |
| `doc.delete(path)` | `self` | Map key or list index |
| `doc.commit(message: nil, timestamp: nil)` | `String` (binary hash) or `nil` | `nil` when nothing pending. `timestamp` is Integer **seconds** (automerge `CommitOptions#with_time`), while `Timestamp` values are **milliseconds**; both follow Automerge, so don't "fix" the difference |
| `doc.rollback` | `Integer` | Number of ops discarded |
| `doc.change(message: nil, timestamp: nil) { \|doc\| }` | `self` | Ruby: commits after block; rolls back and re-raises on any exception |
| `doc.save` | binary `String` | Commits pending ops first (AutoCommit behavior) |

### Ruby to Automerge

| Ruby | Automerge |
|---|---|
| `nil` | `Null` |
| `true` / `false` | `Boolean` |
| `Integer` | `Int`; outside i64 raises `RangeError` |
| `Float` | `F64` |
| `String` | `Str`; bytes must be valid UTF-8 regardless of the String's declared encoding, else `ArgumentError` |
| `Symbol` value | raises `TypeError` |
| `Mechanomeld::Text` | text object filled via `splice_text` |
| `Mechanomeld::Counter` / `Timestamp` / `Uint` / `Bytes` | matching scalar |
| `Hash` | map object, filled recursively; keys `String` or `Symbol`, else `TypeError` |
| `Array` | list object, filled recursively |
| anything else | raises `TypeError` |

A `TypeError` mid-way through a nested `put` leaves partial pending ops; callers use `change`
(which rolls back) when atomicity matters.

## Toolchain: mise.toml

```toml
[tools]
ruby = "4.0"
rust = "1.90"
node = "24"

[tasks.setup]
run = ["bundle install", "npm ci --prefix test/fixtures"]

[tasks.compile]
run = "bundle exec rake compile"

[tasks.test]
run = "bundle exec rake test"   # rake's test task depends on compile

[tasks.fixtures]
run = "node test/fixtures/generate.mjs"

[tasks."test:interop"]
depends = ["compile"]
run = "bundle exec ruby test/interop/write_fixtures.rb && node test/fixtures/interop.mjs"
```

Rake remains the source of truth for compiling and packaging (`RbSys::ExtensionTask`,
rb-sys-dock); mise tasks wrap it.

## Build and packaging

- gemspec: `spec.extensions = ["ext/mechanomeld/extconf.rb"]`; `spec.files` includes `ext/**`,
  `Cargo.toml`, `Cargo.lock`; runtime dependency `rb_sys`.
- Gemfile: `rake-compiler`, `reissue`.
- Rakefile: `RbSys::ExtensionTask.new("mechanomeld", GEMSPEC) { |ext| ext.lib_dir = "lib/mechanomeld" }`
  and `task test: :compile`. No platform list: rb_sys reads the target from `RUBY_TARGET`, which
  `oxidize-rb/actions/cross-gem` sets. `build` must not depend on `compile` (the shared release
  workflow has no Rust).
- `rake build` runs reissue's bump and finalize (which commit), so verify packaging locally with
  `gem build mechanomeld.gemspec`, never `rake build`.
- Platform gemspecs need no custom `cross_compiling` block: rb_sys's `ExtensionTask` already drops
  the `rb_sys` dependency and `.rs`/`Cargo.*`/extconf files, and rake-compiler clears
  `extensions` and bounds `required_ruby_version` to the cross-compiled Ruby range, so a Ruby
  outside that range resolves to the source gem.
- Platforms: `arm64-darwin`, `x86_64-darwin`, `x86_64-linux`, `aarch64-linux`.
  `x86_64-linux-musl` excluded for now.
- `.gitignore`: `target/`, `tmp/`, `lib/mechanomeld/*.bundle`, `lib/mechanomeld/*.so`,
  `lib/mechanomeld/*/`, `test/fixtures/node_modules/`, `test/interop/out/`.

## Releases: reissue

Rakefile:

```ruby
require "reissue/gem"

Reissue::Task.create :reissue do |task|
  task.version_file = "lib/mechanomeld/version.rb"
  task.fragment = :git
end
```

- Changelog entries come from commit trailers: `Added:`, `Changed:`, `Deprecated:`, `Removed:`,
  `Fixed:`, `Security:`. Version bumps from `Version: major|minor|patch`.
- Create `CHANGELOG.md` with `bundle exec rake reissue:initialize`. Bundler tags `vX.Y.Z`,
  matching reissue's default `tag_pattern`. Until the first tag exists, git fragments read every
  commit in the repository.

`.github/workflows/release.yml` (`workflow_dispatch`, with `dry_run` input):

1. **release** — `uses: SOFware/reissue/.github/workflows/shared-ruby-gem-release.yml@main` with
   `ruby_version: "4.0"` (the shared workflow uses `ruby/setup-ruby`, which does not read
   `mise.toml`), `git_user_name`, `git_user_email`, `dry_run`. Publishes the source gem, tags,
   opens the version-bump PR.
2. **native** — `needs: release`, skipped on dry run. Matrix over platforms; checks out
   `v${{ needs.release.outputs.version }}`; `oxidize-rb/actions/setup-ruby-and-rust` +
   `oxidize-rb/actions/cross-gem`; uploads each `.gem` as an artifact.
3. **publish-native** — `needs: native`. Downloads artifacts;
   `rubygems/configure-rubygems-credentials`; `gem push` each platform gem.

Known gap: platform gems publish a few minutes after the source gem; installs in that window
compile from source. The dry-run version preview requires reissue ≥ 0.5.2 (0.5.1 is current).

The release job's `bundle install` does not compile the extension: Bundler installs `gemspec`
path sources with `disable_extensions: true`.

The first run of `release.yml` uses `dry_run: true` to exercise `bundle install`, reissue with no
prior tag, and the initial `CHANGELOG.md` before anything is published.

Prerequisite: RubyGems Trusted Publishing configured for `release.yml`.

## Testing

- **Fixtures** (`generate.mjs`, JS `@automerge/automerge` 3.4.1, committed binaries):
  nested maps and lists, JS default strings (text objects), `ImmutableString` (scalar strings),
  counter, `Date` (timestamp), `new Uint(7)` (reads back in JS as a BigInt), `Uint8Array` (bytes),
  `null`, floats, and a document with a concurrent conflict. Invalid bytes are an inline test string.
- **Phase 1 tests**: every row of the Automerge→Ruby table; path rules and errors; `keys`/`length`
  on map, list, text; `to_h` of the full fixture equals an expected Hash.
- **Phase 2 tests**: every row of the Ruby→Automerge table; `commit` return values; `rollback`;
  `change` rollback on exception; `save` → `load` round trip equals original `to_h`.
- **Interop** (`mise run test:interop`): Ruby writes documents covering the write table; Node
  loads them and asserts expected JS values (including `ImmutableString` for Ruby `String`).
- **CI** (`test.yml`): `jdx/mise-action`; matrix `ubuntu-latest`, `macos-latest` ×
  Ruby 3.2, 3.3, 3.4, 4.0 via `MISE_RUBY_VERSION`; runs `mise run test`, and `mise run test:interop`
  on one cell.

## Open items to verify during implementation

- magnus 0.8.2 with rb-sys 0.9.130 under `oxidize-rb/actions/cross-gem` for all four platforms
  and Ruby 3.2–4.0 (spike only covered arm64-darwin / Ruby 4.0.6 via plain cargo).
- How JS 3.4.1 encodes `Uint` values (fixture may need `new Uint(...)` or may be unreachable from JS;
  if unreachable, cover `Uint` via Ruby round trip only).
- reissue `commit`/`push_finalize` defaults behave with the shared workflow on this repo's branch
  protection settings.
