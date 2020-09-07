use anyhow::Result;
use git2::oid_array::OidArray;
use git2::{BranchType, Error, ErrorCode, Oid};

pub fn delete_merged(repo: &git2::Repository) -> Result<()> {
    for branch in repo.branches(Some(BranchType::Local))? {
        let (branch, _) = branch?;

        let mut ids = Vec::with_capacity(3);
        let branch_id = branch.get().peel_to_commit()?.id();

        ids.push(branch_id);
        ids.push(repo.head()?.peel_to_commit()?.id());

        match branch.upstream() {
            Ok(b) => {
                ids.push(b.into_reference().peel_to_commit()?.id());
            }
            Err(e) if e.code() == ErrorCode::NotFound => (),
            Err(e) => Err(e)?,
        }

        let merge_bases = repo.merge_bases_many(&ids)?;

        let safe = any_descend(repo, branch_id, merge_bases)?;
        println!("{} {:?}", safe, branch.name()?)
    }
    Ok(())
}

fn any_descend(repo: &git2::Repository, branch_id: Oid, merge_bases: OidArray) -> Result<bool> {
    for merge_base in merge_bases.iter() {
        if repo.graph_descendant_of(branch_id, *merge_base)? {
            println!("{:?} extends {:?}", branch_id, merge_base);
            return Ok(true);
        }
    }

    Ok(false)
}
