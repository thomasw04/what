use image::EncodableLayout;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, io::Read, path::Path, vec};

//Public Types
pub enum Asset {
    Texture {
        width: u32,
        height: u32,
        data: Vec<u8>,
    },
    Cubemap {
        size: u32,
        data: HashMap<String, Vec<u8>>,
    },
    Glb {
        data: Vec<u8>,
    },
}

pub enum AssetType {
    Texture,
    Cubemap,
    Glb,
}

//Internal Types
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
struct Entry {
    key: String,
    offset: u64,
    size: u64,
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
            size: textures[i].len() as u64,
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

pub fn read_asset(path: &Path) -> Result<Asset, String> {
    if !path.exists() {
        return Err(format!("File {} does not exist.", path.display()));
    }

    match std::fs::File::open(path) {
        Ok(mut file) => match read_header(&mut file) {
            Ok(header) => match header.ctype {
                Type::Texture(meta) => {
                    let mut data = Vec::<u8>::new();
                    let res = file.read_to_end(&mut data);

                    if let Err(e) = res {
                        return Err(format!("{}", e));
                    }

                    Ok(Asset::Texture {
                        width: meta.width,
                        height: meta.height,
                        data,
                    })
                }
                Type::Cubemap(meta) => {
                    let mut data = HashMap::<String, Vec<u8>>::new();

                    for entry in meta.data {
                        data.insert(entry.key.clone(), vec![0 as u8; entry.size as usize]);
                        let res = file.read_exact(
                            data.get_mut(&entry.key)
                                .expect("This is impossible the fail. What did you do?"),
                        );

                        if let Err(e) = res {
                            return Err(format!("{}", e));
                        }
                    }

                    Ok(Asset::Cubemap {
                        size: meta.size,
                        data: data,
                    })
                }
                Type::Glb(_meta) => {
                    let mut data = Vec::<u8>::new();
                    let res = file.read_to_end(&mut data);

                    if let Err(e) = res {
                        return Err(format!("{}", e));
                    }

                    Ok(Asset::Glb { data })
                }
            },
            Err(e) => Err(format!("{}", e)),
        },
        Err(e) => Err(format!("{}", e)),
    }
}

fn read_header(file: &mut std::fs::File) -> Result<FurHeader, String> {
    let mut size_buf = [0u8; 8];
    if let Err(e) = file.read_exact(&mut size_buf) {
        return Err(format!("{}", e));
    }
    let size = u64::from_be_bytes(size_buf);

    let mut header_buf = vec![0 as u8; size as usize];
    if let Err(e) = file.read_exact(&mut header_buf) {
        return Err(format!("{}", e));
    }

    match serde_json::from_slice::<FurHeader>(header_buf.as_slice()) {
        Ok(meta) => Ok(meta),
        Err(e) => return Err(format!("{}", e)),
    }
}
