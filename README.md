# Rusty-Bear-Engine Asset Utility (What)
This utility program, known as "What," is an essential tool for working with the Rusty-Bear-Engine. It helps identify assets and converts them into a format that the engine can easily read.

## Features (Current)
 - __Texture Conversion__: Convert textures and texture arrays to the .fur file format, which  is the main asset format for the Rusty-Bear-Engine. This format stores everything neatly in a single file. Currently, it stores textures as PNG binary blobs within the .fur file.
 - __Texture Loading__: Load textures and texture arrays from a .fur file.
 - __Command-Line Interface (CLI)__: A user-friendly CLI for straightforward asset conversion.

## Usage
### CLI Commands
You can interact with the Rusty-Bear-Engine Asset Utility via the command-line interface (CLI). Here are some examples of how to use it:

- __Converting Assets:__
To convert assets into the __.fur__ format, use the following command:
```sh
$ ./what convert [INPUT file paths]... -o output.fur
```
- Options:
    - __`[INPUT file paths]`__: Provide the file paths of the assets you want to convert. You can specify multiple input files.
    - __`-o output.fur`__: Specify the name of the output .fur file.
    - __`--overwrite`__: Use this option if you want to overwrite an existing output file.

__Note__: If you don't specify an output file name using -o, the utility will use the input file's name with a .fur extension. However, please be aware that this won't work if you specified multiple input files (e.g. for cubemaps).

### Library

```Rust
pub fn read_texture(path: &Path) -> Result<(Vec<u8>, (u32, u32)), String>
``` 
returns the texture data of a .fur file together with width and height.

```Rust
pub fn read_cubemap(path: &Path) -> Result<(Vec<(String, Vec<u8>)>, u32), String>
``` 
returns a cubemap (6 textures) of a .fur file together with the dimension of each of them. (Cubemap textures have to be quadratic.)

```Rust
pub fn write_texture(
    output: &Path,
    texture: Vec<u8>,
    width: u32,
    format: Option<String>,
    height: u32,
    overwrite: bool,
) -> Result<(), String>
```
writes a texture to a .fur file.

```Rust
pub fn write_cubemap(
    output: &Path,
    textures: &[Vec<u8>],
    size: u32,
    format: Option<String>,
    overwrite: bool,
) -> Result<(), String>
```
writes a cubemap to a .fur file.
