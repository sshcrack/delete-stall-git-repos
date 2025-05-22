use std::path::PathBuf;

use clap::Parser;
use colored::Colorize;
use git2::Repository;
use inquire::{MultiSelect, Select};
use tokio::fs;

/// Simple program to greet a person
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// The directory to scan
    #[arg(short, long, default_value = ".")]
    directory: PathBuf,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let Args { directory } = Args::parse();

    println!("{} {}", "Scanning directory".yellow(), directory.display().to_string().yellow());
    let mut dirs = fs::read_dir(&directory).await?;

    let mut repositories = Vec::new();
    while let Some(dir) = dirs.next_entry().await? {
        let path = dir.path();
        if !path.is_dir() {
            continue;
        }

        let path = path.canonicalize()?;
        let path_d = path.display().to_string();
        let repo = Repository::open(&path);
        if repo.is_err() {
            println!(
                "{}{}",
                path_d.bright_black(),
                " is not a git repository".bright_black()
            );
            continue;
        }

        let repo = repo.unwrap();
        let status = repo.statuses(None);
        if status.is_err() {
            println!("{}{}", "Couldn't get status for".red(), path_d.red());
            continue;
        }

        let status = status.unwrap();
        if !status.is_empty() {
            continue;
        }

        // Get all local branches
        let mut revwalk = repo.revwalk()?;
        for branch in repo.branches(Some(git2::BranchType::Local))? {
            let (branch, _) = branch?;
            let target = branch.get().target();
            if let Some(oid) = target {
                revwalk.push(oid)?;
            }
        }

        // Now exclude all remote branches
        for branch in repo.branches(Some(git2::BranchType::Remote))? {
            let (branch, _) = branch?;
            let target = branch.get().target();
            if let Some(oid) = target {
                revwalk.hide(oid)?;
            }
        }

        let mut is_unclean = false;
        // Iterate over unpushed commits
        for oid_result in revwalk {
            let oid = oid_result?;
            repo.find_commit(oid)?;

            is_unclean = true;
            break;
        }

        if is_unclean {
            println!("{}{}", "Unpushed commits in".red(), path_d.red());
            continue;
        }

        repositories.push(path);
        println!("{}{}", "Clean repository found:".green(), path_d.green());
    }

    println!("{}", "Found the following clean repositories:".green());
    for ele in &repositories {
        println!("{}", ele.display().to_string().green());
    }

    let options: Vec<&str> = vec![
        "Delete all repositories",
        "Select repositories to delete",
        "Cancel",
    ];

    let ans = Select::new("What do you want to do?", options).prompt()?;
    if ans == "Cancel" {
        println!("{}{}", "Cancelled".red(), "Exiting".red());
        return Ok(());
    }

    let repositories = repositories
        .into_iter()
        .map(|ele| {
            ele.to_str()
                .expect("Invalid UTF-8 in file path")
                .to_string()
        })
        .collect::<Vec<String>>();

    let to_delete = if ans == "Delete all repositories" {
        repositories
    } else {
        MultiSelect::new(
            "Select the repositories that should be deleted",
            repositories,
        )
        .prompt()?
    };

    if to_delete.is_empty() {
        println!("{}{}", "Cancelled".red(), "Exiting".red());
        return Ok(());
    }

    println!("{} {} {}", "Deleting a total of".red(), to_delete.len().to_string().red(), "repositories".red());
    for ele in &to_delete {
        println!("{} {}", "Deleting".red(), ele.red());
        let path = PathBuf::from(ele);
        if path.exists() {
            //fs::remove_dir_all(path).await?;
        }
    }
    Ok(())
}
