use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum NetError {
    #[error("configuration réseau invalide : {0}")]
    Config(String),
    #[error("écoute impossible : {0}")]
    Listen(String),
    #[error("connexion impossible : {0}")]
    Dial(String),
    #[error("publication refusée : {0}")]
    Publish(String),
    #[error("message trop grand pour ce topic ({len} > {max})")]
    TooLarge { len: usize, max: usize },
    #[error("le réseau est arrêté")]
    Closed,
    #[error("délai dépassé")]
    Timeout,
    #[error("échec de la requête : {0}")]
    Request(String),
}
