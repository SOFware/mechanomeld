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
