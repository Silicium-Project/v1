use crate::{Model, ModelError};
use std::collections::BTreeSet;

const MAX_MODELS: usize = 256;
const MAX_FILES: usize = 64;
const MAX_PATH_LEN: usize = 200;
const MAX_FILE_BYTES: u64 = 64 * 1024 * 1024 * 1024;

fn invalid(reason: String) -> ModelError {
    ModelError::Invalid(reason)
}

fn safe_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 80
        && name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"._-".contains(&b))
}

/// Chemin relatif sûr : composants `[A-Za-z0-9._-]` séparés par `/`, jamais `.` ni `..`.
fn safe_path(path: &str) -> bool {
    !path.is_empty()
        && path.len() <= MAX_PATH_LEN
        && path
            .split('/')
            .all(|part| safe_name(part) && part != "." && part != "..")
}

pub(crate) fn validate(models: &[Model]) -> Result<(), ModelError> {
    if models.len() > MAX_MODELS {
        return Err(invalid("trop de modèles".into()));
    }
    let mut names = BTreeSet::new();
    for model in models {
        if !safe_name(&model.name) || !names.insert(&model.name) {
            return Err(invalid(format!("nom de modèle : {}", model.name)));
        }
        if model.files.is_empty() || model.files.len() > MAX_FILES {
            return Err(invalid(format!("fichiers de {}", model.name)));
        }
        let mut paths = BTreeSet::new();
        for file in &model.files {
            if !safe_path(&file.path) || !paths.insert(&file.path) {
                return Err(invalid(format!("chemin : {}/{}", model.name, file.path)));
            }
            if file.size > MAX_FILE_BYTES {
                return Err(invalid(format!("taille : {}/{}", model.name, file.path)));
            }
        }
    }
    Ok(())
}
