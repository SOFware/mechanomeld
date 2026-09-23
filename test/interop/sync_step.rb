# frozen_string_literal: true

# One turn of the Ruby peer in test/fixtures/interop.mjs's sync exchange: receives the
# JavaScript message on stdin, writes the reply (if any) to stdout, and keeps the
# document and sync state in test/interop/out/sync between turns.
$LOAD_PATH.unshift File.expand_path("../../lib", __dir__)
require "mechanomeld"
require "json"

dir = File.expand_path("out/sync", __dir__)
doc = Mechanomeld::Document.load(File.binread(File.join(dir, "ruby.automerge")))
state = Mechanomeld::SyncState.decode(File.binread(File.join(dir, "ruby.syncstate")))

$stdin.binmode
message = $stdin.read
doc.receive_sync_message(state, message) unless message.empty?
reply = doc.generate_sync_message(state)

File.binwrite(File.join(dir, "ruby.automerge"), doc.save)
File.binwrite(File.join(dir, "ruby.syncstate"), state.encode)
File.write(File.join(dir, "ruby.heads.json"), JSON.generate(doc.heads))
$stdout.binmode
$stdout.write(reply) if reply
