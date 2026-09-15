# frozen_string_literal: true

require "test_helper"

class TestDocumentRead < Minitest::Test
  def setup
    @doc = Mechanomeld::Document.load(fixture("types"))
  end

  def test_load_rejects_invalid_bytes
    error = assert_raises(Mechanomeld::Error) { Mechanomeld::Document.load("not automerge".b) }
    assert_match(/could not load Automerge document/, error.message)
  end

  def test_js_strings_read_as_text
    assert_equal Mechanomeld::Text.new("héllo 😀"), @doc.get("text")
  end

  def test_immutable_strings_read_as_utf8_strings
    value = @doc.get("string")
    assert_equal "fixed", value
    assert_equal Encoding::UTF_8, value.encoding
  end

  def test_integers
    assert_equal 42, @doc.get("int")
    assert_equal(-3, @doc.get("negative"))
  end

  def test_floats
    assert_equal 1.5, @doc.get("float")
    assert_kind_of Float, @doc.get("whole_float")
  end

  def test_booleans_and_null
    assert_equal true, @doc.get("bool_true")
    assert_equal false, @doc.get("bool_false")
    assert_nil @doc.get("nothing")
  end

  def test_counter
    assert_equal Mechanomeld::Counter.new(5), @doc.get("counter")
  end

  def test_timestamp_is_milliseconds
    assert_equal Mechanomeld::Timestamp.new(1_700_000_000_123), @doc.get("timestamp")
  end

  def test_uint
    assert_equal Mechanomeld::Uint.new(7), @doc.get("uint")
  end

  def test_bytes_are_binary
    value = @doc.get("bytes")
    assert_equal Mechanomeld::Bytes.new("\x00\x01\xFF"), value
    assert_equal Encoding::BINARY, value.value.encoding
  end

  def test_nested_path
    assert_equal 3, @doc.get(["nested", "list", 2, "three"])
  end

  def test_symbol_segments
    assert_equal 3, @doc.get([:nested, :list, 2, :three])
  end

  def test_maps_read_as_hashes_with_string_keys
    expected = {
      "list" => [1, Mechanomeld::Text.new("two"), {"three" => 3}],
      "empty_map" => {},
      "empty_list" => []
    }
    assert_equal expected, @doc.get("nested")
  end

  def test_missing_keys_and_indexes_are_nil
    assert_nil @doc.get("missing")
    assert_nil @doc.get(["missing", "deeper"])
    assert_nil @doc.get(["nested", "list", 99])
  end

  def test_brackets_read_a_path
    assert_equal 1, @doc[["nested", "list", 0]]
  end

  def test_to_h_is_the_whole_document
    expected = {
      "text" => Mechanomeld::Text.new("héllo 😀"),
      "string" => "fixed",
      "int" => 42,
      "negative" => -3,
      "float" => 1.5,
      "whole_float" => 2.0,
      "bool_true" => true,
      "bool_false" => false,
      "nothing" => nil,
      "counter" => Mechanomeld::Counter.new(5),
      "timestamp" => Mechanomeld::Timestamp.new(1_700_000_000_123),
      "uint" => Mechanomeld::Uint.new(7),
      "bytes" => Mechanomeld::Bytes.new("\x00\x01\xFF"),
      "nested" => {
        "list" => [1, Mechanomeld::Text.new("two"), {"three" => 3}],
        "empty_map" => {},
        "empty_list" => []
      }
    }
    assert_equal expected, @doc.to_h
  end

  def test_string_segment_into_list_raises_type_error
    assert_raises(TypeError) { @doc.get(["nested", "list", "0"]) }
  end

  def test_integer_segment_into_map_raises_type_error
    assert_raises(TypeError) { @doc.get(["nested", 0]) }
  end

  def test_negative_index_raises_argument_error
    assert_raises(ArgumentError) { @doc.get(["nested", "list", -1]) }
  end

  def test_descending_into_a_scalar_raises
    assert_raises(Mechanomeld::Error) { @doc.get(["int", "deeper"]) }
  end

  def test_descending_into_text_raises
    assert_raises(Mechanomeld::Error) { @doc.get(["text", 0]) }
  end

  def test_conflicting_values_read_the_winner
    doc = Mechanomeld::Document.load(fixture("conflict"))
    assert_equal "from-b", doc.get("k")
  end
end
