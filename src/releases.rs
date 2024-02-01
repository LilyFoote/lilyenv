use crate::error::Error;
use crate::version::{parse_cpython_filename, parse_pypy_url, Version, PYPY_DOWNLOAD_URL};
use current_platform::CURRENT_PLATFORM;
use url::Url;

#[derive(Debug)]
pub struct Python {
    pub name: String,
    pub url: Url,
    pub version: Version,
    pub release_tag: String,
}

pub async fn cpython_releases() -> Result<Vec<Python>, Error> {
    let octocrab = octocrab::instance();
    octocrab
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
                        .expect("Could not parse hardcoded datetime.")
                        .into(),
                )
        })
        .flat_map(|release| release.assets)
        .filter(|asset| !asset.name.ends_with(".sha256"))
        .filter(|asset| asset.name.contains(CURRENT_PLATFORM))
        .filter(|asset| asset.name.contains("install_only"))
        .map(|asset| {
            let (release_tag, version) = parse_cpython_filename(&asset.name)?;
            Ok(Python {
                name: asset.name,
                url: asset.browser_download_url,
                version,
                release_tag,
            })
        })
        .collect()
}

fn pypy_platform_tag() -> Result<&'static str, Error> {
    match CURRENT_PLATFORM {
        "x86_64-unknown-linux-gnu" => Ok("linux64"),
        "x86_64-apple-darwin" => Ok("macos_x86_64"),
        "aarch64-unknown-linux-gnu" => Ok("aarch64"),
        "aarch64-apple-darwin" => Ok("macos_arm64"),
        _ => Err(Error::Platform(CURRENT_PLATFORM.to_string())),
    }
}

pub fn pypy_releases() -> Result<Vec<Python>, Error> {
    let html = reqwest::blocking::get("https://www.pypy.org/download.html")?.text()?;
    let document = scraper::Html::parse_document(&html);
    let selector = match scraper::Selector::parse("table>tbody>tr>td>p>a") {
        Ok(selector) => selector,
        Err(_) => Err(Error::Scraper(
            "Could not find table of pypy downloads.".to_string(),
        ))?,
    };
    let tag = pypy_platform_tag()?;
    document
        .select(&selector)
        .map(|link| {
            link.value()
                .attr("href")
                .expect("A pypy download <a> tag has a href attribute.")
        })
        .filter(|link| link.starts_with(PYPY_DOWNLOAD_URL))
        .filter(|link| link.contains(tag))
        .map(|url| {
            let (name, release_tag, version) = parse_pypy_url(url)?;
            Ok(Python {
                name,
                url: Url::parse(url)?,
                version,
                release_tag,
            })
        })
        .collect()
}
