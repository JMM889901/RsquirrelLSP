use std::path::PathBuf;

use git2::Repository;
use url::Url;

use crate::downloader::RemoteLoaderError;

/// Pull a repo from github if not already here
/// 
/// It is HEAVILY reccomended to use thunderstore loader instead, since this pulls from main and not the latest release.
pub fn from_github(url: Url) -> Result<PathBuf, RemoteLoaderError> {
    let cache = directories::ProjectDirs::from("", "", "rsquirrel_lsp")
        .map(|dirs| dirs.cache_dir().to_path_buf());
    let cache = cache.ok_or(RemoteLoaderError::NoCacheDir)?;
    let repo_name = url.path();
    let repo_name = repo_name.strip_prefix("/").unwrap_or(repo_name);
    let repo_path = cache.join("git").join(repo_name);
    if repo_path.exists() {
        let repo = Repository::open(&repo_path)?;
        //TODO: Do i want to lock this or update to latest? i think updating to latest is stupid but if you don't want that, thunderstore is right there
        let mut remote = repo.find_remote("origin")?;
        remote.fetch(&["main"], None, None)?;

    } else {
        Repository::clone(url.as_str(), &repo_path)?;
    }
    Ok(repo_path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use url::Url;

    #[test]
    fn test_from_github() {
        let url = Url::parse("https://github.com/R2Northstar/NorthstarMods").unwrap();
        let result = from_github(url.clone());
        assert!(result.is_ok(), "Failed to clone repository: {:?}", result.err());
        let path = result.unwrap();
        println!("Repository cloned to: {:?}", path);
        let second_result = from_github(url);
        assert!(second_result.is_ok(), "Failed to get repo: {:?}", second_result.err());
    }
}