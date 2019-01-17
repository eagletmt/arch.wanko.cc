extern crate git2;

fn main() {
    let repo = git2::Repository::discover(".").expect("Failed to discover repository");
    show_submodules_diff(&repo).unwrap();
}

fn show_submodules_diff(repo: &git2::Repository) -> Result<(), git2::Error> {
    for submodule in repo.submodules()? {
        let status = repo.submodule_status(
            submodule.name().expect(
                "Invalid UTF-8 sequence is found \
                 at submodule's name",
            ),
            git2::SubmoduleIgnore::Dirty,
        )?;
        if status.is_wd_modified() {
            let head_id = submodule.head_id().expect("Unable to get HEAD id");
            let workdir_id = submodule.workdir_id().expect("Unable to get workdir id");
            let path = repo
                .path()
                .parent()
                .unwrap_or(repo.path())
                .join(submodule.path());
            std::process::Command::new("git")
                .env("LESS", "RX")
                .arg("--paginate")
                .arg("-C")
                .arg(path)
                .arg("diff")
                .arg(format!("{}", head_id))
                .arg(format!("{}", workdir_id))
                .status()
                .expect("Failed to run git-diff");
        }
    }

    Ok(())
}
