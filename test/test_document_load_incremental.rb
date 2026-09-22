# frozen_string_literal: true

require "test_helper"

class TestDocumentLoadIncremental < Minitest::Test
  def setup
    @source = Mechanomeld::Document.from({"count" => Mechanomeld::Counter.new(1)})
    @first_save = @source.save
    @source.change { |doc| doc.put("title", Mechanomeld::Text.new("later")) }
  end

  def test_into_an_empty_document_matches_load
    doc = Mechanomeld::Document.new.load_incremental(fixture("types"))
    assert_equal Mechanomeld::Document.load(fixture("types")).to_h, doc.to_h
  end

  def test_applies_a_later_save_onto_an_earlier_one
    doc = Mechanomeld::Document.load(@first_save)
    doc.load_incremental(@source.save)
    assert_equal @source.to_h, doc.to_h
  end

  def test_order_of_saves_does_not_matter
    doc = Mechanomeld::Document.new
    doc.load_incremental(@source.save)
    doc.load_incremental(@first_save)
    assert_equal @source.to_h, doc.to_h
  end

  def test_returns_self
    doc = Mechanomeld::Document.new
    assert_same doc, doc.load_incremental(@first_save)
  end

  def test_rejects_invalid_bytes
    error = assert_raises(Mechanomeld::Error) do
      Mechanomeld::Document.new.load_incremental("not automerge".b)
    end
    assert_match(/could not load Automerge document/, error.message)
  end
end
