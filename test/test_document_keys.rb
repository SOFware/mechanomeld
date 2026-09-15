# frozen_string_literal: true

require "test_helper"

class TestDocumentKeys < Minitest::Test
  def setup
    @doc = Mechanomeld::Document.load(fixture("types"))
  end

  def test_keys_of_root_are_sorted_strings
    expected = %w[bool_false bool_true bytes counter float int negative nested nothing string text timestamp uint whole_float]
    assert_equal expected, @doc.keys
  end

  def test_keys_at_a_path
    assert_equal %w[empty_list empty_map list], @doc.keys("nested")
  end

  def test_keys_of_a_list_raise
    assert_raises(Mechanomeld::Error) { @doc.keys(["nested", "list"]) }
  end

  def test_keys_of_a_missing_path_raise
    assert_raises(Mechanomeld::Error) { @doc.keys("missing") }
  end

  def test_length_of_map
    assert_equal 14, @doc.length
  end

  def test_length_of_list
    assert_equal 3, @doc.length(["nested", "list"])
  end

  def test_length_of_text_counts_code_points
    assert_equal 7, @doc.length("text")
  end
end
