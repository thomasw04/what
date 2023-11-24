use image::EncodableLayout;
use serde::{Deserialize, Serialize};
use std::{fmt::format, path::Path};

mod importer;

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
struct HeaderEntry {
    key: String,
    offset: u64,
}

#[derive(Serialize, Deserialize)]
enum HeaderType {
    Texture(HeaderTexture),
    Cubemap(HeaderCubemap),
    Glb(HeaderGlb),
}

#[derive(Serialize, Deserialize)]
struct HeaderTexture {
    width: u32,
    height: u32,
    format: Option<String>,
    offset: u64,
}

#[derive(Serialize, Deserialize)]
struct HeaderCubemap {
    size: u32,
    format: Option<String>,
    data: Vec<HeaderEntry>,
}

#[derive(Serialize, Deserialize)]
struct HeaderGlb {
    offset: u64,
}

#[derive(Serialize, Deserialize)]
struct FurHeader {
    major: u16,
    minor: u16,
    ctype: HeaderType,
}

pub enum Asset {
    Texture((Vec<u8>, (u32, u32))),
    Cubemap(Vec<(Vec<u8>, (u32, u32))>),
    Glb()
}

pub fn test() {
    gltf::import_buffers()
}

pub fn convert_texture(output: &Path, input: &Path, overwrite: bool) -> Result<(), String> {
    if input.exists() {
        if let Ok(dimension) = image::image_dimensions(input) {
            if let Ok(texture) = std::fs::read(input) {
                return write_texture(
                    output,
                    texture,
                    dimension.0,
                    input
                        .extension()
                        .map(|s| s.to_os_string().into_string().unwrap_or("".to_string())),
                    dimension.1,
                    overwrite,
                );
            }
            return Err(format!("Failed to read file: {}", input.display()));
        }
        return Err(format!(
            "Failed to read image dimensions or file: {}",
            input.display()
        ));
    }

    Err(format!("File {} does not exist.", input.display()))
}

pub fn convert_cubemap(output: &Path, inputs: Vec<&Path>, overwrite: bool) -> Result<(), String> {
    let mut textures = Vec::<Vec<u8>>::new();

    let mut size = 0;
    let mut format = None;

    for input in inputs {
        if input.exists() {
            if let Ok(dimension) = image::image_dimensions(input) {
                if dimension.0 != dimension.1 {
                    return Err(format!(
                        "Cubemap textures need to be quadratic. File: {}, ",
                        input.display()
                    ));
                }

                if size == 0 {
                    size = dimension.0;
                } else if dimension.0 != size {
                    return Err(format!(
                        "All textures must have the same size. File {}",
                        input.display()
                    ));
                }

                if format.is_none() {
                    format = input
                        .extension()
                        .map(|s| s.to_os_string().into_string().unwrap_or("".to_string()));
                }

                if let Ok(texture) = std::fs::read(input) {
                    textures.push(texture);
                } else {
                    return Err(format!("Failed to read file: {}", input.display()));
                }
            } else {
                return Err(format!(
                    "Failed to read image dimensions or file: {}",
                    input.display()
                ));
            }
        }
    }

    if size == 0 {
        return Err("Invalid texture count.".to_string());
    }

    write_cubemap(output, &textures, size, format, overwrite)
}

pub fn write_cubemap(
    output: &Path,
    textures: &[Vec<u8>],
    size: u32,
    format: Option<String>,
    overwrite: bool,
) -> Result<(), String> {
    if output.exists() {
        if overwrite {
            log::warn!("Overwrite flag set. Overwriting file {}", output.display());
        } else {
            return Err(format!("File {} already exists.", output.display()));
        }
    }

    let mut entries = Vec::<Entry>::new();
    let mut offset = 0;
    let keys = ["-x", "+x", "-y", "+y", "-z", "+z"];

    for i in 0..6 {
        entries.push(Entry {
            key: keys[i].to_string(),
            offset,
        });

        offset += textures[i].len() as u64;
    }

    let header = FurHeader {
        major: 1,
        minor: 0,
        ctype: Type::Cubemap(CubemapMeta {
            size,
            format,
            data: entries,
        }),
    };

    if output.parent().is_none() {
        return Err(format!("{} has no parent folder.", output.display()));
    }

    if let Err(e) = std::fs::create_dir_all(output.parent().unwrap()) {
        return Err(format!(
            "Could not create parent folders of {}. Message: {}",
            output.display(),
            e
        ));
    }

    if let Ok(content) = serde_json::to_string(&header) {
        let size: u64 = content.as_bytes().len() as u64;
        return std::fs::write(
            output,
            [
                &size.to_be_bytes(),
                content.as_bytes(),
                textures[0].as_bytes(),
                textures[1].as_bytes(),
                textures[2].as_bytes(),
                textures[3].as_bytes(),
                textures[4].as_bytes(),
                textures[5].as_bytes(),
            ]
            .concat(),
        )
        .map_err(|error| error.to_string());
    }

    Err(format!(
        "Could not serialize header of {}.",
        output.display()
    ))
}

