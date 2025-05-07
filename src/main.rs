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
    activate_virtualenv, cd_site_packages, create_virtualenv, get_version, print_all_versions,
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
    Activate {
        project: String,
        version: Option<Version>,
        #[arg(long)]
        no_cd: bool,
        #[arg(short, long, default_value=None, default_missing_value=".", num_args=0..=1)]
        directory: Option<String>,
    },
    /// List all available virtualenvs, or those for the given Project
    List { project: Option<String> },
    /// Upgrade a Python version to the latest bugfix release
    Upgrade { version: Version },
    /// Open a subshell in a virtualenv's site packages
    SitePackages { project: String, version: Version },
    /// Set the default directory for a project
    SetProjectDirectory {
        project: String,
        #[arg(default_value = ".")]
        default_directory: String,
    },
    /// Unset the default directory for a project
    UnsetProjectDirectory { project: String },
    /// Create a virtualenv given a Project string and a Python version
    Virtualenv {
        project: String,
        version: Version,
        #[arg(short, long, default_value=None, default_missing_value=".", num_args=0..=1)]
        directory: Option<String>,
    },
    /// Remove a virtualenv
    RemoveVirtualenv { project: String, version: Version },
    /// Remove all virtualenvs for a project
    RemoveProject { project: String },
    /// Download a specific Python version or list all Python versions available to download
    Download { version: Option<Version> },
    /// Explicitly set the shell for lilyenv to use
    SetShell {
        shell: String,
        project: Option<String>,
    },
    /// Show information to include in a shell config file
    ShellConfig { project: Option<String> },
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
        Commands::Virtualenv {
            version,
            project,
            directory,
        } => {
            create_virtualenv(&version, &project, directory)?;
        }
        Commands::RemoveVirtualenv { project, version } => {
            remove_virtualenv(&project, &version)?;
        }
        Commands::RemoveProject { project } => {
            remove_project(&project)?;
        }
        Commands::Activate {
            version,
            project,
            no_cd,
            directory,
        } => {
            let version = match version {
                Some(version) => version,
                None => get_version(&project)?,
            };
            activate_virtualenv(&version, &project, no_cd, directory)?;
        }
        Commands::SetShell { shell, project } => set_shell(&shell, project.as_deref())?,
        Commands::ShellConfig { project } => print_shell_config(project.as_deref())?,
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
