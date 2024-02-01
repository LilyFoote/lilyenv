use crate::directories::shell_file;
use crate::error::Error;

pub fn set_shell(shell: &str) -> Result<(), Error> {
    std::fs::write(shell_file(), shell)?;
    Ok(())
}

pub fn get_shell() -> Result<String, Error> {
    match std::fs::read_to_string(shell_file()) {
        Ok(shell) => Ok(shell),
        Err(err) => match err.kind() {
            std::io::ErrorKind::NotFound => Ok(std::env::var("SHELL")?),
            _ => Err(err)?,
        },
    }
}

pub fn print_shell_config() -> Result<(), Error> {
    match get_shell()?.as_str() {
        "bash" => println!(include_str!("bash_config")),
        "zsh" => println!(include_str!("zsh_config")),
        "fish" => println!(include_str!("fish_config")),
        _ => println!("Unknown shell"),
    }
    Ok(())
}
