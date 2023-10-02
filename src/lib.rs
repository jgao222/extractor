use image::ImageFormat;
use std::{fs, io::Write};

use crate::gifparser::GifParser;

pub mod gifparser;

const OUTPUT_ENABLED: bool = true;

#[derive(Debug)]
pub enum Error {
    GifError(&'static str),
}

pub fn extract_images(filename: &String, output_folder: &String) -> Result<(), Error> {
    // try to open file and read bytes from it
    let bytes = fs::read(filename).expect("file path argument should be valid");

    // at every byte, ask the image library to try guessing if there is an image
    // here (based on magic bytes)
    // if so, remember what format that is, ask it to make it into an image, then
    // save it in that format
    // TODO isn't this unnecessary work since the bytes are already in the proper format?
    //      it would be more efficient to just find where the image begins and ends
    //      and splat those direcly to disk without needing to hold the image in memory first...
    // but it should still work? and it should get all image types supported by the
    // image crate?
    // So this is what we're doing manually for GIFs, but not the other images. it's fine, but doing
    // all image types would require parsing all image types. Image library only provides decoder/encoder
    // for files, not lower level parsers for arbitrary raw bytes
    let mut num_images = 0;
    let mut i = 0;
    while i < bytes.len() {
        if let Ok(format) = image::guess_format(&bytes[i..]) {
            if format == ImageFormat::Gif {
                // gifs don't appear to be properly handled (image crate only takes the first frame)
                // so instead if we see the gif start header we will manually search for the
                // end bytes ourselves and write out the bytes and frames manually

                // image lib is "guessing", I think it will accept some padding before the GIF
                // header appears, so it isn't at the front of the buffer at this index?
                const GIF_HEADER: [u8; 4] = *b"GIF8";
                if !&bytes[i..].starts_with(&GIF_HEADER) {
                    continue; // so ignore until we actually get to the header
                }

                if let Err(e) =
                    find_and_write_gif(&bytes, i, format!("{}{}.gif", output_folder, num_images))
                {
                    println!("Encountered error at image {}: {:?}", num_images, e)
                }
                // }
                num_images += 1;
            } else if let Ok(img) = image::load_from_memory_with_format(&bytes[i..], format) {
                if OUTPUT_ENABLED {
                    if let Err(e) = img.save_with_format(
                        format!(
                            "{}/{}.{}",
                            output_folder,
                            num_images,
                            format.extensions_str()[0]
                        ),
                        format,
                    ) {
                        println!("Error when saving image: {}", e);
                    }
                }
                num_images += 1;
            }
        }
        i += 1;
    }
    println!("Extracted {} images", num_images);

    Ok(())
}

fn find_and_write_gif(bytes: &[u8], index: usize, filename: String) -> Result<(), Error> {
    let start_idx = index;
    let mut parser = GifParser::new();
    let blocks = parser
        .parse_gif_from_bytes(&bytes[start_idx..])
        .map_err(|err| Error::GifError(err.message))?;
    let end_idx = start_idx + (blocks.last().unwrap().index + blocks.last().unwrap().size) as usize;

    if OUTPUT_ENABLED {
        let mut manual_output_file =
            fs::File::create(filename).expect("should be able to create a file");

        manual_output_file
            .write_all(&bytes[start_idx..end_idx]) // +1 to include the trailer byte
            .expect("should be able to write segement to disk");
    }

    Ok(())
}
