use extractor::{extract_images, Error};
use std::env::args;

fn main() -> Result<(), Error> {
    let args: Vec<String> = args().collect();

    if args.len() != 3 {
        eprintln!("Error: Bad Args\nUsage: ./extractor.exe [FILE] [TARGET_DIRECTORY]");
    }

    let filename = &args[1];
    let mut output_folder = args[2].clone();
    if !output_folder.ends_with('/') {
        output_folder.push('/');
    }

    extract_images(filename, &output_folder)
}
