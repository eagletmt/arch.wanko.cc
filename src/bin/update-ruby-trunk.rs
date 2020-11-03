use futures::stream::StreamExt as _;
use sha2::Digest as _;
use tokio::io::AsyncWriteExt as _;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    const OWNER: &str = "ruby";
    const REPO: &str = "ruby";
    const PKGNAME: &str = "ruby-trunk";

    let client = reqwest::Client::new();
    let resp = client
        .get(&format!(
            "https://api.github.com/repos/{}/{}/commits",
            OWNER, REPO
        ))
        .header(
            reqwest::header::ACCEPT,
            reqwest::header::HeaderValue::from_static("application/vnd.github.v3+json"),
        )
        .header(
            reqwest::header::USER_AGENT,
            reqwest::header::HeaderValue::from_static(
                "arch.wanko.cc/0.0.0 https://github.com/eagletmt/arch.wanko.cc",
            ),
        )
        .send()
        .await?
        .error_for_status()?;
    let commits: Vec<RepositoryCommit> = resp.json().await?;
    let head_commit = &commits[0];

    let resp = client
        .get(&format!(
            "https://github.com/{}/{}/archive/{}.tar.gz",
            OWNER, REPO, head_commit.sha
        ))
        .send()
        .await?
        .error_for_status()?;
    let sources = std::path::Path::new(PKGNAME).join("sources");
    tokio::fs::create_dir_all(&sources).await?;
    let dest = sources.join(&format!("ruby-{}.tar.gz", head_commit.sha));
    let file = tokio::fs::File::create(dest).await?;
    let mut writer = tokio::io::BufWriter::new(file);
    let mut stream = resp.bytes_stream();
    let mut digest = sha2::Sha256::new();
    while let Some(item) = stream.next().await {
        let b = item?;
        writer.write_all(&b).await?;
        digest.update(&b);
    }
    writer.shutdown().await?;

    let mut handlebars = handlebars::Handlebars::new();
    handlebars.set_strict_mode(true);
    handlebars.register_template_string(
        "PKGBUILD",
        r#"
# Maintainer: Kohei Suzuki <eagletmt@gmail.com>

_commit={{ commit_sha }}
_shortcommit={{ commit_sha_short }}
pkgname='ruby-trunk'
pkgver={{ commit_date }}
pkgrel=1
pkgdesc='An object-oriented language for quick and easy programming'
arch=('i686' 'x86_64')
url='http://www.ruby-lang.org/en/'
depends=('gdbm' 'openssl' 'libffi' 'libyaml' 'gmp' 'zlib')
makedepends=('ruby')  # for baseruby
provides=("ruby=3.0.0" 'rubygems' 'ruby-irb' 'ruby-reline')
conflicts=('ruby' 'rubygems' 'ruby-irb' 'ruby-reline')
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

sha256sums=('{{ sha256 }}'
            '4bb7eb2fe66e396ed16b589cdb656831407b39ad4e138d88536754c0448ac614')
"#
        .trim_start(),
    )?;
    let pkgbuild = handlebars.render(
        "PKGBUILD",
        &Input {
            commit_sha: head_commit.sha.to_owned(),
            commit_sha_short: head_commit.sha[0..10].to_owned(),
            commit_date: head_commit
                .commit
                .committer
                .date
                .format("%Y%m%d")
                .to_string(),
            sha256: format!("{:x}", digest.finalize()),
        },
    )?;
    let pkgbuild_path = std::path::Path::new(PKGNAME)
        .join("PKGBUILDs")
        .join(PKGNAME)
        .join("PKGBUILD");
    let mut file = tokio::fs::File::create(pkgbuild_path).await?;
    file.write_all(pkgbuild.as_bytes()).await?;
    file.shutdown().await?;

    Ok(())
}

#[derive(Debug, serde::Deserialize)]
struct RepositoryCommit {
    sha: String,
    commit: Commit,
}
#[derive(Debug, serde::Deserialize)]
struct Commit {
    committer: CommitAuthor,
}
#[derive(Debug, serde::Deserialize)]
struct CommitAuthor {
    date: chrono::DateTime<chrono::Utc>,
}

#[derive(serde::Serialize)]
struct Input {
    commit_sha: String,
    commit_sha_short: String,
    commit_date: String,
    sha256: String,
}
