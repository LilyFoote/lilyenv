use bzip2::read::BzDecoder;
use clap::{Parser, Subcommand};
use current_platform::CURRENT_PLATFORM;
use flate2::read::GzDecoder;
use std::fs::File;
use std::path::Path;
use tar::Archive;
use url::Url;

const PYPY_DOWNLOAD_URL: &str = "https://downloads.python.org/pypy/";

#[derive(Debug)]
struct Python {
    name: String,
    url: Url,
    version: Version,
    release_tag: String,
}

#[derive(Debug)]
enum Error {
    Request(reqwest::Error),
    Octocrab(octocrab::Error),
    Fs(std::io::Error),
    VersionNotFound(String),
    InvalidVersion(String),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Request(err) => write!(f, "{err}"),
            Self::Octocrab(err) => write!(f, "{err}"),
            Self::Fs(err) => write!(f, "{err}"),
            Self::VersionNotFound(version) => write!(f, "Could not find {version} to download."),
            Self::InvalidVersion(version) => write!(f, "{version} is not a valid Python version"),
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

impl From<octocrab::Error> for Error {
    fn from(err: octocrab::Error) -> Self {
        Self::Octocrab(err)
    }
}

fn parse_version(filename: &str) -> nom::IResult<&str, (String, Version)> {
    use nom::bytes::complete::tag;
    use nom::character::complete::u8;
    let (input, _) = tag("cpython-")(filename)?;
    let (input, (major, _, minor, _, bugfix, _, release_tag)) = nom::sequence::tuple((
        u8,
        tag("."),
        u8,
        tag("."),
        u8,
        tag("+"),
        nom::character::complete::digit1,
    ))(input)?;

    let version = Version {
        interpreter: Interpreter::CPython,
        major,
        minor,
        bugfix: Some(bugfix),
    };
    Ok((input, (release_tag.to_string(), version)))
}

fn parse_pypy_version(url: &str) -> nom::IResult<&str, (String, String, Version)> {
    use nom::bytes::complete::{tag, take_until};
    use nom::character::complete::u8;
    let (filename, _) = tag(PYPY_DOWNLOAD_URL)(url)?;
    let (rest, (_, major, _, minor, _, release_tag)) =
        nom::sequence::tuple((tag("pypy"), u8, tag("."), u8, tag("-"), take_until("-")))(filename)?;

    let version = Version {
        interpreter: Interpreter::PyPy,
        major,
        minor,
        bugfix: None,
    };

    Ok((
        rest,
        (filename.to_string(), release_tag.to_string(), version),
    ))
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
enum Interpreter {
    CPython,
    PyPy,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
struct Version {
    interpreter: Interpreter,
    major: u8,
    minor: u8,
    bugfix: Option<u8>,
}

impl Version {
    fn compatible(&self, other: &Self) -> bool {
        if self == other {
            true
        } else {
            self.interpreter == other.interpreter
                && self.major == other.major
                && self.minor == other.minor
                && other.bugfix.is_none()
        }
    }
}

impl std::fmt::Display for Version {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let prefix = match self.interpreter {
            Interpreter::CPython => "",
            Interpreter::PyPy => "pypy",
        };
        match self.bugfix {
            Some(bugfix) => write!(f, "{}{}.{}.{}", prefix, self.major, self.minor, bugfix),
            None => write!(f, "{}{}.{}", prefix, self.major, self.minor),
        }
    }
}

fn _validate_version(version: &str) -> nom::IResult<&str, Version> {
    use nom::bytes::complete::tag;
    use nom::character::complete::u8;
    use nom::sequence::separated_pair;
    let (rest, interpreter) = nom::combinator::opt(tag("pypy"))(version)?;
    let (rest, (major, minor)) = separated_pair(u8, tag("."), u8)(rest)?;
    let (rest, bugfix) = nom::combinator::opt(nom::sequence::preceded(tag("."), u8))(rest)?;
    nom::combinator::eof(rest)?;
    let interpreter = match interpreter {
        Some(_) => Interpreter::PyPy,
        None => Interpreter::CPython,
    };
    Ok((
        rest,
        Version {
            interpreter,
            major,
            minor,
            bugfix,
        },
    ))
}

fn validate_version(version: &str) -> Result<Version, Error> {
    match _validate_version(version) {
        Ok((_, version)) => Ok(version),
        Err(_) => Err(Error::InvalidVersion(version.into())),
    }
}

async fn releases() -> Result<Vec<Python>, Error> {
    let octocrab = octocrab::instance();
    Ok(octocrab
        .repos("indygreg", "python-build-standalone")
        .releases()
        .list()
        .send()
        .await?
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
        .filter(|asset| !asset.name.ends_with(".sha256"))
        .filter(|asset| asset.name.contains(CURRENT_PLATFORM))
        .filter(|asset| asset.name.contains("install_only"))
        .map(|asset| {
            let (_, (release_tag, version)) = parse_version(&asset.name).unwrap();
            Python {
                name: asset.name,
                url: asset.browser_download_url,
                version,
                release_tag,
            }
        })
        .collect())
}

fn pypy_releases() -> Vec<Python> {
    let html = reqwest::blocking::get("https://www.pypy.org/download.html")
        .unwrap()
        .text()
        .unwrap();
    let document = scraper::Html::parse_document(&html);
    let selector = scraper::Selector::parse("table>tbody>tr>td>p>a").unwrap();
    document
        .select(&selector)
        .map(|link| link.value().attr("href").unwrap())
        .filter(|link| link.starts_with(PYPY_DOWNLOAD_URL))
        .filter(|link| link.contains("linux64"))
        .map(|url| {
            let (_, (name, release_tag, version)) = parse_pypy_version(url).unwrap();
            Python {
                name,
                url: Url::parse(url).unwrap(),
                version,
                release_tag,
            }
        })
        .collect()
}

fn download_python(version: &Version, upgrade: bool) -> Result<(), Error> {
    match version.interpreter {
        Interpreter::CPython => download_cpython(version, upgrade),
        Interpreter::PyPy => download_pypy(version, upgrade),
    }
}

fn lilyenv_dir() -> directories::ProjectDirs {
    directories::ProjectDirs::from("", "", "Lilyenv").expect("Could not find the home directory")
}

fn downloads_dir() -> std::path::PathBuf {
    lilyenv_dir().cache_dir().join("downloads")
}

fn pythons_dir() -> std::path::PathBuf {
    lilyenv_dir().data_local_dir().join("pythons")
}

fn virtualenvs_dir() -> std::path::PathBuf {
    lilyenv_dir().data_local_dir().join("virtualenvs")
}

fn download_cpython(version: &Version, upgrade: bool) -> Result<(), Error> {
    let python_dir = pythons_dir().join(version.to_string());
    if !upgrade && python_dir.exists() {
        return Ok(());
    }

    let downloads = downloads_dir();
    std::fs::create_dir_all(&downloads)?;

    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    let python = match rt
        .block_on(releases())?
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
    let python_dir = pythons_dir().join(version.to_string());
    if !upgrade && python_dir.exists() {
        return Ok(());
    }

    let downloads = downloads_dir();
    std::fs::create_dir_all(&downloads)?;

    let python = match pypy_releases()
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
    let python = pythons_dir().join(version.to_string());
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
    let virtualenv = virtualenvs_dir().join(project).join(version.to_string());
    std::process::Command::new(python_executable)
        .arg("-m")
        .arg("venv")
        .arg(virtualenv)
        .output()?;
    Ok(())
}

fn activate_virtualenv(version: &Version, project: &str) -> Result<(), Error> {
    let virtualenv = virtualenvs_dir().join(project).join(version.to_string());
    if !virtualenv.exists() {
        create_virtualenv(version, project)?
    }
    let path = std::env::var("PATH").expect("Could not read PATH environment variable.");
    let path = format!("{}:{path}", virtualenv.join("bin").display());

    let mut bash = std::process::Command::new("bash")
        .env("VIRTUAL_ENV", &virtualenv)
        .env("VIRTUAL_ENV_PROMPT", format!("{project} ({version}) "))
        .env("PATH", path)
        .env(
            "TERMINFO_DIRS",
            "/etc/terminfo:/lib/terminfo:/usr/share/terminfo",
        )
        .spawn()?;
    bash.wait()?;
    Ok(())
}

fn print_available_downloads() -> Result<(), Error> {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    let mut releases = rt.block_on(releases())?;
    releases.sort_unstable_by_key(|p| p.version);
    for python in releases {
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
    let projects = virtualenvs_dir();
    let virtualenvs = projects.join(project);
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
    /// List all available virtualenvs, or those for the given Project
    List { project: Option<String> },
    /// Upgrade a Python version to the latest bugfix release
    Upgrade { version: String },
    /// Create a virtualenv given a Python version and a Project string
    Virtualenv { version: String, project: String },
    /// Download a specific Python version or list all Python versions available to download
    Download { version: Option<String> },
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
            let version = validate_version(&version)?;
            download_python(&version, false)?;
        }
        Commands::Virtualenv { version, project } => {
            let version = validate_version(&version)?;
            create_virtualenv(&version, &project)?;
        }
        Commands::Activate { version, project } => {
            let version = validate_version(&version)?;
            activate_virtualenv(&version, &project)?;
        }
        Commands::ShellConfig => {
            println!(include_str!("bash_config"));
        }
        Commands::List { project } => match project {
            Some(project) => print_project_versions(project)?,
            None => print_all_versions()?,
        },
        Commands::Upgrade { version } => {
            let version = validate_version(&version)?;
            match version.bugfix {
                Some(_) => eprintln!("Only x.y Python versions can be upgraded, not x.y.z"),
                None => download_python(&version, true)?,
            }
        }
    }
    Ok(())
}

fn main() {
    if let Err(e) = run() {
        eprintln!("{e}");
    }
}
