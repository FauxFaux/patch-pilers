use anyhow::ensure;
use anyhow::Result;
use clap::Arg;
use clap::SubCommand;
use git2::Delta;
use git2::Index;
use git2::Oid;

fn main() -> Result<()> {
    let matches = clap::App::new("patch-pilers")
        .arg(
            Arg::with_name("dest")
                .long("dest")
                .takes_value(true)
                .required(true),
        )
        .setting(clap::AppSettings::SubcommandRequiredElseHelp)
        .subcommand(
            SubCommand::with_name("add-renames").arg(
                Arg::with_name("since")
                    .long("since")
                    .takes_value(true)
                    .required(true),
            ),
        )
        .get_matches();

    let repo = git2::Repository::open(matches.value_of("dest").expect("required"))?;

    match matches.subcommand() {
        ("add-renames", Some(child)) => {
            add_renames(&repo, child.value_of("since").expect("required"))
        }
        _ => unreachable!("subcommands are required"),
    }
}

fn add_renames(repo: &git2::Repository, since: &str) -> Result<()> {
    let mut walk = repo.revwalk()?;
    walk.push_head()?;
    walk.hide(repo.revparse_single(since)?.id())?;
    let mut commits = walk.collect::<Result<Vec<Oid>, git2::Error>>()?;
    commits.reverse();

    for id in commits {
        let commit = repo.find_commit(id)?;
        ensure!(1 == commit.parent_count(), "");
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
        }
    }
    Ok(())
}
