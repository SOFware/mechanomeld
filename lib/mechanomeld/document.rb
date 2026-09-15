# frozen_string_literal: true

module Mechanomeld
  # Native methods are defined in ext/mechanomeld/src/document.rs.
  class Document
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
  end
end
