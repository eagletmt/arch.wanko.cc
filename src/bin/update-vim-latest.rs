use futures::stream::StreamExt as _;
use sha2::Digest as _;
use tokio::io::AsyncWriteExt as _;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    const OWNER: &str = "vim";
    const REPO: &str = "vim";
    const PKGNAME: &str = "vim-latest";
    const ACCEPT: &str = "application/vnd.github.v3+json";

    let client = reqwest::Client::new();
    let resp = client
        .get(&format!(
            "https://api.github.com/repos/{}/{}/git/refs/tags",
            OWNER, REPO
        ))
        .header(
            reqwest::header::ACCEPT,
            reqwest::header::HeaderValue::from_static(ACCEPT),
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
    let tags: Vec<Tag> = resp.json().await?;
    let tag_re = regex::Regex::new(r#"\Arefs/tags/v(.+)\z"#)?;
    let mut tags: std::collections::HashMap<String, String> = tags
        .into_iter()
        .map(|tag| {
            (
                tag.object.sha,
                tag_re
                    .captures(&tag.ref_)
                    .unwrap()
                    .get(1)
                    .unwrap()
                    .as_str()
                    .to_owned(),
            )
        })
        .collect();

    let resp = client
        .get(&format!(
            "https://api.github.com/repos/{}/{}/commits",
            REPO, OWNER
        ))
        .header(
            reqwest::header::ACCEPT,
            reqwest::header::HeaderValue::from_static(ACCEPT),
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
    let commits: Vec<Commit> = resp.json().await?;
    let mut tag = None;
    for commit in commits {
        if let Some(t) = tags.remove(&commit.sha) {
            tag = Some(t);
            break;
        }
    }
    let tag = tag.expect("No tags found");
    let parts: Vec<_> = tag.split(".").collect();
    assert_eq!(parts.len(), 3);
    let baseversion = format!("{}.{}", parts[0], parts[1]);
    let patchlevel = parts[2];

    let resp = client
        .get(&format!(
            "https://github.com/{}/{}/archive/v{}.tar.gz",
            OWNER, REPO, tag
        ))
        .send()
        .await?
        .error_for_status()?;
    let sources = std::path::Path::new(PKGNAME).join("sources");
    tokio::fs::create_dir_all(&sources).await?;
    let dest = sources.join(&format!("vim-{}.tar.gz", tag));
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

pkgname=vim-latest
_baseversion={{ baseversion }}
_patchlevel={{ patchlevel }}
pkgver=${_baseversion}.${_patchlevel}
pkgrel=1
pkgdesc='Vi Improved, a highly configurable, improved version of the vi text editor'
arch=(i686 x86_64)
license=('custom:vim')
url="http://www.vim.org"
depends=('gpm')
makedepends=('perl' 'python' 'ruby' 'luajit')
conflicts=(vim vim-runtime)
provides=(vim=$pkgver vim-runtime=$pkgver)
source=(vim-$pkgver.tar.gz::https://github.com/vim/vim/archive/v$pkgver.tar.gz)

prepare() {
  cd "$srcdir/vim-$pkgver"

  sed -i 's|^.*\(#define SYS_.*VIMRC_FILE.*"\) .*$|\1|' src/feature.h
  sed -i 's|^.*\(#define VIMRC_FILE.*"\) .*$|\1|' src/feature.h
}

build()
{
  cd "$srcdir/vim-$pkgver"

  ./configure --prefix=/usr --localstatedir=/var/lib/vim --mandir=/usr/share/man \
    --with-features=huge --enable-gpm --enable-acl --with-x=no --disable-gui \
    --enable-multibyte --enable-cscope --disable-netbeans \
    --enable-perlinterp=dynamic --enable-python3interp=dynamic \
    --enable-rubyinterp=dynamic --enable-luainterp=dynamic --with-luajit \
    --with-compiledby='Kohei Suzuki <eagletmt@gmail.com>' \
    --disable-smack

  make
}

package() {
  cd "$srcdir/vim-$pkgver"
  make -j1 VIMRCLOC=/etc DESTDIR="$pkgdir" install

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

sha256sums=('{{ sha256 }}')
"#
        .trim_start(),
    )?;
    let pkgbuild = handlebars.render(
        "PKGBUILD",
        &Input {
            baseversion,
            patchlevel: patchlevel.to_owned(),
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
struct Tag {
    #[serde(rename = "ref")]
    ref_: String,
    object: Object,
}
#[derive(Debug, serde::Deserialize)]
struct Object {
    sha: String,
}
#[derive(Debug, serde::Deserialize)]
struct Commit {
    sha: String,
}

#[derive(serde::Serialize)]
struct Input {
    baseversion: String,
    patchlevel: String,
    sha256: String,
}
