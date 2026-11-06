//! Jetons à scopes. Fichier : une ligne par jeton, `scope1,scope2:secret`, `#` pour commenter.

use sil_crypto::ct_eq;
use std::{fs, io::Write, path::Path};
use thiserror::Error;

/// Droit accordé à un jeton.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scope {
    /// Lire l'état (jobs, pairs, règlements).
    Read,
    /// Envoyer du contenu et des événements signés.
    Submit,
}

#[derive(Debug, Error)]
pub enum TokenError {
    #[error("ligne {0} : format `scopes:secret` attendu")]
    Format(usize),
    #[error("ligne {0} : scope inconnu (read, submit)")]
    UnknownScope(usize),
    #[error("ligne {0} : secret trop court (32 caractères au moins) ou invalide")]
    WeakSecret(usize),
    #[error("lecture du fichier de jetons : {0}")]
    Io(String),
}

/// Identité authentifiée : l'indice du jeton (pour la limite de débit) et ses scopes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Principal {
    pub id: usize,
    read: bool,
    submit: bool,
}

impl Principal {
    pub fn allows(&self, scope: Scope) -> bool {
        match scope {
            Scope::Read => self.read,
            Scope::Submit => self.submit,
        }
    }
}

struct Token {
    secret: Vec<u8>,
    principal: Principal,
}

#[derive(Default)]
pub struct TokenSet {
    tokens: Vec<Token>,
}

fn valid_secret(secret: &str) -> bool {
    (32..=128).contains(&secret.len()) && secret.bytes().all(|b| b.is_ascii_graphic())
}

impl TokenSet {
    pub fn parse(text: &str) -> Result<Self, TokenError> {
        let mut tokens = Vec::new();
        for (index, line) in text.lines().enumerate() {
            let number = index + 1;
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let (scopes, secret) = line.split_once(':').ok_or(TokenError::Format(number))?;
            let (mut read, mut submit) = (false, false);
            for scope in scopes.split(',') {
                match scope.trim() {
                    "read" => read = true,
                    "submit" => submit = true,
                    _ => return Err(TokenError::UnknownScope(number)),
                }
            }
            if !valid_secret(secret) {
                return Err(TokenError::WeakSecret(number));
            }
            tokens.push(Token {
                secret: secret.as_bytes().to_vec(),
                principal: Principal {
                    id: tokens.len(),
                    read,
                    submit,
                },
            });
        }
        Ok(Self { tokens })
    }

    /// Lit le fichier, ou en crée un avec un jeton `read,submit` aléatoire (mode 0600 sous Unix).
    pub fn load_or_create(path: &Path) -> Result<Self, TokenError> {
        let io = |e: std::io::Error| TokenError::Io(e.to_string());
        match fs::read_to_string(path) {
            Ok(text) => return Self::parse(&text),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(io(e)),
        }
        let mut raw = [0u8; 32];
        getrandom::getrandom(&mut raw).map_err(|e| TokenError::Io(e.to_string()))?;
        let secret: String = raw.iter().map(|b| format!("{b:02x}")).collect();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(io)?;
        }
        let mut options = fs::OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        std::os::unix::fs::OpenOptionsExt::mode(&mut options, 0o600);
        let mut file = options.open(path).map_err(io)?;
        writeln!(
            file,
            "# Jeton généré au premier démarrage.\nread,submit:{secret}"
        )
        .map_err(io)?;
        Self::parse(&fs::read_to_string(path).map_err(io)?)
    }

    pub fn is_empty(&self) -> bool {
        self.tokens.is_empty()
    }

    /// Authentifie un en-tête `Authorization`. Compare **tous** les jetons sans s'arrêter au
    /// premier trouvé : la durée ne révèle ni quel jeton ni sa position.
    pub fn authenticate(&self, header: Option<&str>) -> Option<Principal> {
        let presented = header?.strip_prefix("Bearer ")?.as_bytes();
        let mut found = None;
        for token in &self.tokens {
            if ct_eq(&token.secret, presented) {
                found = Some(token.principal);
            }
        }
        found
    }
}
