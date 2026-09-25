# frozen_string_literal: true

require "test_helper"

class TestDocumentDiff < Minitest::Test
  REPO_DOCUMENT_DIR = File.join(REPO_DIR, "3b", "TvdjpAxqE36opq4dAe5sD6ps6o")

  def setup
    base = {"scores" => {"alpha" => 1}, "tags" => %w[a b]}
    @doc = Mechanomeld::Document.from(base)
    @start = @doc.heads
  end

  def test_a_nested_put_is_one_patch_with_its_path_and_value
    @doc.change { |doc| doc[["scores", "bravo"]] = 7 }
    expected = {
      "action" => "put",
      "path" => %w[scores bravo],
      "value" => 7,
      "conflict" => false
    }
    assert_equal [expected], @doc.diff(@start, @doc.heads)
  end

  def test_to_heads_defaults_to_the_current_heads
    @doc.change { |doc| doc[["scores", "bravo"]] = 7 }
    assert_equal @doc.diff(@start, @doc.heads), @doc.diff(@start)
  end

  def test_a_put_of_an_object_is_an_empty_container_then_its_children
    @doc.change { |doc| doc["meta"] = {"note" => "hi"} }
    expected = [put_patch(["meta"], {}), put_patch(%w[meta note], "hi")]
    assert_equal expected, @doc.diff(@start)
  end

  def test_deleting_a_key
    @doc.change { |doc| doc.delete(["scores", "alpha"]) }
    expected = {"action" => "delete", "path" => %w[scores alpha]}
    assert_equal [expected], @doc.diff(@start)
  end

  def test_deleting_a_list_element_carries_index_and_length
    @doc.change { |doc| doc.delete(["tags", 0]) }
    expected = {
      "action" => "delete",
      "path" => ["tags", 0],
      "index" => 0,
      "length" => 1
    }
    assert_equal [expected], @doc.diff(@start)
  end

  def test_a_counter_increment_carries_the_delta
    doc, before = repo_document_before_its_incremental_chunks
    increments = doc.diff(before).select { |patch| patch["action"] == "increment" }
    expected = {"action" => "increment", "path" => ["count"], "value" => 9}
    assert_equal [expected], increments
  end

  def test_inserting_into_a_list_carries_the_values
    doc, before = repo_document_before_its_incremental_chunks
    inserts = doc.diff(before).select { |patch| patch["action"] == "insert" }
    values = [Mechanomeld::Text.new(""), Mechanomeld::Text.new("")]
    expected = {"action" => "insert", "path" => ["items", 150], "values" => values}
    assert_equal [expected], inserts
  end

  def test_no_changes_is_empty
    assert_equal [], @doc.diff(@start)
    assert_equal [], @doc.diff(@start, @start)
  end

  def test_from_no_heads_is_the_whole_document
    patches = @doc.diff([]).select { |patch| patch["action"] == "put" }
    expected = [["scores"], ["tags"], %w[scores alpha]]
    assert_equal expected, patches.map { |patch| patch["path"] }
  end

  def test_between_two_historical_head_sets
    @doc.change { |doc| doc[["scores", "bravo"]] = 7 }
    middle = @doc.heads
    @doc.change { |doc| doc[["scores", "charlie"]] = 9 }
    assert_equal [put_patch(%w[scores bravo], 7)], @doc.diff(@start, middle)
  end

  def test_reversed_heads_undo_the_change
    @doc.change { |doc| doc[["scores", "bravo"]] = 7 }
    expected = {"action" => "delete", "path" => %w[scores bravo]}
    assert_equal [expected], @doc.diff(@doc.heads, @start)
  end

  def test_rejects_a_hash_that_is_not_hex
    error = assert_raises(Mechanomeld::Error) { @doc.diff(["zz"]) }
    assert_match(/invalid change hash "zz"/, error.message)
  end

  def test_rejects_a_hash_the_document_does_not_have
    other = Mechanomeld::Document.from({"k" => 1})
    error = assert_raises(Mechanomeld::Error) { @doc.diff(other.heads) }
    assert_match(/unknown change hash "#{other.heads.first}"/, error.message)
    assert_raises(Mechanomeld::Error) { @doc.diff(@start, other.heads) }
  end

  private

  def put_patch(path, value)
    {"action" => "put", "path" => path, "value" => value, "conflict" => false}
  end

  # The repo fixture's snapshot holds count 1 and 150 items; its two incremental
  # chunks each increment count and push an item.
  def repo_document_before_its_incremental_chunks
    doc = Mechanomeld::Document.new
    load_chunks(doc, "snapshot")
    before = doc.heads
    load_chunks(doc, "incremental")
    [doc, before]
  end

  def load_chunks(doc, kind)
    Dir.glob(File.join(REPO_DOCUMENT_DIR, kind, "*")).each do |path|
      doc.load_incremental(File.binread(path))
    end
  end
end
