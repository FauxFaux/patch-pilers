use anyhow::anyhow;
use anyhow::bail;
use anyhow::ensure;
use anyhow::Result;
use git2::BranchType;
use git2::Delta;
use git2::ErrorCode;
use git2::Index;
use git2::Oid;
use regex::Regex;

pub fn add_renames(repo: &git2::Repository, since: &str) -> Result<()> {
    let local_branch_name = repo.head()?;
    let local_branch_name = match local_branch_name.name() {
        Some(name) if name.starts_with("refs/heads/") => &name["refs/heads/".len()..],
        other => bail!("not on local branch: {:?}", other),
    };

    let new_branch_name = format!("pp-rename/{}", local_branch_name);
    match repo.find_branch(&new_branch_name, BranchType::Local) {
        Err(e) if e.code() == ErrorCode::NotFound => (),
        Err(e) => bail!(e),
        Ok(_) => bail!("temporary branch already exists: {:?}", new_branch_name),
    };

    let rename_regex = Regex::new("^[a-z]+:")?;

    let bottom = repo.revparse_single(since)?;

    repo.branch(&new_branch_name, &bottom.peel_to_commit()?, false)?;

    let mut walk = repo.revwalk()?;
    walk.push_head()?;
    walk.hide(bottom.id())?;
    let mut commits = walk.collect::<Result<Vec<Oid>, git2::Error>>()?;
    commits.reverse();

    let mut parent = bottom.peel_to_commit()?;

    for id in commits {
        let commit = repo.find_commit(id)?;
        ensure!(
            1 == commit.parent_count(),
            "there are merges in the history, I give up"
        );
        println!("{:?} {:?}", id, commit.message());
        let parent_tree = commit.parent(0)?.tree()?;

        let diff = repo.diff_tree_to_tree(Some(&parent_tree), Some(&commit.tree()?), None)?;
        let deltas = diff
            .deltas()
            .map(|d| d.status())
            .take(3)
            .collect::<Vec<_>>();

        if deltas == [Delta::Deleted, Delta::Added] || deltas == [Delta::Added, Delta::Deleted] {
            let mut deltas = diff.deltas();
            let left = deltas.next().expect("checked");
            let right = deltas.next().expect("checked");
            assert!(deltas.next().is_none(), "checked");
            let (deleted, added) = if left.status() == Delta::Deleted {
                (left, right)
            } else {
                (right, left)
            };

            assert_eq!(deleted.status(), Delta::Deleted);
            assert_eq!(added.status(), Delta::Added);

            println!(
                "deleted: {:?}, added: {:?}",
                deleted.old_file().path(),
                added.new_file().path()
            );

            let old_path = deleted.old_file().path().expect("deletes have paths");
            let new_path = added.new_file().path_bytes().expect("added has path");

            let mut index = Index::new()?;
            index.read_tree(&parent_tree)?;
            let stage = 0;
            let mut taken = index
                .get_path(old_path, stage)
                .expect("old path must exist");
            index.remove(old_path, stage)?;
            taken.path = new_path.to_vec();
            index.add(&taken)?;
            let new_tree = index.write_tree_to(repo)?;
            println!("{:?}", new_tree);

            let old_message = commit.message().ok_or(anyhow!("invalid commit message"))?;
            let message = rename_regex.replace(old_message, "chore(rename): ");

            let commit_id = repo.commit(
                Some(&format!("refs/heads/{}", new_branch_name)),
                &commit.author(),
                &commit.committer(),
                &message,
                &repo.find_tree(new_tree)?,
                &[&parent],
            )?;

            parent = repo.find_commit(commit_id)?;
        }

        let commit_id = repo.commit(
            Some(&format!("refs/heads/{}", new_branch_name)),
            &commit.author(),
            &commit.committer(),
            &commit.message().ok_or(anyhow!("invalid commit message"))?,
            &commit.tree()?,
            &[&parent],
        )?;

        parent = repo.find_commit(commit_id)?;
    }
    Ok(())
}
