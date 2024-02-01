use crate::types::Version;

fn lilyenv_dir() -> directories::ProjectDirs {
    directories::ProjectDirs::from("", "", "Lilyenv").expect("Could not find the home directory")
}

pub fn downloads_dir() -> std::path::PathBuf {
    lilyenv_dir().cache_dir().join("downloads")
}

pub fn python_dir(version: &Version) -> std::path::PathBuf {
    lilyenv_dir()
        .data_local_dir()
        .join("pythons")
        .join(version.to_string())
}

pub fn virtualenvs_dir() -> std::path::PathBuf {
    lilyenv_dir().data_local_dir().join("virtualenvs")
}

pub fn shell_file() -> std::path::PathBuf {
    lilyenv_dir().data_local_dir().join("shell")
}

pub fn project_dir(project: &str) -> std::path::PathBuf {
    virtualenvs_dir().join(project)
}

pub fn virtualenv_dir(project: &str, version: &Version) -> std::path::PathBuf {
    project_dir(project).join(version.to_string())
}

pub fn project_file(project: &str) -> std::path::PathBuf {
    project_dir(project).join("directory")
}
