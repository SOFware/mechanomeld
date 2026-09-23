# frozen_string_literal: true

# Writes a Ruby-created document for test/fixtures/interop.mjs. Run with `mise run test:interop`.
$LOAD_PATH.unshift File.expand_path("../../lib", __dir__)
require "mechanomeld"
require "fileutils"
require "json"

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
File.write(File.join(out, "ruby.heads.json"), JSON.generate(doc.heads))

# The Ruby peer for interop.mjs's sync exchange: the types fixture plus one change of its
# own, with a fresh sync state. test/interop/sync_step.rb takes its turns from here.
sync_dir = File.join(out, "sync")
FileUtils.mkdir_p(sync_dir)
peer = Mechanomeld::Document.load(File.binread(File.expand_path("../fixtures/types.automerge", __dir__)))
peer.change { |d| d["ruby"] = Mechanomeld::Text.new("from ruby") }
File.binwrite(File.join(sync_dir, "ruby.automerge"), peer.save)
File.binwrite(File.join(sync_dir, "ruby.syncstate"), Mechanomeld::SyncState.new.encode)
