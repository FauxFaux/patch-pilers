use anyhow::anyhow;
use anyhow::Result;

pub fn multi_status(repo: &git2::Repository) -> Result<()> {
    for branch in repo.branches(None)? {
        let (branch, _type) = branch?;
        let name = branch.name()?.ok_or_else(|| {
            anyhow!(
                "utf-8 branch names please, none of this {:?}",
                branch.name_bytes().map(|x| String::from_utf8_lossy(x))
            )
        })?.to_string();
        let branch_commit = branch.into_reference().peel_to_commit()?;

        let branch = repo.revparse(&name)?;

        let mut walk = repo.revwalk()?;
        walk.push(branch_commit.id())?;
        let base = repo.merge_base(branch_commit.id(), repo.head()?.peel_to_commit()?.id())?;
        walk.hide(base)?;

        println!("{} {}", walk.count(), name);
        // for commit in walk {
        //     let commit = commit?;
        //     let commit = repo.find_commit(commit)?;
        //     println!(" - {} {:?}", commit.id(), commit.summary());
        // }
    }
    Ok(())
}
