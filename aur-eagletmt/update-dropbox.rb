#!/usr/bin/env ruby
require 'pathname'
require 'digest/sha2'
require 'erb'

class Source
  class << self
    def dir
      @dir
    end

    def dir=(dir)
      @dir = dir
    end
  end

  def initialize(arch, version)
    @arch = arch
    @version = version
  end

  def path
    self.class.dir.join("dropbox-lnx.#{@arch}-#{@version}.tar.gz")
  end

  def exist?
    path.readable?
  end

  def download
    puts "Get #{url}"
    system('curl', '--silent', '--fail', '-o', path.to_s, url).tap do
      "Done #{url}"
    end
  end

  def url
    "https://d1ilhw0800yew8.cloudfront.net/client/#{path.basename}"
  end

  def hexdigest
    Digest::SHA512.file(path.to_s).hexdigest
  end
end

newver = ARGV[0] or exit(1)
Source.dir = Pathname.new(__dir__).join('sources')
sources = %w[x86 x86_64].map do |arch|
  [arch, Source.new(arch, newver)]
end.to_h

downloaders = sources.map do |_, source|
  Thread.start do
    source.exist? || source.download
  end
end

ok = true
downloaders.each do |downloader|
  v = downloader.value
  ok = ok && v
end
if !ok
  exit 2
end

pkgbuild = Pathname.new(__dir__).join('PKGBUILDs', 'dropbox', 'PKGBUILD')
pkgbuild.open('w') do |f|
  f.puts ERB.new(DATA.read, nil, '-').result(binding)
end

__END__
# Maintainer: Massimiliano Torromeo <massimiliano.torromeo@gmail.com>
# Contributor: Tom < tomgparchaur at gmail dot com >
# Contributor: David Manouchehri <d@32t.ca>

pkgname=dropbox
pkgver=<%= newver %>
pkgrel=1
pkgdesc="A free service that lets you bring your photos, docs, and videos anywhere and share them easily."
arch=("i686" "x86_64")
url="http://www.dropbox.com"
license=(custom)
depends=("dbus-glib" "gtk2" "libsm")
optdepends=(
    'ufw-extras: ufw rules for dropbox'
)
conflicts=("dropbox-experimental")
options=('!strip' '!upx')

_source_arch="x86"
[ "$CARCH" = "x86_64" ] && _source_arch="x86_64"

sha512sums=('<%= sources['x86'].hexdigest %>'
            'b1a2ca11479c9f243c0368d79b36ef87910311af2dd126a3291438083544ed10a640143a58e73be1d27bf016c114e668ea504ed6eed6955370bfcac309e5fb7d'
            'b3e0701afe90693b99d5e23bad6b8637bc27611a42c695d12b3b990d98bf010371b266322cd54c60ffd654ed44f56a85b1fcb51b30db991af60043dc22bf1897'
            'f688115daa8930dffd6e27a7113b137972c20918297c6178bb7e8f820add325d34d452f8bf6bb73fa6b2de73ffa028d27457ed2df390687af8841d9425ebab3e'
            'b08a50766681a55e3bf9f1721549218996dd4dbef183dce4967622a98a52fdcc47325de99794b40462692213bbe390f659cf48023b407ae4fce81997af4d46e2')
[ "$CARCH" = "x86_64" ] && sha512sums[0]='<%= sources['x86_64'].hexdigest %>'

source=("https://dl-web.dropbox.com/u/17/${pkgname}-lnx.${_source_arch}-${pkgver}.tar.gz"
        "dropbox.png"
        "dropbox.desktop"
        "terms.txt"
        "dropbox.service")

package() {
	install -d "$pkgdir/opt"
	cp -R "$srcdir/.dropbox-dist" "$pkgdir/opt/dropbox"

	find "$pkgdir/opt/dropbox/" -type f -exec chmod 644 {} \;
	chmod 755 "$pkgdir/opt/dropbox/dropboxd"
	chmod 755 "$pkgdir/opt/dropbox/dropbox-lnx.${_source_arch}-${pkgver}/dropbox"

	install -d "$pkgdir/usr/bin"
	ln -s "/opt/dropbox/dropboxd" "$pkgdir/usr/bin/dropboxd"

	install -Dm644 "$srcdir/dropbox.desktop" "$pkgdir/usr/share/applications/dropbox.desktop"
	install -Dm644 "$srcdir/dropbox.png" "$pkgdir/usr/share/pixmaps/dropbox.png"
	install -Dm644 "$srcdir/terms.txt" "$pkgdir/usr/share/licenses/$pkgname/terms.txt"
	install -Dm644 "$srcdir/dropbox.service" "$pkgdir/usr/lib/systemd/system/dropbox@.service"
}
