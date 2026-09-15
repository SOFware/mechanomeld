# frozen_string_literal: true

require_relative "mechanomeld/version"
require_relative "mechanomeld/error"

# Precompiled gems ship one binary per Ruby minor version; source builds put it in lib/mechanomeld.
begin
  RUBY_VERSION =~ /(\d+\.\d+)/
  require_relative "mechanomeld/#{Regexp.last_match(1)}/mechanomeld"
rescue LoadError
  require_relative "mechanomeld/mechanomeld"
end
