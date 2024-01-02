use clap::{Parser, Subcommand};
use flate2::read::GzDecoder;
use std::fs::File;
use std::path::Path;
use tar::Archive;
use url::Url;

#[derive(Debug)]
struct Python {
    name: String,
    url: Url,
}

#[derive(Debug)]
enum Error {
    Request(reqwest::Error),
    Fs(std::io::Error),
    VersionNotFound(String),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Request(err) => write!(f, "{}", err),
            Self::Fs(err) => write!(f, "{}", err),
            Self::VersionNotFound(version) => write!(f, "Could not find {} to download.", version),
        }
    }
}

impl std::error::Error for Error {}

impl From<reqwest::Error> for Error {
    fn from(err: reqwest::Error) -> Self {
        Self::Request(err)
    }
}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Self {
        Self::Fs(err)
    }
}

fn parse_version(python: &Python) -> nom::IResult<&str, (String, u8, u8, u8)> {
    use nom::bytes::complete::tag;
    use nom::character::complete::u8;
    let (input, _) = tag("cpython-")(python.name.as_str())?;
    let (input, (major, _, minor, _, bugfix, _, release_tag)) = nom::sequence::tuple((
        u8,
        tag("."),
        u8,
        tag("."),
        u8,
        tag("+"),
        nom::character::complete::digit1,
    ))(input)?;

    Ok((input, (release_tag.to_string(), major, minor, bugfix)))
}

async fn releases(target: &str) -> Vec<Python> {
    let octocrab = octocrab::instance();
    octocrab
        .repos("indygreg", "python-build-standalone")
        .releases()
        .list()
        .send()
        .await
        .unwrap()
        .items
        .into_iter()
        .filter(|release| {
            release.created_at
                > Some(
                    chrono::DateTime::parse_from_rfc3339("2022-02-26T00:00:00Z")
                        .unwrap()
                        .into(),
                )
        })
        .flat_map(|release| release.assets)
        .map(|asset| Python {
            name: asset.name,
            url: asset.browser_download_url,
        })
        .filter(|python| python.name.contains(target))
        .filter(|python| python.name.contains("install_only"))
        .filter(|python| !python.name.ends_with(".sha256"))
        .collect()
}

fn download_python(version: &str) -> Result<(), Error> {
    let lilyenv = directories::ProjectDirs::from("", "", "Lilyenv").unwrap();
    let python_dir = lilyenv.data_local_dir().join("pythons").join(version);
    if python_dir.exists() {
        return Ok(());
    }

    let downloads = lilyenv.cache_dir().join("downloads");
    std::fs::create_dir_all(&downloads)?;

    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let python = match rt
        .block_on(releases("x86_64-unknown-linux-gnu"))
        .into_iter()
        .find(|python| python.name.contains(version))
    {
        Some(python) => python,
        None => {
            return Err(Error::VersionNotFound(version.to_string()));
        }
    };
    let path = downloads.join(python.name);
    if !path.exists() {
        download_file(python.url, &path)?;
    }
    extract_tar_gz(&path, &python_dir)?;
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

fn create_virtualenv(version: &str, project: &str) -> Result<(), Error> {
    let lilyenv = directories::ProjectDirs::from("", "", "Lilyenv").unwrap();
    let python = lilyenv.data_local_dir().join("pythons").join(version);
    if !python.exists() {
        download_python(version)?;
    }
    let python_executable = python.join("python/bin/python3");
    let virtualenv = lilyenv
        .data_local_dir()
        .join("virtualenvs")
        .join(project)
        .join(version);
    std::process::Command::new(python_executable)
        .arg("-m")
        .arg("venv")
        .arg(virtualenv)
        .output()?;
    Ok(())
}

fn activate_virtualenv(version: &str, project: &str) -> Result<(), Error> {
    let lilyenv = directories::ProjectDirs::from("", "", "Lilyenv").unwrap();
    let virtualenv = lilyenv
        .data_local_dir()
        .join("virtualenvs")
        .join(project)
        .join(version);
    if !virtualenv.exists() {
        create_virtualenv(version, project)?
    }
    let path = std::env::var("PATH").unwrap();
    let path = format!("{}:{}", virtualenv.join("bin").display(), path);

    let mut bash = std::process::Command::new("bash")
        .env("VIRTUAL_ENV", &virtualenv)
        .env("VIRTUAL_ENV_PROMPT", format!("{} ({}) ", project, version))
        .env("PATH", path)
        .env(
            "TERMINFO_DIRS",
            "/etc/terminfo:/lib/terminfo:/usr/share/terminfo",
        )
        .spawn()?;
    bash.wait()?;
    Ok(())
}

#[derive(Parser)]
#[command(author, version, about, long_about=None)]
struct Cli {
    #[command(subcommand)]
    cmd: Commands,
}

#[derive(Subcommand, Debug, Clone)]
enum Commands {
    /// Activate a virtualenv given a Python version and a Project string
    Activate { version: String, project: String },
    /// Create a virtualenv given a Python version and a Project string
    Virtualenv { version: String, project: String },
    /// Download a specific Python version or list all Python versions available to download
    Download { version: Option<String> },
}

fn main() {
    let cli = Cli::parse();

    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    match cli.cmd {
        Commands::Download { version: None } => {
            let mut releases = rt.block_on(releases("x86_64-unknown-linux-gnu"));
            releases.sort_unstable_by_key(|p| parse_version(p).unwrap().1);
            for python in releases {
                let (_, (release_tag, major, minor, bugfix)) = parse_version(&python).unwrap();
                println!("{major}.{minor}.{bugfix} ({release_tag})");
            }
        }
        Commands::Download {
            version: Some(version),
        } => download_python(&version).unwrap_or_else(|err| println!("{}", err)),
        Commands::Virtualenv { version, project } => {
            create_virtualenv(&version, &project).unwrap_or_else(|err| println!("{}", err))
        }
        Commands::Activate { version, project } => {
            activate_virtualenv(&version, &project).unwrap_or_else(|err| println!("{}", err))
        }
    }
}
