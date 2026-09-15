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
