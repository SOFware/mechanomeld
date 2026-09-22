# frozen_string_literal: true

require "test_helper"

class TestDocumentLoadRepo < Minitest::Test
  SNAPSHOT_ONLY_ID = "2be82g1cnxj1o64J7cFA5wPeyHms"
  INCREMENTAL_ID = "3bTvdjpAxqE36opq4dAe5sD6ps6o"

  def test_loads_a_document_with_only_a_snapshot
    doc = Mechanomeld::Document.load_repo(REPO_DIR, SNAPSHOT_ONLY_ID)
    expected = {"count" => Mechanomeld::Counter.new(17), "title" => Mechanomeld::Text.new("snapshot only")}
    assert_equal expected, doc.to_h
  end

  def test_loads_a_snapshot_with_later_incremental_chunks
    doc = Mechanomeld::Document.load_repo(REPO_DIR, INCREMENTAL_ID)
    items = Array.new(150) { |i| "item-#{i}" } + %w[first second]
    expected = {"count" => Mechanomeld::Counter.new(10), "items" => items.map { |s| Mechanomeld::Text.new(s) }}
    assert_equal expected, doc.to_h
  end

  def test_ignores_sync_state_files
    assert File.exist?(File.join(REPO_DIR, "3b", "TvdjpAxqE36opq4dAe5sD6ps6o", "sync-state", "peer-storage-id"))
    assert_equal 10, Mechanomeld::Document.load_repo(REPO_DIR, INCREMENTAL_ID).get("count").value
  end

  def test_missing_document_raises
    error = assert_raises(Mechanomeld::Error) { Mechanomeld::Document.load_repo(REPO_DIR, "2beNotThere") }
    assert_match(/no chunks for document 2beNotThere/, error.message)
  end
end
