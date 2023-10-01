use extractor::{extract_images, Error};
use std::env::args;

fn main() -> Result<(), Error> {
    // const PNG_HEADER: [u8; 8] = [137, 80, 78, 71, 13, 10, 26, 10];
    // const JPEG_HEADER: [u8; 2] = [0xFF, 0xD8];
    // this technically isn't completely right since there is a more specific version as well on the end
    // const GIF_HEADER: [u8; 4] = *b"GIF8";

    let args: Vec<String> = args().collect();

    assert!(args.len() == 3);
    let filename = &args[1];
    let mut output_folder = args[2].clone();
    if !output_folder.ends_with('/') {
        output_folder.push('/');
    }

    extract_images(filename, &output_folder)
}
