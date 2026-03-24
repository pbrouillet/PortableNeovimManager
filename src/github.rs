use bytes::Bytes;
use futures_util::StreamExt;
use serde::Deserialize;
use thiserror::Error;

const GITHUB_API_BASE: &str = "https://api.github.com/repos/neovim/neovim/releases";
const USER_AGENT: &str = "portable-neovim-manager";

#[derive(Debug, Error)]
pub enum GithubError {
    #[error(transparent)]
    Request(#[from] reqwest::Error),

    #[error("No suitable asset found for {0}")]
    NoAssetFound(String),

    #[error("No releases found")]
    NoReleases,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Release {
    pub tag_name: String,
    pub name: Option<String>,
    pub prerelease: bool,
    pub assets: Vec<Asset>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Asset {
    pub name: String,
    pub browser_download_url: String,
    pub size: u64,
}

pub async fn fetch_releases() -> Result<Vec<Release>, GithubError> {
    let client = reqwest::Client::new();
    let releases = client
        .get(GITHUB_API_BASE)
        .header(reqwest::header::USER_AGENT, USER_AGENT)
        .send()
        .await?
        .error_for_status()?
        .json::<Vec<Release>>()
        .await?;
    Ok(releases)
}

pub async fn fetch_latest_stable() -> Result<Release, GithubError> {
    let releases = fetch_releases().await?;
    releases
        .into_iter()
        .find(|r| !r.prerelease)
        .ok_or(GithubError::NoReleases)
}

pub async fn fetch_release_by_tag(tag: &str) -> Result<Release, GithubError> {
    let url = format!("{GITHUB_API_BASE}/tags/{tag}");
    let client = reqwest::Client::new();
    let release = client
        .get(&url)
        .header(reqwest::header::USER_AGENT, USER_AGENT)
        .send()
        .await?
        .error_for_status()?
        .json::<Release>()
        .await?;
    Ok(release)
}

pub fn select_asset(release: &Release) -> Result<&Asset, GithubError> {
    let os = std::env::consts::OS;
    let arch = std::env::consts::ARCH;

    // Neovim release assets use "arm64" rather than Rust's "aarch64".
    let arch_token = match arch {
        "aarch64" => "arm64",
        other => other,
    };

    let platform_desc = format!("{os}/{arch}");

    release
        .assets
        .iter()
        .find(|a| match os {
            "windows" => {
                let name = a.name.to_lowercase();
                name.contains("win64") && name.ends_with(".zip")
            }
            "linux" => {
                let name = a.name.to_lowercase();
                name.contains("linux") && name.contains(arch_token) && name.ends_with(".tar.gz")
            }
            "macos" => {
                let name = a.name.to_lowercase();
                name.contains("macos") && name.contains(arch_token) && name.ends_with(".tar.gz")
            }
            _ => false,
        })
        .ok_or(GithubError::NoAssetFound(platform_desc))
}

pub async fn download_asset(asset: &Asset) -> Result<Bytes, GithubError> {
    let client = reqwest::Client::new();
    let bytes = client
        .get(&asset.browser_download_url)
        .header(reqwest::header::USER_AGENT, USER_AGENT)
        .send()
        .await?
        .error_for_status()?
        .bytes()
        .await?;
    Ok(bytes)
}

pub async fn download_asset_with_progress<F>(
    asset: &Asset,
    on_progress: F,
) -> Result<Bytes, GithubError>
where
    F: Fn(u64, u64),
{
    let client = reqwest::Client::new();
    let response = client
        .get(&asset.browser_download_url)
        .header(reqwest::header::USER_AGENT, USER_AGENT)
        .send()
        .await?
        .error_for_status()?;

    let total = response.content_length().unwrap_or(asset.size);
    let mut downloaded: u64 = 0;
    let mut buf = Vec::with_capacity(total as usize);

    let mut stream = response.bytes_stream();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        downloaded += chunk.len() as u64;
        buf.extend_from_slice(&chunk);
        on_progress(downloaded, total);
    }

    Ok(Bytes::from(buf))
}
