use image::EncodableLayout;
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
struct Entry {
    key: String,
    offset: u64,
}

#[derive(Serialize, Deserialize)]
enum Type {
    Texture(TextureMeta),
    Cubemap(CubemapMeta),
    Glb(GlbMeta),
}

#[derive(Serialize, Deserialize)]
struct TextureMeta {
    width: u32,
    height: u32,
    format: Option<String>,
    offset: u64,
}

#[derive(Serialize, Deserialize)]
struct CubemapMeta {
    size: u32,
    format: Option<String>,
    data: Vec<Entry>,
}

#[derive(Serialize, Deserialize)]
struct GlbMeta {
    offset: u64,
}

#[derive(Serialize, Deserialize)]
struct FurHeader {
    major: u16,
    minor: u16,
    ctype: Type,
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
            return Err("Failed to read file".to_string());
        }
        return Err(
            "Failed to read image dimensions. (Potentially failed to read file)".to_string(),
        );
    }

    Err("File does not exist.".to_string())
}

pub fn convert_cubemap(output: &Path, inputs: Vec<&Path>, overwrite: bool) -> Result<(), String> {
    let mut textures = Vec::<Vec<u8>>::new();

    let mut size = 0;
    let mut format = None;

    for input in inputs {
        if input.exists() {
            if let Ok(dimension) = image::image_dimensions(input) {
                if dimension.0 != dimension.1 {
                    return Err(
                        "Cubemap textures need to be quadratic. Width == Height".to_string()
                    );
                }

                if size == 0 {
                    size = dimension.0;
                } else if dimension.0 != size {
                    return Err("All textures must have the same size.".to_string());
                }

                if format.is_none() {
                    format = input
                        .extension()
                        .map(|s| s.to_os_string().into_string().unwrap_or("".to_string()));
                }

                if let Ok(texture) = std::fs::read(input) {
                    textures.push(texture);
                }
                return Err("Failed to read file".to_string());
            }
            return Err(
                "Failed to read image dimensions. (Potentially failed to read file)".to_string(),
            );
        }
    }

    if size == 0 {
        return Err("Invalid texture size.".to_string());
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
    if output.exists() && !overwrite {
        return Err("File already exists.".to_string());
    }

    let mut entries = Vec::<Entry>::new();
    let mut offset = 0;
    let keys = ["-x", "+x", "-y", "+y", "-z", "+z"];

    for i in 0..5 {
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
        return Err("No parent folder.".to_string());
    }

    if let Err(e) = std::fs::create_dir_all(output.parent().unwrap()) {
        return Err(format!("Could not create parent folders. Message: {}", e));
    }

    if let Ok(content) = serde_json::to_string(&header) {
        return std::fs::write(
            output,
            [
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

    Err("Could not serialize header.".to_string())
}

pub fn write_texture(
    output: &Path,
    texture: Vec<u8>,
    width: u32,
    format: Option<String>,
    height: u32,
    overwrite: bool,
) -> Result<(), String> {
    if output.exists() && !overwrite {
        return Err("File already exists.".to_string());
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
        return Err("No parent folder.".to_string());
    }

    if let Err(e) = std::fs::create_dir_all(output.parent().unwrap()) {
        return Err(format!("Could not create parent folders. Message: {}", e));
    }

    if let Ok(content) = serde_json::to_string(&header) {
        let size: u64 = content.as_bytes().len() as u64;
        return std::fs::write(
            output,
            [&size.to_be_bytes(), content.as_bytes(), texture.as_bytes()].concat(),
        )
        .map_err(|error| error.to_string());
    }

    Err("Could not serialize header.".to_string())
}

pub fn read_texture(path: &Path) -> Result<(Vec<u8>, (u32, u32)), String> {
    if !path.exists() {
        return Err("File does not exist.".to_string());
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
                _ => Err("Invalid asset type. Expected texture.".to_string()),
            };
        }

        return Err("Failed to deserialize meta data. Invalid format.".to_string());
    }

    Err("Failed to open file.".to_string())
}

pub fn read_cubemap(path: &Path) -> Result<(Vec<(String, Vec<u8>)>, u32), String> {
    if !path.exists() {
        return Err("File does not exist.".to_string());
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
                _ => return Err("Invalid asset type. Expected texture.".to_string()),
            }
        }

        return Err("Failed to deserialize meta data. Invalid format.".to_string());
    }

    Err("Failed to open file.".to_string())
}
