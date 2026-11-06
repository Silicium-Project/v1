use thiserror::Error;

#[derive(Debug, Error)]
pub enum NodeError {
    #[error("configuration : {0}")]
    Config(String),
    #[error("stockage : {0}")]
    Storage(String),
    #[error("réseau : {0}")]
    Net(String),
    #[error("API de contrôle : {0}")]
    Control(String),
}
