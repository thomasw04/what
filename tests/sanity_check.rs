use std::path::PathBuf;

use what::{Asset, What};

#[test]
fn test_read_file_no_base() {
    let mut what = What::new(1e8 as usize, None);

    let actual = what.load_file("tests/assets/error.png", 0).unwrap();
    let expected = include_bytes!("assets/error.png");
    assert_eq!(expected, actual.as_slice());
}

#[test]
fn test_read_file_with_base() {
    let mut what = What::new(
        1e8 as usize,
        Some(what::Location::File(PathBuf::from("tests/assets"))),
    );

    let actual = what.load_file("error.png", 0).unwrap();
    let expected = include_bytes!("assets/error.png");
    assert_eq!(expected, actual.as_slice());
}

#[test]
fn test_load_asset() {
    let mut what = What::new(
        1e8 as usize,
        Some(what::Location::File(PathBuf::from("tests/assets"))),
    );

    let actual = what.load_asset("error.fur", 0).unwrap();
    let expected = include_bytes!("assets/error.png");

    if let Asset::Texture(data) = actual {
        assert_eq!(expected, data.data.as_slice());
        assert_eq!(data.width, 512);
        assert_eq!(data.height, 512);
    } else {
        panic!("Expected texture.");
    }
}

#[test]
fn test_convert_asset() {
    let mut what = What::new(
        1e8 as usize,
        Some(what::Location::File(PathBuf::from("tests/assets"))),
    );

    what.convert_texture("error_gen.fur", "error.png", true)
        .unwrap();

    let actual = what.load_asset("error_gen.fur", 0).unwrap();
    let expected = include_bytes!("assets/error.png");

    if let Asset::Texture(data) = actual {
        assert_eq!(expected, data.data.as_slice());
        assert_eq!(data.width, 512);
        assert_eq!(data.height, 512);
    } else {
        panic!("Expected texture.");
    }
}

#[test]
fn test_convert_cubemap() {
    let mut what = What::new(
        1e8 as usize,
        Some(what::Location::File(PathBuf::from("tests/assets"))),
    );

    what.convert_cubemap(
        "cubemap_gen.fur",
        &[
            "error.png",
            "error.png",
            "error.png",
            "error.png",
            "error.png",
            "error.png",
        ],
        true,
    )
    .unwrap();

    let actual = what.load_asset("cubemap_gen.fur", 0).unwrap();

    let expected = include_bytes!("assets/error.png");

    if let Asset::TextureArray(data) = actual {
        assert_eq!(data.size, 512);
        assert_eq!(data.data.len(), 6);
        for face in data.data {
            assert_eq!(face.len(), expected.len());
            assert_eq!(face.as_slice(), expected);
        }
    } else {
        panic!("Expected cubemap.");
    }
}

#[test]
fn test_convert_shader() {
    let mut what = What::new(
        1e8 as usize,
        Some(what::Location::File(PathBuf::from("tests/assets"))),
    );

    what.convert_shader("shader_gen.fur", "shader.wgsl", true)
        .unwrap();

    let actual = what.load_asset("shader_gen.fur", 0).unwrap();

    let actual = if let Asset::Shader(data) = actual {
        data
    } else {
        panic!("Expected shader.");
    };

    let mut info = naga::valid::Validator::new(
        naga::valid::ValidationFlags::all(),
        naga::valid::Capabilities::all(),
    );

    let module = naga::front::spv::Frontend::new(
        actual.data.into_iter(),
        &naga::front::spv::Options::default(),
    )
    .parse()
    .unwrap();

    info.validate(&module).unwrap();
}
