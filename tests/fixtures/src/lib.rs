use std::io;
use std::path::{Path, PathBuf};

/// The hackathon reference package: a Fleet holding `workflows/hackathon` and the
/// skills and agents its graph names.
pub fn hackathon_dir() -> PathBuf {
    todo!("red")
}

/// The small Fleet: two skills and one agent.
pub fn fleet_dir() -> PathBuf {
    todo!("red")
}

/// The mixed-language test repository's files (not yet a Git repository).
pub fn mixed_repo_dir() -> PathBuf {
    todo!("red")
}

/// Copies a directory tree, files and executable bits, into `dest`.
pub fn copy_tree(src: &Path, dest: &Path) -> io::Result<()> {
    let _ = (src, dest);
    todo!("red")
}

/// Writes the mixed-language repository into `dest` as a Git repository on `main`
/// with one commit and a clean tree.
pub fn materialize_repo(dest: &Path) -> io::Result<()> {
    let _ = dest;
    todo!("red")
}

/// A bare Git repository on disk, standing in for a remote for publish, install and
/// pull-request flows.
#[derive(Debug, Clone)]
pub struct GitServer {
    path: PathBuf,
}

impl GitServer {
    /// Creates `<root>/server.git` as a bare repository with `main` as its default
    /// branch; an existing one is left as it is.
    pub fn create(root: &Path) -> io::Result<GitServer> {
        let _ = root;
        todo!("red")
    }

    pub fn path(&self) -> &Path {
        let _ = &self.path;
        todo!("red")
    }

    /// The URL to clone from or push to.
    pub fn url(&self) -> String {
        todo!("red")
    }

    /// Commits the contents of `dir` to `main` on the server.
    pub fn seed(&self, dir: &Path) -> io::Result<()> {
        let _ = dir;
        todo!("red")
    }

    /// Branch names on the server, sorted.
    pub fn branches(&self) -> io::Result<Vec<String>> {
        todo!("red")
    }
}
