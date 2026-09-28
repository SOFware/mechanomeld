# frozen_string_literal: true

module Mechanomeld
  # Native methods (new, load, load_incremental, get, keys, length, put, delete, commit,
  # rollback, save, heads, includes_heads?, diff, generate_sync_message,
  # receive_sync_message) are defined in ext/mechanomeld/src/document.rs.
  class Document
    def self.from(hash, actor_id: nil)
      raise ArgumentError, "Automerge document root must be a Hash" unless hash.is_a?(Hash)

      doc = new(actor_id: actor_id)
      hash.each { |key, value| doc.put([key], value) }
      doc.commit
      doc
    end

    # Loads a document from the directory written by @automerge/automerge-repo's
    # NodeFSStorageAdapter. Snapshot chunks are applied before incremental ones, as
    # automerge-repo does; its sync-state files are not document data and are skipped.
    def self.load_repo(base_dir, document_id)
      document_dir = File.join(base_dir, document_id[0, 2], document_id[2..])
      chunks = %w[snapshot incremental].flat_map do |kind|
        Dir.glob(File.join(document_dir, kind, "*")).sort.select { |path| File.file?(path) }
      end
      raise Error, "no chunks for document #{document_id} in #{base_dir}" if chunks.empty?

      chunks.each_with_object(new) { |path, doc| doc.load_incremental(File.binread(path)) }
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
