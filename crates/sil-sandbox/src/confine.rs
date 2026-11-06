use crate::SandboxError;
use std::{
    fs,
    path::{Component, Path, PathBuf},
};

/// Racine dans laquelle tout chemin fourni par un tiers (réseau, API) est résolu.
#[derive(Debug, Clone)]
pub struct Confined {
    root: PathBuf,
}

fn bad(path: &str) -> SandboxError {
    SandboxError::BadPath(path.to_string())
}

impl Confined {
    /// Crée la racine si besoin et la canonicalise.
    pub fn new(root: &Path) -> Result<Self, SandboxError> {
        fs::create_dir_all(root).map_err(|e| SandboxError::Io(e.to_string()))?;
        let root = root
            .canonicalize()
            .map_err(|e| SandboxError::Io(e.to_string()))?;
        Ok(Self { root })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Résout un chemin **relatif** sous la racine.
    ///
    /// Refusés : vide, absolu, `..`, `.`, antislash, `:`, octet nul, composant vide, et
    /// tout composant existant qui est un lien symbolique ou dont la cible sort de la racine
    /// (liens, jonctions Windows). Le contrôle porte sur l'état du disque *au moment de
    /// l'appel* : il faut résoudre juste avant d'ouvrir, dans un dossier que le tiers ne
    /// contrôle pas.
    pub fn resolve(&self, relative: &str) -> Result<PathBuf, SandboxError> {
        if relative.is_empty() || relative.contains(['\\', ':', '\0']) {
            return Err(bad(relative));
        }
        let mut path = self.root.clone();
        for component in Path::new(relative).components() {
            let Component::Normal(part) = component else {
                return Err(bad(relative));
            };
            path.push(part);
            if let Ok(meta) = fs::symlink_metadata(&path) {
                if meta.file_type().is_symlink() {
                    return Err(SandboxError::Symlink(path));
                }
                let real = path
                    .canonicalize()
                    .map_err(|e| SandboxError::Io(e.to_string()))?;
                if !real.starts_with(&self.root) {
                    return Err(SandboxError::Escape(path));
                }
            }
        }
        if relative.split('/').any(str::is_empty) {
            return Err(bad(relative));
        }
        Ok(path)
    }
}
