# frozen_string_literal: true

require "test_helper"

class TestDocumentWrite < Minitest::Test
  SCALARS = {
    "true" => true,
    "false" => false,
    "int" => -3,
    "float" => 2.5,
    "string" => "plain",
    "counter" => Mechanomeld::Counter.new(5),
    "timestamp" => Mechanomeld::Timestamp.new(1_700_000_000_123),
    "uint" => Mechanomeld::Uint.new(7),
    "bytes" => Mechanomeld::Bytes.new("\x00\xFF")
  }.freeze

  def setup
    @doc = Mechanomeld::Document.new
  end

  def test_new_document_is_empty
    assert_equal({}, @doc.to_h)
  end

  def test_new_accepts_a_hex_actor_id
    assert_instance_of Mechanomeld::Document, Mechanomeld::Document.new(actor_id: "0123456789abcdef")
  end

  def test_new_rejects_an_invalid_actor_id
    assert_raises(Mechanomeld::Error) { Mechanomeld::Document.new(actor_id: "not hex") }
  end

  def test_put_scalars_reads_them_back
    SCALARS.each { |key, value| @doc.put(key, value) }
    SCALARS.each { |key, value| assert_equal value, @doc.get(key), key }
  end

  def test_put_nil
    @doc.put("nothing", nil)
    assert_includes @doc.keys, "nothing"
    assert_nil @doc.get("nothing")
  end

  def test_put_string_stays_a_string
    @doc.put("s", "plain")
    assert_instance_of String, @doc.get("s")
  end

  def test_put_text_creates_a_text_object
    @doc.put("t", Mechanomeld::Text.new("héllo"))
    assert_equal Mechanomeld::Text.new("héllo"), @doc.get("t")
    assert_equal 5, @doc.length("t")
  end

  def test_put_returns_self
    assert_same @doc, @doc.put("k", 1)
  end

  def test_bracket_assignment
    @doc["k"] = 1
    assert_equal 1, @doc["k"]
  end

  def test_put_nested_hashes_and_arrays
    @doc.put("nested", {"list" => [1, "two", {three: 3}], "empty" => {}})
    assert_equal({"list" => [1, "two", {"three" => 3}], "empty" => {}}, @doc.get("nested"))
  end

  def test_put_overwrites_an_existing_list_index
    @doc.put("list", [1, 2, 3])
    @doc.put(["list", 1], "b")
    assert_equal [1, "b", 3], @doc.get("list")
  end

  def test_put_past_the_end_of_a_list_raises
    @doc.put("list", [1, 2, 3])
    assert_raises(Mechanomeld::Error) { @doc.put(["list", 3], 4) }
  end

  def test_put_into_a_missing_parent_raises
    assert_raises(Mechanomeld::Error) { @doc.put(["missing", "k"], 1) }
  end

  def test_put_with_an_empty_path_raises_argument_error
    assert_raises(ArgumentError) { @doc.put([], 1) }
  end

  def test_integer_outside_i64_raises_range_error
    assert_raises(RangeError) { @doc.put("big", 2**64) }
  end

  def test_negative_uint_raises_range_error
    assert_raises(RangeError) { @doc.put("u", Mechanomeld::Uint.new(-1)) }
  end

  def test_symbol_value_raises_type_error
    assert_raises(TypeError) { @doc.put("s", :symbol) }
  end

  def test_unsupported_object_raises_type_error
    assert_raises(TypeError) { @doc.put("o", Object.new) }
  end

  def test_invalid_utf8_string_raises_argument_error
    assert_raises(ArgumentError) { @doc.put("s", "\xFF".b) }
  end

  def test_non_string_hash_key_raises_type_error
    assert_raises(TypeError) { @doc.put("h", {1 => "a"}) }
  end

  def test_delete_a_map_key
    @doc.put("k", 1)
    assert_same @doc, @doc.delete("k")
    assert_equal({}, @doc.to_h)
  end

  def test_delete_a_list_index
    @doc.put("list", [1, 2, 3])
    @doc.delete(["list", 0])
    assert_equal [2, 3], @doc.get("list")
  end

  def test_save_returns_binary_that_loads_back
    @doc.put("nested", {"list" => [1, Mechanomeld::Text.new("two")]})
    @doc.put("count", Mechanomeld::Counter.new(1))
    bytes = @doc.save
    assert_equal Encoding::BINARY, bytes.encoding
    assert_equal @doc.to_h, Mechanomeld::Document.load(bytes).to_h
  end
end
