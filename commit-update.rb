#!/usr/bin/env ruby
ENV['BUNDLE_GEMFILE'] ||= File.expand_path('Gemfile', __dir__)
require 'bundler/setup'
require 'pathname'
require 'open3'
require 'rugged'

class PKGBUILD
  def initialize(content)
    @content = content
  end

  def pkgver
    out, err, status = Open3.capture3('bash', stdin_data: "#{@content}\necho $pkgver")
    unless status.success?
      puts out
      $stderr.puts err
      $stderr.puts "Exit with #{status.exitstatus}"
      abort 'Cannot retrieve pkgver'
    end
    out.chomp
  end
end

repo = Rugged::Repository.new(__dir__)
index = repo.index
modified_entries = []
index.each do |entry|
  path = entry[:path]
  if repo.status(path).include?(:index_modified) && File.basename(path) == 'PKGBUILD'
    modified_entries << entry
  end
end

if modified_entries.empty?
  abort 'No PKGBUILD is modified'
elsif modified_entries.size > 1
  abort 'Multiple PKGBUILDs are modified'
end
new_pkgbuild_entry = modified_entries.first

new_content = repo.lookup(new_pkgbuild_entry[:oid]).content
old_pkgbuild_entry = repo.head.target.tree.path(new_pkgbuild_entry[:path])
old_content = repo.lookup(old_pkgbuild_entry[:oid]).content

old_pkgver = PKGBUILD.new(old_content).pkgver
new_pkgver = PKGBUILD.new(new_content).pkgver
pkgname = Pathname.new(new_pkgbuild_entry[:path]).parent.basename.to_s
message = "Update #{pkgname} #{old_pkgver} -> #{new_pkgver}"

exec('git', 'commit', '-m', message)
