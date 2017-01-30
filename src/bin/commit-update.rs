extern crate git2;

fn main() {
    let repo = git2::Repository::discover(".").expect("Failed to discover repository");
    let change = find_modified_pkgbuild(&repo)
        .unwrap()
        .or_else(|| find_modified_submodule(&repo).unwrap());
    match change {
        Some(change) => {
            let new_version = evaluate_pkgbuild(&change.new_pkgbuild).unwrap();
            let message = if let Some(old_pkgbuild) = change.old_pkgbuild {
                let old_version = evaluate_pkgbuild(&old_pkgbuild).unwrap();
                format!("Update {} {} -> {}",
                        change.pkgname,
                        old_version,
                        new_version)
            } else {
                format!("Add {} {}", change.pkgname, new_version)
            };

            use std::os::unix::process::CommandExt;
            let err = std::process::Command::new("git").arg("commit").arg("-m").arg(message).exec();
            panic!("{:?}", err);
        }
        None => {
            println!("No PKGBUILD is modified");
            std::process::exit(1);
        }
    }
}

struct PKGBUILDChange {
    new_pkgbuild: Vec<u8>,
    old_pkgbuild: Option<Vec<u8>>,
    pkgname: String,
}

fn find_modified_pkgbuild(repo: &git2::Repository) -> Result<Option<PKGBUILDChange>, git2::Error> {
    use std::os::unix::ffi::OsStrExt;

    let mut new_pkgbuild_entry = None;

    for entry in try!(repo.index()).iter() {
        let path = entry.path.clone();
        let path = std::path::Path::new(std::ffi::OsStr::from_bytes(&path));
        let status = try!(repo.status_file(path));
        if status.intersects(git2::STATUS_INDEX_MODIFIED | git2::STATUS_INDEX_NEW) {
            if let Some(filename) = path.file_name() {
                if filename == "PKGBUILD" {
                    if let None = new_pkgbuild_entry {
                        new_pkgbuild_entry = Some(entry);
                    } else {
                        panic!("Multiple PKGBUILDs are modified");
                    }
                }
            }
        }
    }

    let new_pkgbuild_entry = match new_pkgbuild_entry {
        Some(entry) => entry,
        None => return Ok(None),
    };
    let new_pkgbuild_blob = try!(repo.find_blob(new_pkgbuild_entry.id));
    let new_pkgbuild_content = new_pkgbuild_blob.content().to_vec();
    let pkgbuild_path = std::path::Path::new(std::ffi::OsStr::from_bytes(&new_pkgbuild_entry.path));
    let head_oid = try!(repo.head()).target().unwrap();
    let head = try!(repo.find_commit(head_oid));
    let head_tree = try!(head.tree());
    let old_pkgbuild_content = match head_tree.get_path(pkgbuild_path) {
        Ok(entry) => {
            let blob = try!(repo.find_blob(entry.id()));
            Some(blob.content().to_vec())
        }
        Err(_) => None,
    };
    let pkgname_osstr =
        pkgbuild_path.parent().and_then(|p| p.file_name()).expect("Invalid PKGBUILD path");
    let pkgname = pkgname_osstr.to_os_string()
        .into_string()
        .expect("Invalid UTF-8 sequence is found at pkgname");

    Ok(Some(PKGBUILDChange {
        new_pkgbuild: new_pkgbuild_content,
        old_pkgbuild: old_pkgbuild_content,
        pkgname: pkgname,
    }))
}

fn find_modified_submodule(repo: &git2::Repository) -> Result<Option<PKGBUILDChange>, git2::Error> {
    // TODO
    Ok(None)
}

fn evaluate_pkgbuild(content: &[u8]) -> Result<String, std::io::Error> {
    use std::io::Write;

    let mut child = try!(std::process::Command::new("bash")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn());
    {
        let mut stdin = child.stdin.take().unwrap();
        try!(stdin.write_all(content));
        try!(stdin.write_all(b"\necho -n $pkgver-$pkgrel"));
    }
    let output = try!(child.wait_with_output());
    Ok(String::from_utf8(output.stdout).expect("Invalid UTF-8 sequence in PKGBUILD output"))
}
