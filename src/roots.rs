//! TODO
use crate::project::Project;
use std::env;
use std::os::unix::fs::symlink;
use std::path::{Path, PathBuf};

/// Roots manipulation
#[derive(Clone)]
pub struct Roots {
    root_dir: PathBuf,
    id: String,
}

impl Roots {
    /// Create a Roots struct to manage roots within the root_dir
    /// directory.
    ///
    /// `id` is a unique identifier for this project's checkout.
    pub fn new(root_dir: PathBuf, id: String) -> Roots {
        Roots { root_dir, id }
    }

    /// Construct a Roots struct based on a project's GC root directory
    /// and ID.
    pub fn from_project(project: &Project) -> Result<Roots, std::io::Error> {
        Ok(Roots::new(project.gc_root_path()?, project.id()))
    }

    /// Store a new root under name
    pub fn add(&self, name: &str, store_path: &PathBuf) -> Result<PathBuf, AddRootError> {
        let mut path = self.root_dir.clone();
        path.push(name);

        debug!("Adding root from {:?} to {:?}", store_path, path,);
        std::fs::remove_file(&path).or_else(|e| AddRootError::remove(e, &path))?;

        std::fs::remove_file(&path).or_else(|e| AddRootError::remove(e, &path))?;

        symlink(&store_path, &path).map_err(|e| AddRootError::symlink(e, &store_path, &path))?;

        let mut root = if let Ok(path) = env::var("NIX_STATE_DIR") {
            PathBuf::from(path)
        } else {
            PathBuf::from("/nix/var/nix/")
        };
        root.push("gcroots");
        root.push("per-user");

        root.push(env::var("USER").expect("env var 'USER' must be set"));

        // The user directory sometimes doesn’t exist,
        // but we can create it (it’s root but `rwxrwxrwx`)
        ensure_directory_exists(&root);

        root.push(format!("{}-{}", self.id, name));

        debug!("Connecting root from {:?} to {:?}", path, root,);
        std::fs::remove_file(&root).or_else(|e| AddRootError::remove(e, &root))?;

        symlink(&path, &root).map_err(|e| AddRootError::symlink(e, &path, &root))?;

        Ok(path)
    }
}

/// Check if a directory exists and is a directory.
/// Try to create it if nothing’s there.
/// Panic for everything else.
fn ensure_directory_exists(dir: &PathBuf) {
    match std::fs::metadata(dir) {
        Err(e) => match e.kind() {
            std::io::ErrorKind::NotFound => std::fs::create_dir(dir).expect(
                format!(
                    "directory {} does not exist and we can’t create it",
                    dir.display()
                )
                .as_str(),
            ),
            _ => {}
        },
        Ok(meta) => {
            if !meta.is_dir() {
                panic!("{} is not a directory", dir.display())
            }
        }
    }
}

/// Error conditions encountered when adding roots
#[derive(Debug)]
pub enum AddRootError {
    /// IO-related errors
    Io(std::io::Error, String),
}

impl AddRootError {
    /// Ignore NotFound errors (it is after all a remove), and otherwise
    /// return an error explaining a delete on path failed.
    fn remove(err: std::io::Error, path: &Path) -> Result<(), AddRootError> {
        if err.kind() == std::io::ErrorKind::NotFound {
            Ok(())
        } else {
            Err(AddRootError::Io(
                err,
                format!("Failed to delete {}", path.display()),
            ))
        }
    }

    /// Return an error explaining what symlink failed
    fn symlink(err: std::io::Error, src: &Path, dest: &Path) -> AddRootError {
        AddRootError::Io(
            err,
            format!("Failed to symlink {} to {}", src.display(), dest.display()),
        )
    }
}
