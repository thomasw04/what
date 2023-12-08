use backend::Backend;
use error::Error;
use image::EncodableLayout;
use lfu::LfuCache;
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
};

mod backend;
mod error;
mod lfu;

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
struct HeaderEntry {
    key: String,
    offset: u64,
}

#[derive(Serialize, Deserialize)]
enum HeaderType {
    Texture(HeaderTexture),
    Cubemap(HeaderCubemap),
    Gltf(HeaderGltf),
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
struct HeaderGltf {
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
    Cubemap(Vec<(String, Vec<u8>, (u32, u32))>),
    Glb(
        gltf::Document,
        Vec<gltf::buffer::Data>,
        Vec<gltf::image::Data>,
    ),
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

    let mut entries = Vec::<HeaderEntry>::new();
    let mut offset = 0;
    let keys = ["-x", "+x", "-y", "+y", "-z", "+z"];

    for i in 0..6 {
        entries.push(HeaderEntry {
            key: keys[i].to_string(),
            offset,
        });

        offset += textures[i].len() as u64;
    }

    let header = FurHeader {
        major: 1,
        minor: 0,
        ctype: HeaderType::Cubemap(HeaderCubemap {
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
        ctype: HeaderType::Texture(HeaderTexture {
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

pub enum Location {
    File(PathBuf),
    Http(String),
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
struct Guid {
    id: u32,
}

impl Guid {
    fn new(id: u32) -> Guid {
        Guid { id }
    }
}

struct GuidGenerator {
    used: HashSet<u32>,
}

impl GuidGenerator {
    fn new() -> GuidGenerator {
        GuidGenerator {
            used: HashSet::new(),
        }
    }

    fn generate(&mut self) -> Guid {
        let mut id = rand::random::<u32>();
        while self.used.contains(&id) {
            id = rand::random::<u32>();
        }
        self.used.insert(id);
        Guid::new(id)
    }
}

pub struct What {
    guid_generator: GuidGenerator,
    paths: HashMap<String, Guid>,
    cache: LfuCache<Guid, Vec<u8>>,
    location: Option<Location>,
}

impl What {
    pub fn new(max_size: usize, location: Option<Location>) -> What {
        What {
            guid_generator: GuidGenerator::new(),
            paths: HashMap::new(),
            cache: LfuCache::new(max_size),
            location,
        }
    }

    pub fn shrink_to_fit(&mut self, max_size: usize) {
        self.cache.shrink_to_fit(max_size);
    }

    pub fn load_file(&mut self, path: &str, priority: usize) -> Result<Vec<u8>, Error> {
        let key = *self
            .paths
            .entry(path.to_string())
            .or_insert_with(|| self.guid_generator.generate());

        if let Some(data) = self.cache.get(&key) {
            return Ok(data.clone());
        }

        let (data, other) = <What as Backend>::read_file(&self.location, path)?;
        self.cache.insert(&key, data.clone(), priority);

        if let Some(other) = other {
            for (key, data) in other {
                let guid = self.guid_generator.generate();
                self.paths.insert(key, guid);
                self.cache.insert(&guid, data, priority);
            }
        }
        Ok(data)
    }

    pub fn load_asset(&mut self, path: &str, priority: usize) -> Result<Asset, Error> {
        let data = self.load_file(path, priority)?;

        let mut size_buf = [0u8; 8];
        size_buf[..8].copy_from_slice(&data[..8]);
        let size = u64::from_be_bytes(size_buf);

        match serde_json::from_slice::<FurHeader>(&data) {
            Ok(meta) => match meta.ctype {
                HeaderType::Texture(texture_meta) => {
                    let texture =
                        data[(size as usize + 7 + texture_meta.offset as usize)..].to_vec();
                    Ok(Asset::Texture((
                        texture,
                        (texture_meta.width, texture_meta.height),
                    )))
                }
                HeaderType::Cubemap(cubemap_meta) => {
                    let mut textures = Vec::<(String, Vec<u8>, (u32, u32))>::new();
                    for (i, entry) in cubemap_meta.data.iter().enumerate() {
                        let default_end = HeaderEntry {
                            key: "".to_string(),
                            offset: u64::MAX,
                        };
                        let end = cubemap_meta.data.get(i + 1).unwrap_or(&default_end);
                        textures.push((
                            entry.key.clone(),
                            data[((size as usize + 7) + entry.offset as usize)
                                ..end.offset as usize]
                                .to_vec(),
                            (cubemap_meta.size, cubemap_meta.size),
                        ));
                    }
                    Ok(Asset::Cubemap(textures))
                }
                HeaderType::Gltf(gltf_meta) => {
                    let slice = &data[(size as usize + 7 + gltf_meta.offset as usize)..];
                    let base = match &self.location {
                        Some(Location::File(path)) => Some(path.clone()),
                        _ => None,
                    };

                    return gltf::import_slice(slice, base.as_deref(), |_, path| {
                        let res = self.load_file(path, priority);

                        match res {
                            Err(Error::Io(err)) => Err(gltf::Error::Io(err)),
                            _ => Ok(res.unwrap()),
                        }
                    })
                    .map_err(Error::GltfError)
                    .map(|(document, buffers, images)| Asset::Glb(document, buffers, images));
                }
            },
            Err(err) => Err(Error::JsonError(err)),
        }
    }
}
