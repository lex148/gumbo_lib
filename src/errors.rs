use thiserror::Error;
pub type Result<T> = std::result::Result<T, GumboError>;

#[derive(Debug, Error)]
pub enum GumboError {
    #[error("Error Reading IO")]
    IoError(#[from] std::io::Error),
}
