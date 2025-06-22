use std::path::PathBuf;

use thunderstore::models::Package;
use url::Url;

use crate::downloader::RemoteLoaderError;


pub fn from_thunderstore(
    url: Url,
    cache_dir: &std::path::Path,
) -> Result<PathBuf, RemoteLoaderError> {
    let cache = directories::ProjectDirs::from("", "", "rsquirrel_lsp")
        .map(|dirs| dirs.cache_dir().to_path_buf());
    let cache = cache.ok_or(RemoteLoaderError::NoCacheDir)?;
    let repo_name = url.path();
    let repo_name = repo_name.strip_prefix("/c/northstar/p/").unwrap_or(repo_name);
    let repo_name = repo_name.replace("/", "-");
    let path = cache.join("thunderstore").join(repo_name);
    if path.exists() {
        //TODO: Update repo if needed
    } else {
        //
    }
    todo!()
}

#[cfg(test)]
mod tests{
    use url::Url;

    #[test]
    fn test_dirs() {
        let url = Url::parse("https://thunderstore.io/c/northstar/p/northstar/Northstar/").unwrap();
        println!("URL: {}", url);
        let stripped = url.path().strip_prefix("/").unwrap_or(url.path());
        println!("Stripped path: {}", stripped);

        let client = thunderstore::Client::new();
    }

    #[tokio::test]
    async fn test_ts_loader() {
        let client = thunderstore::Client::new();
        let url = Url::parse("https://thunderstore.io/c/northstar/p/northstar/Northstar/").unwrap();
        let repo_name = url.path();
        println!("URL: {}", url);
        let repo_name = repo_name.strip_prefix("/c/northstar/p/").and_then(|x| x.strip_suffix("/")).unwrap_or(repo_name);
        println!("Stripped repo name: {}", repo_name);
        let repo_name = repo_name.replace("/", "-");
        println!("Repo name: {}", repo_name);
        let package = client.get_package(repo_name).await;
        assert!(package.is_ok(), "Failed to get package: {:?}", package.err());
        println!("Package: {:?}", package.unwrap());
    }
}