use std::path::{Path, PathBuf};

use crate::{error::Error, Location};

pub trait Backend {
    fn read_file(
        base: &Option<Location>,
        path: &str,
    ) -> Result<(Vec<u8>, Option<Vec<(String, Vec<u8>)>>), Error>;
    fn write_file(path: &str, bytes: Vec<u8>, overwrite: bool) -> Result<(), String>;
}

#[cfg(target_arch = "wasm32")]
impl Backend for What {
    fn read_file(path: &str) -> Result<Vec<u8>, String> {
        const MAX_REQUESTS: usize = 5;

        for i in 0..MAX_REQUESTS {
            match ureq::get(&path).call() {
                Ok(file) => {
                    let mut bytes: Vec<u8> = if let Some(value) = file.header("Content-Length") {
                        value.parse().map_or(Vec::new(), Vec::with_capacity)
                    } else {
                        Vec::new()
                    };

                    return if let Err(err) = file.into_reader().read_to_end(&mut bytes) {
                        Err(format!("Failed to read file at {}. Err: {}", path, err))
                    } else {
                        Ok(bytes)
                    };
                }
                Err(err) => {
                    log::warn!(
                        "Failed to retrieve file {}. ({}/{MAX_REQUESTS}) Trying again. Err: {}",
                        path,
                        i + 1,
                        err
                    );
                }
            }
        }

        Err(format!("Failed to retrieve file {}.", path))
    }

    fn write_file(_path: &str, _bytes: Vec<u8>, _overwrite: bool) -> Result<(), String> {
        unimplemented!("Files can only be written in native builds, not in WASM")
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl Backend for crate::What {
    fn read_file(
        base: &Option<Location>,
        path: &str,
    ) -> Result<(Vec<u8>, Option<Vec<(String, Vec<u8>)>>), Error> {
        match base {
            Some(Location::File(base)) => read_file_impl(&base.join(path)),
            Some(Location::Http(_base)) => todo!("Http fetch not yet implemented."),
            None => read_file_impl(&PathBuf::from(path)),
        }
    }

    fn write_file(path: &str, content: Vec<u8>, overwrite: bool) -> Result<(), String> {
        let path = Path::new(&path);

        if path.exists() {
            if overwrite {
                log::warn!("Overwrite flag set. Overwriting file {}", path.display());
            } else {
                return Err(format!("File {} already exists.", path.display()));
            }
        }

        if let Err(e) = std::fs::create_dir_all(path.parent().unwrap()) {
            return Err(format!(
                "Could not create parent folders of {}. Err: {}",
                path.display(),
                e
            ));
        }

        std::fs::write(path, content)
            .map_err(|err| format!("Failed to write to file {}. Err: {}", path.display(), err))
    }
}

fn read_file_impl(path: &PathBuf) -> Result<(Vec<u8>, Option<Vec<(String, Vec<u8>)>>), Error> {
    if !path.exists() {
        return Err(Error::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("File {} not found.", path.display()),
        )));
    }

    std::fs::read(path)
        .map_err(Error::Io)
        .map(|bytes| (bytes, None))
}
