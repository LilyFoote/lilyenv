use bzip2::read::BzDecoder;
use clap::{Parser, Subcommand};
use flate2::read::GzDecoder;
use std::fs::File;
use std::path::Path;
use tar::Archive;
use url::Url;

mod directories;
mod error;
mod releases;
mod types;
use crate::directories::{
    downloads_dir, project_dir, project_file, python_dir, shell_file, virtualenv_dir,
    virtualenvs_dir,
};
use crate::error::Error;
use crate::releases::{cpython_releases, pypy_releases};
use crate::types::{Interpreter, Version};

fn download_python(version: &Version, upgrade: bool) -> Result<(), Error> {
    match version.interpreter {
        Interpreter::CPython => download_cpython(version, upgrade),
        Interpreter::PyPy => download_pypy(version, upgrade),
    }
}

fn download_cpython(version: &Version, upgrade: bool) -> Result<(), Error> {
    let python_dir = python_dir(version);
    if !upgrade && python_dir.exists() {
        return Ok(());
    }

    let downloads = downloads_dir();
    std::fs::create_dir_all(&downloads)?;

    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    let python = match rt
        .block_on(cpython_releases())?
        .into_iter()
        .find(|python| python.version.compatible(version))
    {
        Some(python) => python,
        None => {
            return Err(Error::VersionNotFound(version.to_string()));
        }
    };
    let path = downloads.join(python.name);
    if upgrade || !path.exists() {
        download_file(python.url, &path)?;
    }
    extract_tar_gz(&path, &python_dir)?;
    Ok(())
}

fn download_pypy(version: &Version, upgrade: bool) -> Result<(), Error> {
    let python_dir = python_dir(version);
    if !upgrade && python_dir.exists() {
        return Ok(());
    }

    let downloads = downloads_dir();
    std::fs::create_dir_all(&downloads)?;

    let python = match pypy_releases()?
        .into_iter()
        .find(|python| python.version.compatible(version))
    {
        Some(python) => python,
        None => {
            return Err(Error::VersionNotFound(version.to_string()));
        }
    };
    let path = downloads.join(python.name);
    if upgrade || !path.exists() {
        download_file(python.url, &path)?;
    }
    extract_tar_bz2(&path, &python_dir)?;
    Ok(())
}

fn download_file(url: Url, target: &Path) -> Result<(), Error> {
    let client = reqwest::blocking::Client::builder()
        .user_agent("lilyenv")
        .build()?;
    let response = client.get(url).send()?;
    let mut file = File::create(target)?;
    let mut content = std::io::Cursor::new(response.bytes()?);
    std::io::copy(&mut content, &mut file)?;
    Ok(())
}

fn extract_tar_gz(source: &Path, target: &Path) -> Result<(), std::io::Error> {
    let tar_gz = File::open(source)?;
    let tar = GzDecoder::new(tar_gz);
    let mut archive = Archive::new(tar);
    archive.unpack(target)?;
    Ok(())
}

fn extract_tar_bz2(source: &Path, target: &Path) -> Result<(), std::io::Error> {
    let tar_gz = File::open(source)?;
    let tar = BzDecoder::new(tar_gz);
    let mut archive = Archive::new(tar);
    archive.unpack(target)?;
    Ok(())
}

fn create_virtualenv(version: &Version, project: &str) -> Result<(), Error> {
    let python = python_dir(version);
    if !python.exists() {
        download_python(version, false)?;
    }
    let next = std::fs::read_dir(&python)?
        .next()
        .unwrap_or_else(|| {
            panic!(
                "Expected subdirectory missing from downloaded python at {:?}.",
                &python
            )
        })?
        .path();
    let python_executable = next.join("bin/python3");
    let virtualenv = virtualenv_dir(project, version);
    std::process::Command::new(python_executable)
        .arg("-m")
        .arg("venv")
        .arg(virtualenv)
        .output()?;
    Ok(())
}

fn remove_virtualenv(project: &str, version: &Version) -> Result<(), Error> {
    let virtualenv = virtualenv_dir(project, version);
    std::fs::remove_dir_all(virtualenv)?;
    Ok(())
}

fn remove_project(project: &str) -> Result<(), Error> {
    std::fs::remove_dir_all(project_dir(project))?;
    Ok(())
}

