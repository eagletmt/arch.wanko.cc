#!/usr/bin/env ruby
ENV['BUNDLE_GEMFILE'] ||= File.expand_path('Gemfile', __dir__)
require 'bundler/setup'
require 'rugged'

def show_submodule_diff(submodule)
  system('git', '--git-dir', submodule.repository.path, 'diff', submodule.head_oid, submodule.repository.head.target.oid)
end

repo = Rugged::Repository.discover('.')
repo.submodules.each do |submodule|
  if submodule.modified_in_workdir?
    show_submodule_diff(submodule)
  end
end
