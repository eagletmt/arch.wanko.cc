#!/usr/bin/env ruby
require 'digest'
require 'erb'
require 'json'
require 'net/http'
require 'openssl'
require 'pathname'
require 'time'

owner = 'ruby'
repo = 'ruby'
pkgname = 'ruby-trunk'

head_commit = Net::HTTP.start('api.github.com', 443, use_ssl: true) do |https|
  req = Net::HTTP::Get.new("/repos/#{owner}/#{repo}/commits")
  req['Accept'] = 'application/vnd.github.v3+json'
  res = https.request(req)
  if res.code != '200'
    raise "HTTP Error: #{res.code}: #{res.message}"
  end
  JSON.parse(res.body).first
end

commit_sha = head_commit['sha']
commit_date = Time.parse(head_commit['commit']['committer']['date']).strftime('%Y%m%d')

url = "https://github.com/ruby/ruby/archive/#{commit_sha}.tar.gz"
dest = Pathname.new(__dir__).join('sources', "ruby-#{commit_sha}.tar.gz")
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

_commit=<%= commit_sha %>
_shortcommit=<%= commit_sha[0, 10] %>
pkgname='ruby-trunk'
pkgver=<%= commit_date %>
pkgrel=1
pkgdesc='An object-oriented language for quick and easy programming'
arch=('i686' 'x86_64')
url='http://www.ruby-lang.org/en/'
depends=('gdbm' 'openssl' 'libffi' 'libyaml' 'gmp' 'zlib')
makedepends=('ruby')  # for baseruby
provides=("ruby=3.0.0" 'rubygems')
conflicts=('ruby')
backup=('etc/gemrc')
install='ruby.install'
license=('BSD' 'custom')
options=('!emptydirs' '!strip' 'staticlibs')
source=("ruby-${_commit}.tar.gz::https://github.com/ruby/ruby/archive/${_commit}.tar.gz"
        'gemrc')

build() {
  cd ruby-${_commit}

  autoreconf -i
  cat > revision.h << EOS
#define RUBY_REVISION "${_shortcommit}"
#define RUBY_FULL_REVISION "${_commit}"
EOS
  PKG_CONFIG=/usr/bin/pkg-config ./configure \
    --prefix=/usr \
    --sysconfdir=/etc \
    --localstatedir=/var \
    --sharedstatedir=/var/lib \
    --libexecdir=/usr/lib/ruby \
    --enable-shared \
    --disable-rpath \
    --with-dbm-type=gdbm_compat \
    --enable-debug-env \
    --disable-install-doc \
    CFLAGS="$CFLAGS -ggdb3"

  make
}

check() {
  cd ruby-${_commit}

  make test
}

package() {
  cd ruby-${_commit}

  make DESTDIR="${pkgdir}" install

  install -D -m644 ${srcdir}/gemrc "${pkgdir}/etc/gemrc"

  install -D -m644 COPYING "${pkgdir}/usr/share/licenses/ruby/LICENSE"
  install -D -m644 BSDL "${pkgdir}/usr/share/licenses/ruby/BSDL"
}

sha1sums=('<%= source_digest_sha1 %>'
          'de4b760b7e2cd9af88ca67536ce37b950f1ee514')
