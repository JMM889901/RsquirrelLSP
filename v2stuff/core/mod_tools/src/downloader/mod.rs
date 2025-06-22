
#[derive(Debug)]
pub enum RemoteLoaderError {
    InvalidUrl,
    GitError(git2::Error),
    NoCacheDir,
    OutOfDate,
}

impl From<git2::Error> for RemoteLoaderError {
    fn from(err: git2::Error) -> Self {
        RemoteLoaderError::GitError(err)
    }
}

mod git_loader;
mod thunderstore_loader;
pub use git_loader::from_github;
pub use thunderstore_loader::from_thunderstore;