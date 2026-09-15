# frozen_string_literal: true

require "test_helper"

class TestDocumentChange < Minitest::Test
  def setup
    @doc = Mechanomeld::Document.new
  end

  def test_commit_returns_a_binary_change_hash
    @doc.put("k", 1)
    hash = @doc.commit(message: "set k", timestamp: 1_700_000_000)
    assert_equal 32, hash.bytesize
    assert_equal Encoding::BINARY, hash.encoding
  end

  def test_commit_with_nothing_pending_returns_nil
    assert_nil @doc.commit
  end

  def test_rollback_discards_pending_operations
    @doc.put("k", 1)
    @doc.put("j", 2)
    assert_equal 2, @doc.rollback
    assert_equal({}, @doc.to_h)
  end

  def test_change_commits_the_block
    @doc.change(message: "set k") { |doc| doc["k"] = 1 }
    assert_equal 1, @doc["k"]
    assert_nil @doc.commit
  end

  def test_change_returns_self
    assert_same @doc, @doc.change { |doc| doc["k"] = 1 }
  end

  def test_change_rolls_back_when_the_block_raises
    @doc.change { |doc| doc["kept"] = 1 }
    assert_raises(RuntimeError) do
      @doc.change do |doc|
        doc["lost"] = 2
        raise "boom"
      end
    end
    assert_equal({"kept" => 1}, @doc.to_h)
  end

  def test_from_builds_a_committed_document
    doc = Mechanomeld::Document.from({"a" => 1, :b => [true]})
    assert_equal({"a" => 1, "b" => [true]}, doc.to_h)
    assert_nil doc.commit
  end

  def test_from_accepts_an_actor_id
    doc = Mechanomeld::Document.from({"a" => 1}, actor_id: "0123456789abcdef")
    assert_equal({"a" => 1}, doc.to_h)
  end

  def test_from_rejects_a_non_hash
    assert_raises(ArgumentError) { Mechanomeld::Document.from([1]) }
  end
end
