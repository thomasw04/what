use backend::Backend;
use error::Error;
use image::EncodableLayout;
use lfu::LfuCache;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};
use utils::{Guid, GuidGenerator};

mod backend;
mod error;
mod lfu;
mod utils;

//--------------------------------------------------------------------------------------------------
// Internal Header Format
//--------------------------------------------------------------------------------------------------

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
struct HeaderEntry {
    key: String,
    offset: u64,
}

#[derive(Serialize, Deserialize)]
enum HeaderType {
    Texture(HeaderTexture),
    TextureArray(HeaderTextureArray),
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
struct HeaderTextureArray {
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

//--------------------------------------------------------------------------------------------------
// Public API
//--------------------------------------------------------------------------------------------------

const VERSION_MAJOR: u16 = 1;
const VERSION_MINOR: u16 = 0;

pub struct TextureData {
    pub width: u32,
    pub height: u32,
    pub format: Option<String>,
    pub data: Vec<u8>,
}

pub struct TextureArrayData {
    pub size: u32,
    pub format: Option<String>,
    pub keys: Vec<String>,
    pub data: Vec<Vec<u8>>,
}

pub enum Asset {
    Texture(TextureData),
    TextureArray(TextureArrayData),
    Glb(
        gltf::Document,
        Vec<gltf::buffer::Data>,
        Vec<gltf::image::Data>,
    ),
}

pub enum Location {
    File(PathBuf),
    Http(String),
}

pub struct What {
    guid_generator: GuidGenerator,
    paths: HashMap<String, Guid>,
    cache: LfuCache<Guid, Vec<u8>>,
    location: Option<Location>,
}

//--------------------------------------------------------------------------------------------------
// Implementations
//--------------------------------------------------------------------------------------------------

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

    pub fn load_file<S: AsRef<str>>(&mut self, path: S, priority: usize) -> Result<Vec<u8>, Error> {
        let path = path.as_ref();

        let key = if let Some(guid) = self.paths.get(path) {
            guid
        } else {
            self.paths
                .insert(path.to_string(), self.guid_generator.generate());

            if let Some(guid) = self.paths.get(path) {
                guid
            } else {
                return Err(Error::Unknown(
                    "I don't know what happened. Unable to generate Guid.".to_string(),
                ));
            }
        };

        if let Some(data) = self.cache.get(key) {
            return Ok(data.clone());
        }

        let (data, other) = <What as Backend>::read_file(&self.location, path)?;
        self.cache.insert(key, data.clone(), priority);

        if let Some(other) = other {
            for (key, data) in other {
                let guid = self.guid_generator.generate();
                self.paths.insert(key, guid);
                self.cache.insert(&guid, data, priority);
            }
        }
        Ok(data)
    }

