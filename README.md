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

### Documents stored by automerge-repo

`Document.load_repo` reads a document straight out of the directory written by `@automerge/automerge-repo`'s `NodeFSStorageAdapter`, applying its snapshot and incremental chunks and skipping its sync-state files.

```ruby
doc = Mechanomeld::Document.load_repo("automerge-repo-data", "4NtRZtd6yUtzf8qmEpKitAaW4JRp")
```

`Document#load_incremental(bytes)` is the piece underneath: it applies any saved Automerge bytes (a full save, a snapshot chunk, or an incremental change chunk) into an existing document and returns the document.

### Heads

`Document#heads` is the document's version: the change hashes JavaScript's `Automerge.getHeads(doc)` returns, as lowercase hex, so the two sides compare with `==`. `Document#includes_heads?(heads)` is true when every hash in `heads` is a change the document already contains, so `heads` is the same version or an older one.

```ruby
doc.heads                  # => ["9d434616e7af865757bad75cdebe4151bccf7e29214a71f2a65150846c2a4275"]
doc.includes_heads?(heads) # => true when the document is at or past `heads`
```

Both take hex. automerge-repo's `handle.heads()` returns base58 URL heads, which are not accepted; send `Automerge.getHeads(handle.doc())` from JavaScript instead. A hash that is not 32 bytes of hex raises `Mechanomeld::Error`; a well-formed hash the document does not have is simply not included. Like `save`, both commit any pending change first.

### What changed between two versions

`Document#diff(from_heads, to_heads = doc.heads)` is Automerge's diff: the patches that take the document from one version to another, as Hashes with String keys. Both arguments are hex heads as `Document#heads` returns them; `to_heads` defaults to the current heads, and `[]` as `from_heads` diffs from the empty document.

```ruby
before = doc.heads
doc.change { |d| d[["scores", "bravo"]] = 7 }
doc.diff(before)
# => [{"action" => "put", "path" => ["scores", "bravo"], "value" => 7, "conflict" => false}]
```

Every patch has `"action"` and `"path"`. The path addresses the property the patch touches from the root, map keys as Strings and list indexes as Integers, as JavaScript's `Automerge.diff` reports it. The other keys depend on the action:

| action | keys |
|---|---|
| `put` | `value`, `conflict` (true when this value won a concurrent write) |
| `delete` | `index` and `length` when deleting from a list; nothing more for a map key |
| `insert` | `values`, the elements inserted at `path`'s index, in order |
| `splice_text` | `value`, the String spliced into a `Text` at `path`'s index |
| `increment` | `value`, the amount a `Counter` changed by, which may be negative |
| `conflict` | nothing more: a concurrent write now conflicts at `path` |
| `mark` | `marks`, each `{"name", "value", "start", "end"}` on a `Text` |

Putting or inserting a map, list, or `Text` yields the empty container (`{}`, `[]`, or `Mechanomeld::Text.new("")`); its contents follow as their own patches. Values use the same Ruby types as `get`. A head that is not hex, or that names a change the document does not contain, raises `Mechanomeld::Error`. Like `save`, `diff` commits any pending change first.

### Syncing with a peer

`Document#generate_sync_message` and `Document#receive_sync_message` run Automerge's per-document sync protocol, so a Ruby document can exchange just the changes each side lacks with another Automerge peer: JavaScript's `Automerge.generateSyncMessage` / `receiveSyncMessage`, or another Ruby document. A `Mechanomeld::SyncState` tracks what one peer is known to have; keep one per peer for each document.

```ruby
left = Mechanomeld::Document.load(bytes)
right = Mechanomeld::Document.load(bytes)
left.change { |d| d["left"] = 1 }
right.change { |d| d["right"] = 2 }

left_state = Mechanomeld::SyncState.new  # left's view of right
right_state = Mechanomeld::SyncState.new # right's view of left
loop do
  to_right = left.generate_sync_message(left_state)
  right.receive_sync_message(right_state, to_right) if to_right
  to_left = right.generate_sync_message(right_state)
  left.receive_sync_message(left_state, to_left) if to_left
  break if to_right.nil? && to_left.nil?
end
left.heads == right.heads # => true
```

Messages are binary Strings. `generate_sync_message` returns nil when the peer is up to date or the last message is still unanswered. `SyncState#encode` and `SyncState.decode(bytes)` carry a state across connections, keeping only the heads both sides are known to share (as JavaScript's `encodeSyncState` does), so a process that handles one message per request can decode, receive, generate, and encode again. Like `save`, both sync methods commit any pending change first. Bytes that are not a sync message or a sync state raise `Mechanomeld::Error`. The protocol carries no document identity: a message from a peer holding a different document merges that document in.

This is the per-document protocol only. An automerge-repo sync server wraps these messages in its own envelope (peer and document ids, join and leave messages, CBOR encoding), which this gem does not provide.

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
mise run fixtures      # regenerate test/fixtures/*.automerge, heads.json, and repo/
```

Check packaging with `gem build mechanomeld.gemspec`. Do not run `rake build` or `rake release` locally: reissue bumps the version and commits during `build`.

## Releasing

Releases run from the **Release gem to RubyGems.org** GitHub Actions workflow. Changelog entries and version bumps come from commit trailers (`Added:`, `Changed:`, `Fixed:`, `Version: minor`, ...). Run it with `dry_run` first.

## Contributing

Bug reports and pull requests are welcome on GitHub at https://github.com/SOFware/mechanomeld.
