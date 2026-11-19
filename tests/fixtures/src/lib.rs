use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;

fn data(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join(name)
}

/// The hackathon reference package: a Fleet holding `workflows/hackathon` and the
/// skills and agents its graph names.
pub fn hackathon_dir() -> PathBuf {
    data("hackathon")
}

/// The small Fleet: two skills and one agent.
pub fn fleet_dir() -> PathBuf {
    data("fleet")
}

/// The mixed-language test repository's files (not yet a Git repository).
pub fn mixed_repo_dir() -> PathBuf {
    data("repo")
}

/// Copies a directory tree, files and executable bits, into `dest`.
pub fn copy_tree(src: &Path, dest: &Path) -> io::Result<()> {
    fs::create_dir_all(dest)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let target = dest.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_tree(&entry.path(), &target)?;
        } else {
            fs::copy(entry.path(), &target)?;
        }
    }
    Ok(())
}

/// Runs git with a fixed identity and fixed dates so commits are reproducible.
fn git(dir: &Path, args: &[&str]) -> io::Result<String> {
    let out = Command::new("git")
        .args(args)
        .current_dir(dir)
        .env("GIT_AUTHOR_NAME", "Fixture")
        .env("GIT_AUTHOR_EMAIL", "fixture@example.com")
        .env("GIT_COMMITTER_NAME", "Fixture")
        .env("GIT_COMMITTER_EMAIL", "fixture@example.com")
        .env("GIT_AUTHOR_DATE", "2026-01-01T00:00:00Z")
        .env("GIT_COMMITTER_DATE", "2026-01-01T00:00:00Z")
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_SYSTEM", "/dev/null")
        .output()?;
    if !out.status.success() {
        return Err(io::Error::other(format!(
            "git {}: {}",
            args.join(" "),
            String::from_utf8_lossy(&out.stderr).trim()
        )));
    }
    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
}

/// Turns `dir` into a Git repository on `main` with one commit of everything in it.
fn commit_all(dir: &Path) -> io::Result<()> {
    git(dir, &["init", "-q", "-b", "main"])?;
    git(dir, &["add", "-A"])?;
    git(dir, &["commit", "-q", "-m", "Initial commit"])?;
    Ok(())
}

/// Writes the mixed-language repository into `dest` as a Git repository on `main`
/// with one commit and a clean tree.
pub fn materialize_repo(dest: &Path) -> io::Result<()> {
    copy_tree(&mixed_repo_dir(), dest)?;
    commit_all(dest)
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
        let path = root.join("server.git");
        if !path.join("HEAD").is_file() {
            fs::create_dir_all(&path)?;
            git(&path, &["init", "-q", "--bare", "-b", "main"])?;
        }
        Ok(GitServer { path })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    /// The URL to clone from or push to.
    pub fn url(&self) -> String {
        self.path.display().to_string()
    }

    /// Replaces `main` on the server with one commit of the contents of `dir`.
    pub fn seed(&self, dir: &Path) -> io::Result<()> {
        let work = tempfile::tempdir()?;
        copy_tree(dir, work.path())?;
        commit_all(work.path())?;
        git(work.path(), &["push", "-q", "--force", &self.url(), "main"])?;
        Ok(())
    }

    /// Branch names on the server, sorted.
    pub fn branches(&self) -> io::Result<Vec<String>> {
        let out = git(
            &self.path,
            &["for-each-ref", "--format=%(refname:short)", "refs/heads"],
        )?;
        let mut names: Vec<String> = out.lines().map(str::to_string).collect();
        names.sort();
        Ok(names)
    }
}