    pub fn load_asset<S: AsRef<str>>(&mut self, path: S, priority: usize) -> Result<Asset, Error> {
        let path = path.as_ref();
        let data = self.load_file(path, priority)?;

        const HEADER_BEGIN: usize = 8;

        let mut size_buf = [0u8; HEADER_BEGIN];
        size_buf[..HEADER_BEGIN].copy_from_slice(&data[..HEADER_BEGIN]);
        let size = u64::from_be_bytes(size_buf);

        let header_end = HEADER_BEGIN + size as usize;

        match serde_json::from_slice::<FurHeader>(&data[HEADER_BEGIN..header_end]) {
            Ok(meta) => match meta.ctype {
                HeaderType::Texture(texture_meta) => {
                    let texture = data[(header_end + texture_meta.offset as usize)..].to_vec();
                    Ok(Asset::Texture(TextureData {
                        width: texture_meta.width,
                        height: texture_meta.height,
                        format: texture_meta.format,
                        data: texture,
                    }))
                }
                HeaderType::TextureArray(texarray_meta) => {
                    let mut textures = Vec::<Vec<u8>>::new();
                    let mut keys = Vec::<String>::new();
                    for (i, entry) in texarray_meta.data.iter().enumerate() {
                        let end_offset = if i + 1 >= texarray_meta.data.len() {
                            data.len()
                        } else {
                            header_end + texarray_meta.data[i + 1].offset as usize
                        };

                        textures
                            .push(data[(header_end + entry.offset as usize)..end_offset].to_vec());

                        keys.push(entry.key.clone());
                    }
                    Ok(Asset::TextureArray(TextureArrayData {
                        size: texarray_meta.size,
                        format: texarray_meta.format,
                        keys,
                        data: textures,
                    }))
                }
                HeaderType::Gltf(gltf_meta) => {
                    let slice = &data[(header_end + gltf_meta.offset as usize)..];
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

    fn write_texture<P: AsRef<Path>>(
        &self,
        output: P,
        texture: &TextureData,
        overwrite: bool,
    ) -> Result<(), String> {
        let output = output.as_ref();

        let output = if let Some(Location::File(path)) = &self.location {
            path.join(output)
        } else {
            output.to_path_buf()
        };

        if output.exists() {
            if overwrite {
                log::warn!("Overwrite flag set. Overwriting file {}", output.display());
            } else {
                return Err(format!("File {} already exists.", output.display()));
            }
        }

        let header = FurHeader {
            major: VERSION_MAJOR,
            minor: VERSION_MINOR,
            ctype: HeaderType::Texture(HeaderTexture {
                width: texture.width,
                height: texture.height,
                format: texture.format.as_ref().map(String::from),
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
                [
                    &size.to_be_bytes(),
                    content.as_bytes(),
                    texture.data.as_bytes(),
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

    fn write_texture_array<P: AsRef<Path>>(
        &self,
        output: P,
        textures: &TextureArrayData,
        overwrite: bool,
    ) -> Result<(), String> {
        let output = output.as_ref();

        let output = if let Some(Location::File(path)) = &self.location {
            path.join(output)
        } else {
            output.to_path_buf()
        };

        if output.exists() {
            if overwrite {
                log::warn!("Overwrite flag set. Overwriting file {}", output.display());
            } else {
                return Err(format!("File {} already exists.", output.display()));
            }
        }

        if textures.keys.len() != textures.data.len() {
            return Err(format!(
                "Texture array keys and data must have the same length. Keys: {} Textures: {}",
                textures.keys.len(),
                textures.data.len()
            ));
        }

        let mut entries = Vec::<HeaderEntry>::new();
        let mut offset = 0;

        for i in 0..textures.keys.len() {
            entries.push(HeaderEntry {
                key: textures.keys[i].to_string(),
                offset,
            });

            offset += textures.data[i].len() as u64;
        }

        let header = FurHeader {
            major: VERSION_MAJOR,
            minor: VERSION_MINOR,
            ctype: HeaderType::TextureArray(HeaderTextureArray {
                size: textures.size,
                format: textures.format.as_ref().map(String::from),
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

        //Gather all the data.
        let content = textures
            .data
            .iter()
            .flat_map(|a| a.iter())
            .cloned()
            .collect::<Vec<u8>>();

        if let Ok(header) = serde_json::to_string(&header) {
            let size: u64 = header.as_bytes().len() as u64;
            return std::fs::write(
                output,
                [&size.to_be_bytes(), header.as_bytes(), content.as_bytes()].concat(),
            )
            .map_err(|error| error.to_string());
        }

        Err(format!(
            "Could not serialize header of {}.",
            output.display()
        ))
    }

    pub fn convert_texture<P: AsRef<Path>>(
        &self,
        output: P,
        input: P,
        overwrite: bool,
    ) -> Result<(), String> {
        let output = output.as_ref();
        let input = input.as_ref();

        let input = if let Some(Location::File(path)) = &self.location {
            path.join(input)
        } else {
            input.to_path_buf()
        };

        let input = input.as_path();

        if input.exists() {
            if let Ok(dimension) = image::image_dimensions(input) {
                if let Ok(texture) = std::fs::read(input) {
                    let texture = TextureData {
                        width: dimension.0,
                        height: dimension.1,
                        format: input
                            .extension()
                            .map(|s| s.to_os_string().into_string().unwrap_or("".to_string())),
                        data: texture,
                    };

                    return self.write_texture(output, &texture, overwrite);
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

    pub fn convert_texture_array<P: AsRef<Path>, S: Into<String> + Clone>(
        &self,
        output: P,
        keys: Option<&[S]>,
        inputs: &[P],
        overwrite: bool,
    ) -> Result<(), String> {
        let output = output.as_ref();
        let inputs = inputs
            .iter()
            .map(|a| {
                if let Some(Location::File(path)) = &self.location {
                    path.join(a)
                } else {
                    a.as_ref().to_path_buf()
                }
            })
            .collect::<Vec<PathBuf>>();
        let keys = keys.map(|a| {
            a.iter()
                .map(|b| (*b).clone().into())
                .collect::<Vec<String>>()
        });

        let mut textures = Vec::<Vec<u8>>::with_capacity(inputs.len());

        let keys = keys.unwrap_or_else(|| {
            inputs
                .iter()
                .map(|a| {
                    a.file_stem()
                        .unwrap_or(std::ffi::OsStr::new("Unknown"))
                        .to_string_lossy()
                        .to_string()
                })
                .collect()
        });

        let mut size = 0;
        let mut format = None;

        for input in &inputs {
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
                            "All textures must have the same size. File: {}",
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
            } else {
                return Err(format!("File {} does not exist.", input.display()));
            }
        }

        let textures = TextureArrayData {
            size,
            format,
            keys,
            data: textures,
        };

        self.write_texture_array(output, &textures, overwrite)
    }

    pub fn convert_cubemap<P: AsRef<Path>>(
        &self,
        output: P,
        inputs: &[P],
        overwrite: bool,
    ) -> Result<(), String> {
        let keys = vec!["+x", "-x", "+y", "-y", "+z", "-z"];
        self.convert_texture_array(output, Some(&keys), inputs, overwrite)
    }
}
