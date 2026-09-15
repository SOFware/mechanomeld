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
