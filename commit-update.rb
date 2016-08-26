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

Result = Struct.new(:pkgname, :old_pkgbuild, :new_pkgbuild)

def find_modified_pkgbuild(repo)
  modified_entries = []
  repo.index.each do |entry|
    path = entry[:path]
    if repo.status(path).include?(:index_modified) && File.basename(path) == 'PKGBUILD'
      modified_entries << entry
    end
  end

  if modified_entries.empty?
    return nil
  elsif modified_entries.size > 1
    abort 'Multiple PKGBUILDs are modified'
  end
  new_pkgbuild_entry = modified_entries.first

  Result.new.tap do |r|
    r.new_pkgbuild = repo.lookup(new_pkgbuild_entry[:oid]).content
    old_pkgbuild_entry = repo.head.target.tree.path(new_pkgbuild_entry[:path])
    r.old_pkgbuild = repo.lookup(old_pkgbuild_entry[:oid]).content
    r.pkgname = Pathname.new(new_pkgbuild_entry[:path]).parent.basename.to_s
  end
end

def find_modified_submodule(repo)
  modified_submodules = []
  repo.submodules.each do |submodule|
    if submodule.modified_in_index? || submodule.added_to_index?
      modified_submodules << submodule
    end
  end

  if modified_submodules.empty?
    return nil
  elsif modified_submodules.size > 1
    abort 'Multiple submodules are modified'
  end
  submodule = modified_submodules.first

  Result.new.tap do |r|
    r.pkgname = Pathname.new(submodule.path).basename.to_s
    submodule.repository.index.each do |entry|
      if entry[:path] == 'PKGBUILD'
        r.new_pkgbuild = submodule.repository.lookup(entry[:oid]).content
        if submodule.head_oid
          old_head_commit = submodule.repository.lookup(submodule.head_oid)
          old_pkgbuild_entry =  old_head_commit.tree.path(entry[:path])
          r.old_pkgbuild = submodule.repository.lookup(old_pkgbuild_entry[:oid]).content
        end
      end
    end
  end
end

repo = Rugged::Repository.discover('.')
result = find_modified_pkgbuild(repo) || find_modified_submodule(repo)

if result
  new_pkgver = PKGBUILD.new(result.new_pkgbuild).pkgver
  message =
    if result.old_pkgbuild
      old_pkgver = PKGBUILD.new(result.old_pkgbuild).pkgver
      "Update #{result.pkgname} #{old_pkgver} -> #{new_pkgver}"
    else
      "Add #{result.pkgname} #{new_pkgver}"
    end

  exec('git', 'commit', '-m', message)
else
  abort 'No PKGBUILD is modified'
end
