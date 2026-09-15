# frozen_string_literal: true

module Mechanomeld
  # Native methods (new, load, get, keys, length, put, delete, commit, rollback, save)
  # are defined in ext/mechanomeld/src/document.rs.
  class Document
    def self.from(hash, actor_id: nil)
      raise ArgumentError, "Automerge document root must be a Hash" unless hash.is_a?(Hash)

      doc = new(actor_id: actor_id)
      hash.each { |key, value| doc.put([key], value) }
      doc.commit
      doc
    end

    def [](path)
      get(path)
    end

    def []=(path, value)
      put(path, value)
    end

    def to_h
      get([])
    end
    alias_method :to_hash, :to_h

    def change(message: nil, timestamp: nil)
      yield self
      commit(message: message, timestamp: timestamp)
      self
    rescue Exception
      rollback
      raise
    end
  end
end
