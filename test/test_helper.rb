# frozen_string_literal: true

$LOAD_PATH.unshift File.expand_path("../lib", __dir__)
require "mechanomeld"

require "json"
require "minitest/autorun"

module FixtureHelper
  # Bytes of a document written by test/fixtures/generate.mjs.
  def fixture(name)
    File.binread(File.expand_path("fixtures/#{name}.automerge", __dir__))
  end

  # The NodeFSStorageAdapter directory written by test/fixtures/generate.mjs.
  REPO_DIR = File.expand_path("fixtures/repo", __dir__)

  # Automerge.getHeads of a fixture, recorded by test/fixtures/generate.mjs.
  def fixture_heads(name)
    JSON.parse(File.read(File.expand_path("fixtures/heads.json", __dir__))).fetch(name)
  end
end

Minitest::Test.include(FixtureHelper)
