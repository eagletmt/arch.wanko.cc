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
                format!(
                    "Update {} {} -> {}",
                    change.pkgname, old_version, new_version
                )
            } else {
                format!("Add {} {}", change.pkgname, new_version)
            };

            use std::os::unix::process::CommandExt;
            let err = std::process::Command::new("git")
                .arg("commit")
                .arg("-m")
                .arg(message)
                .exec();
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

    for entry in repo.index()?.iter() {
        let path = entry.path.clone();
        let path = std::path::Path::new(std::ffi::OsStr::from_bytes(&path));
        let status = repo.status_file(path)?;
        if status.is_index_modified() || status.is_index_new() {
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
    let new_pkgbuild_blob = repo.find_blob(new_pkgbuild_entry.id)?;
    let new_pkgbuild_content = new_pkgbuild_blob.content().to_vec();
    let pkgbuild_path = std::path::Path::new(std::ffi::OsStr::from_bytes(&new_pkgbuild_entry.path));
    let head_oid = repo.head()?.target().unwrap();
    let head = repo.find_commit(head_oid)?;
    let head_tree = head.tree()?;
    let old_pkgbuild_content = match head_tree.get_path(pkgbuild_path) {
        Ok(entry) => {
            let blob = repo.find_blob(entry.id())?;
            Some(blob.content().to_vec())
        }
        Err(_) => None,
    };

    Ok(Some(PKGBUILDChange {
        new_pkgbuild: new_pkgbuild_content,
        old_pkgbuild: old_pkgbuild_content,
        pkgname: path_to_pkgname(pkgbuild_path.parent().expect("Invalid PKGBUILD path")),
    }))
}

fn find_modified_submodule(repo: &git2::Repository) -> Result<Option<PKGBUILDChange>, git2::Error> {
    let mut modified_submodule = None;

    for submodule in repo.submodules()? {
        let status = repo.submodule_status(
            submodule.name().expect(
                "Invalid UTF-8 sequence is found \
                 at submodule's name",
            ),
            git2::SubmoduleIgnore::Dirty,
        )?;
        if status.is_index_modified() || status.is_index_added() {
            if let None = modified_submodule {
                modified_submodule = Some(submodule);
            } else {
                panic!("Multiple submodules are modified");
            }
        }
    }

    let modified_submodule = match modified_submodule {
        Some(submodule) => submodule,
        None => return Ok(None),
    };

    let path = repo
        .path()
        .parent()
        .unwrap_or(repo.path())
        .join(modified_submodule.path());
    let sub_repo = git2::Repository::open(path)?;
    let index_id = modified_submodule
        .index_id()
        .expect("Unable to get index id of the submodule");
    let new_pkgbuild_content = get_pkgbuild_content(&sub_repo, index_id, "PKGBUILD")?;
    let old_pkgbuild_content = if let Some(head_id) = modified_submodule.head_id() {
        Some(get_pkgbuild_content(&sub_repo, head_id, "PKGBUILD")?)
    } else {
        None
    };

    let pkgname = path_to_pkgname(modified_submodule.path());
    Ok(Some(PKGBUILDChange {
        new_pkgbuild: new_pkgbuild_content,
        old_pkgbuild: old_pkgbuild_content,
        pkgname: pkgname,
    }))
}

fn get_pkgbuild_content<P>(
    repo: &git2::Repository,
    commit_oid: git2::Oid,
    path: P,
) -> Result<Vec<u8>, git2::Error>
where
    P: AsRef<std::path::Path>,
{
    let commit = repo.find_commit(commit_oid)?;
    let tree = commit.tree()?;
    let tree_entry = tree.get_path(path.as_ref())?;
    let blob = repo.find_blob(tree_entry.id())?;
    Ok(blob.content().to_vec())
}

fn path_to_pkgname(path: &std::path::Path) -> String {
    path.file_name()
        .expect("Invalid PKGBUILD path")
        .to_os_string()
        .into_string()
        .expect("Invalid UTF-8 sequence is found at pkgname")
}

fn evaluate_pkgbuild(content: &[u8]) -> Result<String, std::io::Error> {
    use std::io::Write;

    let mut child = std::process::Command::new("bash")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()?;
    {
        let mut stdin = child.stdin.take().unwrap();
        stdin.write_all(content)?;
        stdin.write_all(b"\necho -n $pkgver-$pkgrel")?;
    }
    let output = child.wait_with_output()?;
    Ok(String::from_utf8(output.stdout).expect("Invalid UTF-8 sequence in PKGBUILD output"))
}
