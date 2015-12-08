#!/usr/bin/env ruby
require 'digest'
require 'erb'
require 'json'
require 'net/http'
require 'openssl'
require 'pathname'

owner = 'vim'
repo = 'vim'
pkgname = 'vim-latest'

ACCEPT = 'application/vnd.github.v3+json'

def find_latest_tag(all_tags, commits)
  commits.each do |commit|
    if tag = all_tags[commit['sha']]
      return tag
    end
  end
  nil
end

https = Net::HTTP.new('api.github.com', 443)
https.use_ssl = true
https.verify_mode = OpenSSL::SSL::VERIFY_PEER
tag = https.start do
  req = Net::HTTP::Get.new("/repos/#{owner}/#{repo}/git/refs/tags")
  req['Accept'] = ACCEPT
  res = https.request(req)
  if res.code != '200'
    raise "HTTP Error: #{res.code}: #{res.message}"
  end
  all_tags = {}
  JSON.parse(res.body).each do |ref|
    all_tags[ref['object']['sha']] = ref['ref'][%r{\Arefs/tags/(.+)\z}, 1]
  end

  req = Net::HTTP::Get.new("/repos/#{owner}/#{repo}/commits")
  req['Accept'] = ACCEPT
  res = https.request(req)
  if res.code != '200'
    raise "HTTP Error: #{res.code}: #{res.message}"
  end
  find_latest_tag(all_tags, JSON.parse(res.body))
end

if m = tag.match(/\Av(\d+).(\d+).(\d+)/)
  baseversion = "#{m[1]}.#{m[2]}"
  patchlevel = m[3]
else
  raise "Malformed tag name: #{tag}"
end

version = "#{baseversion}.#{patchlevel}"
url = "https://github.com/#{owner}/#{repo}/archive/#{tag}.tar.gz"
dest = Pathname.new(__dir__).join('sources', "vim-#{version}.tar.gz")
dest.parent.mkpath
unless system('curl', '-vL', '-o', dest.to_s, url)
  abort "curl error"
end

source_digest_sha1 = Digest::SHA1.file(dest.to_s).hexdigest
pkgbuild = Pathname.new(__dir__).join('PKGBUILDs', pkgname, 'PKGBUILD')
pkgbuild.open('w') do |f|
  f.puts ERB.new(DATA.read, nil, '-').result(binding)
end

__END__
# Maintainer: Kohei Suzuki <eagletmt@gmail.com>

pkgname=vim-latest
_baseversion=<%= baseversion %>
_patchlevel=<%= patchlevel %>
pkgver=${_baseversion}.${_patchlevel}
pkgrel=1
pkgdesc='Vi Improved, a highly configurable, improved version of the vi text editor'
arch=(i686 x86_64)
license=('custom:vim')
url="http://www.vim.org"
depends=('gpm')
makedepends=('perl' 'python' 'python2' 'ruby' 'luajit')
conflicts=(vim vim-runtime)
provides=(vim=$pkgver vim-runtime=$pkgver)
source=(vim-$pkgver.tar.gz::https://github.com/<%= owner %>/<%= repo %>/archive/v$pkgver.tar.gz)

prepare() {
  cd "$srcdir/vim-$pkgver"

  sed -i 's|set dummy python;|set dummy python2;|g' src/auto/configure
  sed -i 's|^.*\(#define SYS_.*VIMRC_FILE.*"\) .*$|\1|' src/feature.h
  sed -i 's|^.*\(#define VIMRC_FILE.*"\) .*$|\1|' src/feature.h
}

build()
{
  cd "$srcdir/vim-$pkgver"

  ./configure --prefix=/usr --localstatedir=/var/lib/vim --mandir=/usr/share/man \
  --with-features=huge --enable-gpm --enable-acl --with-x=no --disable-gui \
  --enable-multibyte --enable-cscope \
  --enable-perlinterp=dynamic --enable-pythoninterp=yes --enable-python3interp=yes \
  --enable-rubyinterp=dynamic --enable-luainterp=dynamic --with-luajit \
  --with-compiledby='Kohei Suzuki <eagletmt@gmail.com>' \
  --disable-smack

  make
}

package() {
  cd "$srcdir/vim-$pkgver"
  make VIMRCLOC=/etc DESTDIR="$pkgdir" install

  cd "$pkgdir/usr/bin"
  rm ex view                # provided by (n)vi in core

  # delete some manpages
  cd "$pkgdir/usr/share/man"
  rm -f {*/,}man1/ex.1 {*/,}man1/view.1      # provided by (n)vi
  rm -f {*/,}man1/evim.1                     # this does not make sense in the console version

  local _shortver=${_baseversion/./}
  # patch runtime
  cd "$pkgdir/usr/share/vim/vim$_shortver/"
  sed -i "s/rpmsave/pacsave/;s/rpmnew/pacnew/;s/,\*\.ebuild/\0,PKGBUILD*,*.install/" filetype.vim

  # fix FS#17216
  sed -i 's|messages,/var|messages,/var/log/messages.log,/var|' \
    "$pkgdir/usr/share/vim/vim$_shortver/filetype.vim"


  install -dm755 "$pkgdir/usr/share/licenses/$pkgname"
  cd "$pkgdir/usr/share/licenses/$pkgname"
  ln -s ../../vim/vim$_shortver/doc/uganda.txt license.txt
}

check() {
  cd "$srcdir/vim-$pkgver"
  make -j1 test
}

# vim:set ts=2 sw=2 et:

sha1sums=('<%= source_digest_sha1 %>')
