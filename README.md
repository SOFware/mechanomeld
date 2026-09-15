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
