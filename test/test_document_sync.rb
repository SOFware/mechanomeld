# frozen_string_literal: true

require "test_helper"

class TestDocumentSync < Minitest::Test
  def setup
    base = Mechanomeld::Document.from({"title" => Mechanomeld::Text.new("shared")}).save
    @left = Mechanomeld::Document.load(base)
    @right = Mechanomeld::Document.load(base)
    @left_state = Mechanomeld::SyncState.new
    @right_state = Mechanomeld::SyncState.new
  end

  # Exchanges messages both ways until neither side has anything left to send.
  def sync(left = @left, right = @right, left_state = @left_state, right_state = @right_state)
    10.times do
      to_right = left.generate_sync_message(left_state)
      right.receive_sync_message(right_state, to_right) if to_right
      to_left = right.generate_sync_message(right_state)
      left.receive_sync_message(left_state, to_left) if to_left
      return if to_right.nil? && to_left.nil?
    end
    flunk "peers did not converge within 10 rounds"
  end

  def test_peers_converge_on_each_others_changes
    @left.change { |doc| doc["left"] = 1 }
    @right.change { |doc| doc["right"] = 2 }
    sync
    expected = {"title" => Mechanomeld::Text.new("shared"), "left" => 1, "right" => 2}
    assert_equal expected, @left.to_h
    assert_equal expected, @right.to_h
    assert_equal @left.heads, @right.heads
  end

  def test_nothing_to_send_once_peers_converge
    @left.change { |doc| doc["left"] = 1 }
    sync
    assert_nil @left.generate_sync_message(@left_state)
    assert_nil @right.generate_sync_message(@right_state)
  end

  def test_a_fresh_peer_receives_the_whole_document
    @left.change { |doc| doc["items"] = [1, 2, 3] }
    fresh = Mechanomeld::Document.new
    sync(@left, fresh, @left_state, Mechanomeld::SyncState.new)
    assert_equal @left.to_h, fresh.to_h
    assert_equal @left.heads, fresh.heads
  end

  def test_state_survives_encode_and_decode_mid_sync
    @left.change { |doc| doc["left"] = 1 }
    @right.change { |doc| doc["right"] = 2 }
    @right.receive_sync_message(@right_state, @left.generate_sync_message(@left_state))
    @left.receive_sync_message(@left_state, @right.generate_sync_message(@right_state))
    left_state = Mechanomeld::SyncState.decode(@left_state.encode)
    right_state = Mechanomeld::SyncState.decode(@right_state.encode)
    sync(@left, @right, left_state, right_state)
    assert_equal @left.to_h, @right.to_h
    assert_equal @left.heads, @right.heads
  end

  def test_messages_are_binary_strings
    message = @left.generate_sync_message(@left_state)
    assert_equal Encoding::BINARY, message.encoding
    refute_empty message
  end

  def test_receive_returns_self
    message = @left.generate_sync_message(@left_state)
    assert_same @right, @right.receive_sync_message(@right_state, message)
  end

  def test_generate_commits_pending_operations_like_save
    @left.put("k", 1)
    @left.generate_sync_message(@left_state)
    assert_nil @left.commit
  end

  def test_receive_commits_pending_operations_like_save
    message = @left.generate_sync_message(@left_state)
    @right.put("k", 1)
    @right.receive_sync_message(@right_state, message)
    assert_nil @right.commit
  end

  def test_a_message_from_an_unrelated_document_merges_it_in
    other = Mechanomeld::Document.from({"other" => 1})
    sync(other, @left, Mechanomeld::SyncState.new, @left_state)
    assert_equal({"title" => Mechanomeld::Text.new("shared"), "other" => 1}, @left.to_h)
  end

  def test_rejects_bytes_that_are_not_a_sync_message
    error = assert_raises(Mechanomeld::Error) do
      @left.receive_sync_message(@left_state, "not a message".b)
    end
    assert_match(/could not decode sync message/, error.message)
  end
end