pub fn write_texture(
    output: &Path,
    texture: Vec<u8>,
    width: u32,
    format: Option<String>,
    height: u32,
    overwrite: bool,
) -> Result<(), String> {
    if output.exists() {
        if overwrite {
            log::warn!("Overwrite flag set. Overwriting file {}", output.display());
        } else {
            return Err(format!("File {} already exists.", output.display()));
        }
    }

    let header = FurHeader {
        major: 1,
        minor: 0,
        ctype: Type::Texture(TextureMeta {
            width,
            height,
            format,
            offset: 0,
        }),
    };

    if output.parent().is_none() {
        return Err(format!("{} has no parent folder.", output.display()));
    }

    if let Err(e) = std::fs::create_dir_all(output.parent().unwrap()) {
        return Err(format!(
            "Could not create parent folders of {}. Message: {}",
            output.display(),
            e
        ));
    }

    if let Ok(content) = serde_json::to_string(&header) {
        let size: u64 = content.as_bytes().len() as u64;
        return std::fs::write(
            output,
            [&size.to_be_bytes(), content.as_bytes(), texture.as_bytes()].concat(),
        )
        .map_err(|error| error.to_string());
    }

    Err(format!(
        "Could not serialize header of {}.",
        output.display()
    ))
}

trait Backend {
    fn read_file(&self, path: String) -> Result<Vec<u8>, String>;
    fn write_file(&self, path: String, bytes: Vec<u8>, overwrite: bool) -> Result<(), String>;
}

struct What {}

#[cfg(target_arch = "wasm32")]
impl Backend for What {
    fn read_file(&self, path: String) -> Result<Vec<u8>, String> {
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

    fn write_file(&self, _path: String, _bytes: Vec<u8>, _overwrite: bool) -> Result<(), String> {
        unimplemented!("Files can only be written in native builds, not in WASM")
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl Backend for What {
    fn read_file(&self, path: String) -> Result<Vec<u8>, String> {
        let path = Path::new(&path);

        if !path.exists() {
            return Err(format!("File {} does not exist.", path.display()));
        }

        std::fs::read(path)
            .map_err(|err| format!("Failed to read file {}. Err: {}", path.display(), err))
    }

    fn write_file(&self, path: String, content: Vec<u8>, overwrite: bool) -> Result<(), String> {
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

impl What {
    
    pub fn 
}

pub fn read_texture(path: &Path) -> Result<(Vec<u8>, (u32, u32)), String> {
    if !path.exists() {
        return Err(format!("File {} does not exist.", path.display()));
    }

    if let Ok(file) = std::fs::read(path) {
        //Convert to size
        let mut size_buf = [0u8; 8];
        size_buf[..8].copy_from_slice(&file.as_bytes()[..8]);
        let size = u64::from_be_bytes(size_buf);

        if let Ok(meta) = serde_json::from_slice::<FurHeader>(&file[8..(size as usize + 7)]) {
            return match meta.ctype {
                Type::Texture(texture_meta) => Ok((
                    file[(size as usize + 7)..].to_vec(),
                    (texture_meta.width, texture_meta.height),
                )),
                _ => Err(format!(
                    "Invalid asset type of {}. Expected texture.",
                    path.display()
                )),
            };
        }

        return Err(format!(
            "Failed to deserialize meta data of {}. Invalid format.",
            path.display()
        ));
    }

    Err(format!("Failed to open file: {}", path.display()))
}

pub fn read_cubemap(path: &Path) -> Result<(Vec<(String, Vec<u8>)>, u32), String> {
    if !path.exists() {
        return Err(format!("File {} does not exist.", path.display()));
    }

    if let Ok(file) = std::fs::read(path) {
        //Convert to size
        let mut size_buf = [0u8; 8];
        size_buf[..8].copy_from_slice(&file.as_bytes()[..8]);
        let size = u64::from_be_bytes(size_buf);

        if let Ok(meta) = serde_json::from_slice::<FurHeader>(&file[8..(size as usize + 7)]) {
            match meta.ctype {
                Type::Cubemap(cubemap_meta) => {
                    let mut textures = Vec::<(String, Vec<u8>)>::new();
                    for (i, entry) in cubemap_meta.data.iter().enumerate() {
                        let default_end = Entry {
                            key: "".to_string(),
                            offset: u64::MAX,
                        };
                        let end = cubemap_meta.data.get(i + 1).unwrap_or(&default_end);
                        textures.push((
                            entry.key.clone(),
                            file[((size as usize + 7) + entry.offset as usize)
                                ..end.offset as usize]
                                .to_vec(),
                        ));
                    }
                    return Ok((textures, cubemap_meta.size));
                }
                _ => {
                    return Err(format!(
                        "Invalid asset type of {}. Expected texture.",
                        path.display()
                    ))
                }
            }
        }

        return Err(format!(
            "Failed to deserialize meta data of {}. Invalid format.",
            path.display()
        ));
    }

    Err(format!("Failed to open file: {}", path.display()))
}
