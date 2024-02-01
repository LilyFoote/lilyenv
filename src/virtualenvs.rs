use crate::directories::{project_dir, project_file, python_dir, virtualenv_dir, virtualenvs_dir};
use crate::download::download_python;
use crate::error::Error;
use crate::shell::get_shell;
use crate::version::Version;

pub fn create_virtualenv(version: &Version, project: &str) -> Result<(), Error> {
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

pub fn remove_virtualenv(project: &str, version: &Version) -> Result<(), Error> {
    let virtualenv = virtualenv_dir(project, version);
    std::fs::remove_dir_all(virtualenv)?;
    Ok(())
}

pub fn remove_project(project: &str) -> Result<(), Error> {
    std::fs::remove_dir_all(project_dir(project))?;
    Ok(())
}

pub fn set_project_directory(project: &str, default_directory: &str) -> Result<(), Error> {
    std::fs::write(project_file(project), default_directory)?;
    Ok(())
}

pub fn unset_project_directory(project: &str) -> Result<(), Error> {
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

pub fn activate_virtualenv(version: &Version, project: &str) -> Result<(), Error> {
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

pub fn cd_site_packages(project: &str, version: &Version) -> Result<(), Error> {
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

pub fn print_project_versions(project: String) -> Result<(), Error> {
    let virtualenvs = project_dir(&project);
    let versions = list_versions(virtualenvs)?;
    println!("{}", versions.join(" "));
    Ok(())
}

pub fn print_all_versions() -> Result<(), Error> {
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
