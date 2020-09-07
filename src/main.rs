use anyhow::Result;
use clap::Arg;
use clap::SubCommand;

mod add_renames;
mod delete_merged;
mod multi_status;

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
        .subcommand(SubCommand::with_name("delete-merged"))
        .subcommand(SubCommand::with_name("multi-status"))
        .get_matches();

    let repo = git2::Repository::open(matches.value_of("dest").expect("required"))?;

    match matches.subcommand() {
        ("add-renames", Some(child)) => {
            add_renames::add_renames(&repo, child.value_of("since").expect("required"))
        }
        ("delete-merged", Some(child)) => delete_merged::delete_merged(&repo),
        ("multi-status", Some(child)) => multi_status::multi_status(&repo),
        _ => unreachable!("subcommands are required"),
    }
}
