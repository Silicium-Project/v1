use crate::{ModelError, validate::validate};
use serde::Deserialize;
use sil_content::ContentStore;
use sil_types::ContentId;
use std::collections::{BTreeMap, BTreeSet};

/// Un fichier d'un modèle, épinglé.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ModelFile {
    /// Chemin relatif dans le modèle (`a-z A-Z 0-9 . _ -` et `/`, sans `..`).
    pub path: String,
    pub id: ContentId,
    pub size: u64,
}

/// Un modèle : un nom et ses fichiers.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Model {
    pub name: String,
    #[serde(rename = "file")]
    pub files: Vec<ModelFile>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Raw {
    #[serde(default, rename = "model")]
    models: Vec<Model>,
}

/// Catalogue validé.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Catalog {
    models: BTreeMap<String, Model>,
    pinned: BTreeSet<ContentId>,
}

impl Catalog {
    /// Lit et valide un catalogue TOML.
    pub fn parse(text: &str) -> Result<Self, ModelError> {
        let raw: Raw = toml::from_str(text).map_err(|e| ModelError::Parse(e.message().into()))?;
        validate(&raw.models)?;
        let pinned = raw
            .models
            .iter()
            .flat_map(|m| m.files.iter().map(|f| f.id))
            .collect();
        let models = raw
            .models
            .into_iter()
            .map(|m| (m.name.clone(), m))
            .collect();
        Ok(Self { models, pinned })
    }

    pub fn get(&self, name: &str) -> Result<&Model, ModelError> {
        self.models
            .get(name)
            .ok_or_else(|| ModelError::UnknownModel(name.into()))
    }

    pub fn models(&self) -> impl Iterator<Item = &Model> {
        self.models.values()
    }

    /// Ce contenu appartient-il au catalogue ? C'est la seule question que le réseau pose
    /// avant de télécharger ou de servir un modèle.
    pub fn is_pinned(&self, id: &ContentId) -> bool {
        self.pinned.contains(id)
    }

    /// Fichiers du modèle absents du magasin.
    pub fn missing<'a>(
        &'a self,
        name: &str,
        store: &ContentStore,
    ) -> Result<Vec<&'a ModelFile>, ModelError> {
        Ok(self
            .get(name)?
            .files
            .iter()
            .filter(|file| !store.has(&file.id))
            .collect())
    }

    /// Le modèle est-il complet dans le magasin ?
    pub fn is_ready(&self, name: &str, store: &ContentStore) -> Result<bool, ModelError> {
        Ok(self.missing(name, store)?.is_empty())
    }
}
