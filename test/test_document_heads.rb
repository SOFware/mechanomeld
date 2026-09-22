# frozen_string_literal: true

require "test_helper"

class TestDocumentHeads < Minitest::Test
  HEX_HASH = /\A[0-9a-f]{64}\z/

  def setup
    @doc = Mechanomeld::Document.new
  end

  def test_a_new_document_has_no_heads
    assert_equal [], @doc.heads
  end

  def test_one_change_gives_one_hex_hash
    @doc.change { |doc| doc["k"] = 1 }
    assert_equal 1, @doc.heads.length
    assert_match HEX_HASH, @doc.heads.first
  end

  def test_heads_change_after_loading_a_later_save
    @doc.change { |doc| doc["k"] = 1 }
    before = @doc.heads
    later = Mechanomeld::Document.load(@doc.save)
    later.change { |doc| doc["k"] = 2 }
    @doc.load_incremental(later.save)
    refute_equal before, @doc.heads
    assert_equal later.heads, @doc.heads
  end

  def test_heads_commits_pending_operations_like_save
    @doc.put("k", 1)
    refute_empty @doc.heads
    assert_nil @doc.commit
  end

  def test_heads_match_javascript_getHeads_for_fixtures
    assert_equal fixture_heads("types"), Mechanomeld::Document.load(fixture("types")).heads
    assert_equal fixture_heads("conflict"), Mechanomeld::Document.load(fixture("conflict")).heads
  end

  def test_heads_match_javascript_getHeads_for_a_repo_document
    id = "3bTvdjpAxqE36opq4dAe5sD6ps6o"
    assert_equal fixture_heads(id), Mechanomeld::Document.load_repo(REPO_DIR, id).heads
  end

  def test_includes_its_own_heads
    @doc.change { |doc| doc["k"] = 1 }
    assert @doc.includes_heads?(@doc.heads)
  end

  def test_includes_heads_captured_before_a_later_change
    @doc.change { |doc| doc["k"] = 1 }
    earlier = @doc.heads
    @doc.change { |doc| doc["k"] = 2 }
    assert @doc.includes_heads?(earlier)
    assert @doc.includes_heads?(earlier + @doc.heads)
  end

  def test_includes_an_empty_list_of_heads
    assert @doc.includes_heads?([])
  end

  def test_does_not_include_heads_of_an_unrelated_document
    @doc.change { |doc| doc["k"] = 1 }
    other = Mechanomeld::Document.from({"k" => 1})
    refute @doc.includes_heads?(other.heads)
    refute @doc.includes_heads?(@doc.heads + other.heads)
  end

  def test_does_not_include_heads_of_a_later_version
    @doc.change { |doc| doc["k"] = 1 }
    earlier = Mechanomeld::Document.load(@doc.save)
    @doc.change { |doc| doc["k"] = 2 }
    refute earlier.includes_heads?(@doc.heads)
  end

  def test_rejects_a_hash_that_is_not_hex
    error = assert_raises(Mechanomeld::Error) { @doc.includes_heads?(["zz"]) }
    assert_match(/invalid change hash "zz"/, error.message)
  end

  def test_rejects_a_hash_of_the_wrong_length
    error = assert_raises(Mechanomeld::Error) { @doc.includes_heads?(["abcd"]) }
    assert_match(/invalid change hash "abcd": incorrect length/, error.message)
  end
end
