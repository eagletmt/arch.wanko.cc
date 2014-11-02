###
# Compass
###

# Change Compass configuration
# compass_config do |config|
#   config.output_style = :compact
# end

###
# Page options, layouts, aliases and proxies
###

# Per-page layout changes:
#
# With no layout
# page "/path/to/file.html", :layout => false
#
# With alternative layout
# page "/path/to/file.html", :layout => :otherlayout
#
# A path which all have the same layout
# with_layout :admin do
#   page "/admin/*"
# end

# Proxy pages (http://middlemanapp.com/dynamic-pages/)
# proxy "/this-page-has-no-template.html", "/template-file.html", :locals => {
#  :which_fake_page => "Rendering a fake page with a local variable" }

###
# Helpers
###

# Automatic image dimensions on image_tag helper
# activate :automatic_image_sizes

# Reload the browser automatically whenever files change
# activate :livereload

# Methods defined in the helpers block are available in templates
# helpers do
#   def some_helper
#     "Helping"
#   end
# end

###
# Gem
###
require 'slim'

set :css_dir, 'stylesheets'

set :js_dir, 'javascripts'

set :images_dir, 'images'

require 'dotenv'
Dotenv.load!

require 'tempfile'
require 'aws-sdk-resources'
require 'akabei/repository'

class S3Repository
  def initialize(name)
    @name = name
    @bucket = Aws::S3::Resource.new(region: ENV['REGION']).bucket(ENV['BUCKET'])
  end

  def packages(arch)
    f = Tempfile.new("#{@name}.#{arch}.db")
    f.binmode
    @bucket.object("#{@name}/os/#{arch}/#{@name}.db").get do |chunk|
      f.write(chunk)
    end
    f.close
    repo = Akabei::Repository.load(f.path)
    repo.each
  end
end

data.repositories.each do |repo|
  proxy "#{repo}/index.html", 'repository/index.html', locals: { repo: S3Repository.new(repo), repo_name: repo }
end
ignore 'repository/index.html'

# Build-specific configuration
configure :build do
  # For example, change the Compass output style for deployment
  # activate :minify_css

  # Minify Javascript on build
  # activate :minify_javascript

  # Enable cache buster
  # activate :asset_hash

  # Use relative URLs
  # activate :relative_assets

  # Or use a different image path
  # set :http_path, "/Content/images/"
end

activate :s3_sync do |s3_sync|
  s3_sync.bucket = ENV['BUCKET']
  s3_sync.region = ENV['REGION']
  s3_sync.delete = false
  s3_sync.reduced_redundancy_storage = true
end
