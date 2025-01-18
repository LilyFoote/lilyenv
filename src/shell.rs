use crate::directories::shell_file;
use crate::error::Error;

pub fn set_shell(shell: &str, project: Option<&str>) -> Result<(), Error> {
    let file = shell_file(project);
    match std::fs::write(&file, shell) {
        Ok(_) => {}
        Err(err) => match err.kind() {
            std::io::ErrorKind::NotFound => {
                let dir = file.parent().expect("shell file always has a parent");
                std::fs::create_dir_all(dir)?;
                std::fs::write(&file, shell)?;
            }
            _ => Err(err)?,
        },
    }
    Ok(())
}

pub fn get_shell(project: Option<&str>) -> Result<String, Error> {
    match std::fs::read_to_string(shell_file(project)) {
        Ok(shell) => Ok(shell),
        Err(err) => match err.kind() {
            std::io::ErrorKind::NotFound => match project {
                Some(_project) => get_shell(None),
                None => Ok(std::env::var("SHELL")?),
            },
            _ => Err(err)?,
        },
    }
}

pub fn print_shell_config(project: Option<&str>) -> Result<(), Error> {
    match get_shell(project)?.as_str() {
        "bash" => println!(include_str!("bash_config")),
        "zsh" => println!(include_str!("zsh_config")),
        "fish" => println!(include_str!("fish_config")),
        _ => println!("Unknown shell"),
    }
    Ok(())
}
