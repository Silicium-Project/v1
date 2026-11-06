//! Emplacements sur disque : `objects/<2 premiers hex>/<hex>.{data,manifest}`, `tmp/`.

use sil_types::ContentId;
use std::path::{Path, PathBuf};

pub(crate) fn object_dir(root: &Path, id: &ContentId) -> PathBuf {
    root.join("objects").join(&id.to_hex()[..2])
}

pub(crate) fn data_path(root: &Path, id: &ContentId) -> PathBuf {
    object_dir(root, id).join(format!("{id}.data"))
}

pub(crate) fn manifest_path(root: &Path, id: &ContentId) -> PathBuf {
    object_dir(root, id).join(format!("{id}.manifest"))
}

pub(crate) fn tmp_dir(root: &Path) -> PathBuf {
    root.join("tmp")
}

pub(crate) fn part_path(root: &Path, id: &ContentId) -> PathBuf {
    tmp_dir(root).join(format!("{id}.part"))
}

pub(crate) fn have_path(root: &Path, id: &ContentId) -> PathBuf {
    tmp_dir(root).join(format!("{id}.have"))
}
