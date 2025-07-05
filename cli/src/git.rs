use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, Clone)]
pub struct GitRepository {
    pub url: String,
    pub local_path: PathBuf,
}

impl GitRepository {
    pub fn new(url: &str, local_path: PathBuf) -> Self {
        Self {
            url: url.to_string(),
            local_path,
        }
    }

    /// Convert github.com/user/repo format to git URL
    pub fn url_from_import_path(import_path: &str) -> String {
        if import_path.starts_with("http://") || import_path.starts_with("https://") {
            import_path.to_string()
        } else {
            format!("https://{}.git", import_path)
        }
    }

    /// Clone the repository if it doesn't exist
    pub fn clone_if_needed(&self) -> Result<(), String> {
        if self.local_path.exists() {
            println!("  Repository already exists at {}", self.local_path.display());
            return Ok(());
        }

        // Create parent directories
        if let Some(parent) = self.local_path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create parent directories: {}", e))?;
        }

        println!("  Cloning {} to {}", self.url, self.local_path.display());
        
        // Use git2 for cloning
        match git2::Repository::clone(&self.url, &self.local_path) {
            Ok(_) => {
                println!("  ✓ Successfully cloned repository");
                Ok(())
            }
            Err(e) => {
                // Fallback to command line git if git2 fails
                println!("  git2 failed ({}), trying command line git...", e);
                self.clone_with_command()
            }
        }
    }

    /// Fallback to command line git
    fn clone_with_command(&self) -> Result<(), String> {
        let output = Command::new("git")
            .args(&["clone", &self.url, &self.local_path.to_string_lossy()])
            .output()
            .map_err(|e| format!("Failed to execute git command: {}", e))?;

        if output.status.success() {
            println!("  ✓ Successfully cloned repository using git command");
            Ok(())
        } else {
            let error = String::from_utf8_lossy(&output.stderr);
            Err(format!("Git clone failed: {}", error))
        }
    }

    /// Checkout a specific version (tag, branch, or commit)
    pub fn checkout_version(&self, version: &str) -> Result<(), String> {
        if !self.local_path.exists() {
            return Err("Repository not found. Clone first.".to_string());
        }

        println!("  Checking out version: {}", version);

        // Open the repository
        let repo = git2::Repository::open(&self.local_path)
            .map_err(|e| format!("Failed to open repository: {}", e))?;

        // First, try to resolve as a branch or tag
        let resolved_version = if version == "latest" || version == "*" {
            "main".to_string()
        } else {
            version.to_string()
        };

        // Try different checkout strategies
        self.try_checkout_strategies(&repo, &resolved_version)
    }

    fn try_checkout_strategies(&self, repo: &git2::Repository, version: &str) -> Result<(), String> {
        // Strategy 1: Try as a branch
        if let Ok(branch) = repo.find_branch(version, git2::BranchType::Local) {
            return self.checkout_branch(repo, &branch);
        }

        // Strategy 2: Try as a remote branch
        let remote_branch_name = format!("origin/{}", version);
        if let Ok(branch) = repo.find_branch(&remote_branch_name, git2::BranchType::Remote) {
            return self.checkout_remote_branch(repo, &branch, version);
        }

        // Strategy 3: Try as a tag
        if let Ok(tag_ref) = repo.find_reference(&format!("refs/tags/{}", version)) {
            return self.checkout_tag(repo, &tag_ref);
        }

        // Strategy 4: Try as a commit hash
        if let Ok(oid) = git2::Oid::from_str(version) {
            if let Ok(commit) = repo.find_commit(oid) {
                return self.checkout_commit(repo, &commit);
            }
        }

        // Strategy 5: Fallback to main/master
        for default_branch in &["main", "master"] {
            if let Ok(branch) = repo.find_branch(default_branch, git2::BranchType::Local) {
                println!("  Version '{}' not found, falling back to '{}'", version, default_branch);
                return self.checkout_branch(repo, &branch);
            }
            
            let remote_branch_name = format!("origin/{}", default_branch);
            if let Ok(branch) = repo.find_branch(&remote_branch_name, git2::BranchType::Remote) {
                println!("  Version '{}' not found, falling back to '{}'", version, default_branch);
                return self.checkout_remote_branch(repo, &branch, default_branch);
            }
        }

        Err(format!("Could not find version '{}' in repository", version))
    }

    fn checkout_branch(&self, repo: &git2::Repository, branch: &git2::Branch) -> Result<(), String> {
        let branch_ref = branch.get();
        let commit = branch_ref.peel_to_commit()
            .map_err(|e| format!("Failed to get commit from branch: {}", e))?;

        repo.checkout_tree(commit.as_object(), None)
            .map_err(|e| format!("Failed to checkout tree: {}", e))?;

        repo.set_head(branch_ref.name().unwrap())
            .map_err(|e| format!("Failed to set HEAD: {}", e))?;

        println!("  ✓ Checked out branch: {}", branch.name().unwrap().unwrap_or("unknown"));
        Ok(())
    }

    fn checkout_remote_branch(&self, repo: &git2::Repository, branch: &git2::Branch, local_name: &str) -> Result<(), String> {
        let branch_ref = branch.get();
        let commit = branch_ref.peel_to_commit()
            .map_err(|e| format!("Failed to get commit from remote branch: {}", e))?;

        // Create a local branch tracking the remote
        let _local_branch = repo.branch(local_name, &commit, false)
            .map_err(|e| format!("Failed to create local branch: {}", e))?;

        repo.checkout_tree(commit.as_object(), None)
            .map_err(|e| format!("Failed to checkout tree: {}", e))?;

        repo.set_head(&format!("refs/heads/{}", local_name))
            .map_err(|e| format!("Failed to set HEAD: {}", e))?;

        println!("  ✓ Checked out remote branch as local: {}", local_name);
        Ok(())
    }

    fn checkout_tag(&self, repo: &git2::Repository, tag_ref: &git2::Reference) -> Result<(), String> {
        let commit = tag_ref.peel_to_commit()
            .map_err(|e| format!("Failed to get commit from tag: {}", e))?;

        repo.checkout_tree(commit.as_object(), None)
            .map_err(|e| format!("Failed to checkout tree: {}", e))?;

        repo.set_head_detached(commit.id())
            .map_err(|e| format!("Failed to set HEAD to tag: {}", e))?;

        println!("  ✓ Checked out tag: {}", tag_ref.shorthand().unwrap_or("unknown"));
        Ok(())
    }

    fn checkout_commit(&self, repo: &git2::Repository, commit: &git2::Commit) -> Result<(), String> {
        repo.checkout_tree(commit.as_object(), None)
            .map_err(|e| format!("Failed to checkout tree: {}", e))?;

        repo.set_head_detached(commit.id())
            .map_err(|e| format!("Failed to set HEAD to commit: {}", e))?;

        println!("  ✓ Checked out commit: {}", commit.id());
        Ok(())
    }

    /// Fetch latest changes from remote
    pub fn fetch(&self) -> Result<(), String> {
        if !self.local_path.exists() {
            return Err("Repository not found. Clone first.".to_string());
        }

        let repo = git2::Repository::open(&self.local_path)
            .map_err(|e| format!("Failed to open repository: {}", e))?;

        let mut remote = repo.find_remote("origin")
            .map_err(|e| format!("Failed to find origin remote: {}", e))?;

        println!("  Fetching latest changes...");
        remote.fetch(&[] as &[&str], None, None)
            .map_err(|e| format!("Failed to fetch: {}", e))?;

        println!("  ✓ Fetched latest changes");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_url_conversion() {
        assert_eq!(
            GitRepository::url_from_import_path("github.com/user/repo"),
            "https://github.com/user/repo.git"
        );
        
        assert_eq!(
            GitRepository::url_from_import_path("https://github.com/user/repo.git"),
            "https://github.com/user/repo.git"
        );
    }
}
