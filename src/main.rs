use clap::{Parser, Subcommand};

mod directories;
mod download;
mod error;
mod releases;
mod shell;
mod version;
mod virtualenvs;
use crate::download::{download_python, print_available_downloads};
use crate::error::Error;
use crate::shell::{print_shell_config, set_shell};
use crate::version::Version;
use crate::virtualenvs::{
    activate_virtualenv, cd_site_packages, create_virtualenv, print_all_versions,
    print_project_versions, remove_project, remove_virtualenv, set_project_directory,
    unset_project_directory,
};

#[derive(Parser)]
#[command(author, version, about, long_about=None)]
struct Cli {
    #[command(subcommand)]
    cmd: Commands,
}

#[derive(Subcommand, Debug, Clone)]
enum Commands {
    /// Activate a virtualenv given a Project string and a Python version
    Activate { project: String, version: Version },
    /// List all available virtualenvs, or those for the given Project
    List { project: Option<String> },
    /// Upgrade a Python version to the latest bugfix release
    Upgrade { version: Version },
    /// Open a subshell in a virtualenv's site packages
    SitePackages { project: String, version: Version },
    /// Set the default directory for a project
    SetProjectDirectory {
        project: String,
        default_directory: Option<String>,
    },
    /// Unset the default directory for a project
    UnsetProjectDirectory { project: String },
    /// Create a virtualenv given a Project string and a Python version
    Virtualenv { project: String, version: Version },
    /// Remove a virtualenv
    RemoveVirtualenv { project: String, version: Version },
    /// Remove all virtualenvs for a project
    RemoveProject { project: String },
    /// Download a specific Python version or list all Python versions available to download
    Download { version: Option<Version> },
    /// Explicitly set the shell for lilyenv to use
    SetShell { shell: String },
    /// Show information to include in a shell config file
    ShellConfig,
}

fn run() -> Result<(), Error> {
    let cli = Cli::parse();

    match cli.cmd {
        Commands::Download { version: None } => print_available_downloads()?,
        Commands::Download {
            version: Some(version),
        } => {
            download_python(&version, false)?;
        }
        Commands::Virtualenv { version, project } => {
            create_virtualenv(&version, &project)?;
        }
        Commands::RemoveVirtualenv { project, version } => {
            remove_virtualenv(&project, &version)?;
        }
        Commands::RemoveProject { project } => {
            remove_project(&project)?;
        }
        Commands::Activate { version, project } => {
            activate_virtualenv(&version, &project)?;
        }
        Commands::SetShell { shell } => set_shell(&shell)?,
        Commands::ShellConfig => print_shell_config()?,
        Commands::List { project } => match project {
            Some(project) => print_project_versions(project)?,
            None => print_all_versions()?,
        },
        Commands::Upgrade { version } => match version.bugfix {
            Some(_) => eprintln!("Only x.y Python versions can be upgraded, not x.y.z"),
            None => download_python(&version, true)?,
        },
        Commands::SetProjectDirectory {
            project,
            default_directory,
        } => {
            let default_directory = match default_directory {
                Some(default_directory) => default_directory,
                None => std::env::current_dir()?
                    .to_str()
                    .expect("The current directory should be valid unicode.")
                    .to_string(),
            };
            set_project_directory(&project, &default_directory)?;
        }
        Commands::UnsetProjectDirectory { project } => unset_project_directory(&project)?,
        Commands::SitePackages { project, version } => {
            cd_site_packages(&project, &version)?;
        }
    }
    Ok(())
}

fn main() {
    if let Err(e) = run() {
        eprintln!("{e}");
    }
}
