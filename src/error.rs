use std::fmt::Debug;

pub enum Error {
    Io(std::io::Error),
    GltfError(gltf::Error),
    JsonError(serde_json::Error),
    Unknown(String),
}

impl Debug for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Io(err) => write!(f, "Io error: {}", err),
            Error::GltfError(err) => write!(f, "Gltf error: {}", err),
            Error::JsonError(err) => write!(f, "Json error: {}", err),
            Error::Unknown(err) => write!(f, "Unknown error: {}", err),
        }
    }
}