fn activate_virtualenv(version: &Version, project: &str) -> Result<(), Error> {
    let virtualenv = virtualenv_dir(project, version);
    if !virtualenv.exists() {
        create_virtualenv(version, project)?
    }
    let path = std::env::var("PATH")?;
    let path = format!("{}:{path}", virtualenv.join("bin").display());

    let mut shell = std::process::Command::new(get_shell()?);
    let shell = match project_directory(project)? {
        Some(directory) => shell.current_dir(directory),
        _ => &mut shell,
    };
    let python = python_dir(version).join("python");
    let mut shell = shell
        .env("VIRTUAL_ENV", &virtualenv)
        .env("VIRTUAL_ENV_PROMPT", format!("{project} ({version}) "))
        .env("PATH", path)
        .env(
            "TERMINFO_DIRS",
            "/etc/terminfo:/lib/terminfo:/usr/share/terminfo",
        )
        .env("LD_LIBRARY_PATH", python.join("lib"))
        .spawn()?;
    shell.wait()?;
    Ok(())
}

fn cd_site_packages(project: &str, version: &Version) -> Result<(), Error> {
    let virtualenv = virtualenv_dir(project, version);
    let lib = virtualenv.join("lib");
    let next = std::fs::read_dir(&lib)?
        .next()
        .unwrap_or_else(|| {
            panic!(
                "Expected subdirectory missing from virtualenv at {:?}.",
                &lib
            )
        })?
        .path();
    let site_packages = next.join("site-packages");

    let mut shell = std::process::Command::new(get_shell()?)
        .current_dir(site_packages)
        .spawn()?;
    shell.wait()?;
    Ok(())
}

fn print_available_downloads() -> Result<(), Error> {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    let mut releases = rt.block_on(cpython_releases())?;
    releases.sort_unstable_by_key(|p| p.version);
    for python in releases {
        println!("{} ({})", python.version, python.release_tag);
    }
    let mut pypy_releases = pypy_releases()?;
    pypy_releases.sort_unstable_by_key(|p| p.version);
    for python in pypy_releases {
        println!("{} ({})", python.version, python.release_tag);
    }
    Ok(())
}

fn list_versions(path: std::path::PathBuf) -> Result<Vec<String>, Error> {
    Ok(std::fs::read_dir(path)?
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .map(|version| {
            version
                .file_name()
                .to_str()
                .expect("Could not convert a version to utf-8.")
                .to_string()
        })
        .collect::<Vec<_>>())
}

fn print_project_versions(project: String) -> Result<(), Error> {
    let virtualenvs = project_dir(&project);
    let versions = list_versions(virtualenvs)?;
    println!("{}", versions.join(" "));
    Ok(())
}

fn print_all_versions() -> Result<(), Error> {
    let projects = virtualenvs_dir();
    for project in std::fs::read_dir(projects)? {
        let project = project?;
        let versions = list_versions(project.path())?;
        println!(
            "{}: {}",
            project
                .file_name()
                .to_str()
                .expect("Could not convert a project directory name to utf-8"),
            versions.join(" ")
        );
    }
    Ok(())
}

fn set_project_directory(project: &str, default_directory: &str) -> Result<(), Error> {
    std::fs::write(project_file(project), default_directory)?;
    Ok(())
}

fn unset_project_directory(project: &str) -> Result<(), Error> {
    std::fs::remove_file(project_file(project))?;
    Ok(())
}

fn project_directory(project: &str) -> Result<Option<String>, Error> {
    match std::fs::read_to_string(project_file(project)) {
        Ok(default_directory) => Ok(Some(default_directory)),
        Err(err) => match err.kind() {
            std::io::ErrorKind::NotFound => Ok(None),
            _ => Err(err)?,
        },
    }
}

fn set_shell(shell: &str) -> Result<(), Error> {
    std::fs::write(shell_file(), shell)?;
    Ok(())
}

fn get_shell() -> Result<String, Error> {
    match std::fs::read_to_string(shell_file()) {
        Ok(shell) => Ok(shell),
        Err(err) => match err.kind() {
            std::io::ErrorKind::NotFound => Ok(std::env::var("SHELL")?),
            _ => Err(err)?,
        },
    }
}

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
        Commands::ShellConfig => match get_shell()?.as_str() {
            "bash" => println!(include_str!("bash_config")),
            "zsh" => println!(include_str!("zsh_config")),
            "fish" => println!(include_str!("fish_config")),
            _ => println!("Unknown shell"),
        },
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
