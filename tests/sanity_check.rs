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

    What::convert_texture("tests/assets/error_gen.fur", "tests/assets/error.png", true).unwrap();

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
