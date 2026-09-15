# frozen_string_literal: true

require "bundler/gem_tasks"
require "minitest/test_task"
require "rb_sys/extensiontask"
require "reissue/gem"

GEMSPEC = Gem::Specification.load("mechanomeld.gemspec")

# No platform list: cross-gem builds set RUBY_TARGET, which rb_sys reads.
RbSys::ExtensionTask.new("mechanomeld", GEMSPEC) do |ext|
  ext.lib_dir = "lib/mechanomeld"
end

Minitest::TestTask.create
task test: :compile

Reissue::Task.create :reissue do |task|
  task.version_file = "lib/mechanomeld/version.rb"
  task.fragment = :git
  # The post-release bump goes on a pushed branch; the shared release workflow opens its PR.
  # (0.5.1's default, stated explicitly: it is the "Option A" the shared workflow expects.)
  task.push_reissue = :branch
end

task default: :test
