# frozen_string_literal: true

require "bundler/gem_tasks"
require "minitest/test_task"
require "rb_sys/extensiontask"

GEMSPEC = Gem::Specification.load("mechanomeld.gemspec")

# No platform list: cross-gem builds set RUBY_TARGET, which rb_sys reads.
RbSys::ExtensionTask.new("mechanomeld", GEMSPEC) do |ext|
  ext.lib_dir = "lib/mechanomeld"
end

Minitest::TestTask.create
task test: :compile

task default: :test
