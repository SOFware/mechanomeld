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
