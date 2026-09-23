# frozen_string_literal: true

require "test_helper"

class TestSyncState < Minitest::Test
  def test_encode_gives_a_binary_string
    encoded = Mechanomeld::SyncState.new.encode
    assert_equal Encoding::BINARY, encoded.encoding
    refute_empty encoded
  end

  def test_decode_round_trips_encode
    encoded = Mechanomeld::SyncState.new.encode
    assert_equal encoded, Mechanomeld::SyncState.decode(encoded).encode
  end

  def test_decode_rejects_invalid_bytes
    error = assert_raises(Mechanomeld::Error) { Mechanomeld::SyncState.decode("nope".b) }
    assert_match(/could not decode sync state/, error.message)
  end
end
