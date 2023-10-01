use std::env::args;
use std::fs;
use std::io::Write;

// use image::codecs::gif::{GifDecoder, GifEncoder};
use image::{guess_format, ImageFormat};

fn main() {
    // const PNG_HEADER: [u8; 8] = [137, 80, 78, 71, 13, 10, 26, 10];
    // const JPEG_HEADER: [u8; 2] = [0xFF, 0xD8];
    // this technically isn't completely right since there is a more specific version as well on the end
    const GIF_HEADER: [u8; 4] = *b"GIF8";

    let args: Vec<String> = args().collect();

    assert!(args.len() == 3);
    let filename = &args[1];
    let mut output_folder = args[2].clone();
    if !output_folder.ends_with('/') {
        output_folder.push('/');
    }

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
    let mut num_images = 0;
    let mut i = 0;
    while i < bytes.len() {
        if let Ok(format) = image::guess_format(&bytes[i..]) {
            if format == ImageFormat::Gif {
                // gifs don't appear to be properly handled (image crate only takes the first frame)
                // so instead if we see the gif start header we will manually search for the
                // end bytes ourselves and write out the bytes and frames manually
                // TODO This didn't work because there were 0x3B's and 0x003B's showing
                //      up before EOF and ignoring them would have been missing out on much of the data
                //      how do modern decoders handle seeing the trailer byte? shouldn't the trailer byte
                //      signal the definite end of the GIF file???
                //      Turns out the GIF Format goes like this
                //      If you see the GIF header, then you are guaranteed some other metadata like
                //       color tables, logical screen descriptor. Then there is optionally some extensions blocks. Then
                //       there are image data blocks, which importantly have a byte field specifying how
                //       many bytes are in the block. So even if the trailing byte is seen, if it is within
                //       a block we know it is data!
                //       Unfortunately, this makes it hard to know where the GIF ends without knowing
                //       where the last block is, meaning we need to parse block data out of the GIF,
                //       so we need to know about the GIF structure and write a parser/state machine for it...
                // assert!(bytes[i..i + GIF_HEADER.len()] == GIF_HEADER);
                // let start_idx = i;
                // i += GIF_HEADER.len(); // no need to check the gif header/magic bytes for the trailer
                // while bytes[i] != 0x3B || bytes[i - 1] != 0x00 {
                //     i += 1;
                // }
                // fs::write(
                //     format!("{}/{}.gif", output_folder, num_images),
                //     &bytes[start_idx..i + 1],
                // )
                // .expect("manually outputting gif bytes should write without error");
                // println!("GIF ended after {} bytes", i - start_idx);
                // num_images += 1;

                // This approach with re-encoding and de-encoding doesn't preserve
                // stuff like if the gif repeats, and also balloons the filesize
                // for some reason
                // let manual_output_file =
                //     fs::File::create(format!("{}/{}.gif", output_folder, num_images))
                //         .expect("should be able to create a file");

                // let decoder = GifDecoder::new(&bytes[i..]).expect("gif was properly formatted");
                // let mut encoder = GifEncoder::new(manual_output_file);

                // encoder
                //     .try_encode_frames(decoder.into_frames())
                //     .expect("directly encoding decoded gif frames should be fine");

                // num_images += 1;

                // instead, let's just try giving the gif every byte until another
                // image file's magic numbers are found or EOF?

                // let's try jumping over blocks, don't read the data inside
                // ugh, we also need to jump over extension blocks, optional global color table,
                // and optional local color tables...
                // and all that involves linked lists of sub-blocks...
                // let start_idx = i;
                // while bytes[i] != 0x3B {
                //     if bytes[i] == 0x2C {
                //         // 0x2C is `,` comma, the block starting delimiter
                //         // the length of the block is 11 bytes away from the block start
                //         i += bytes[i + 11] as usize; // u8 should fit in usize...
                //     } else {
                //         i += 1;
                //     }
                // }
                // Let's just give whole block until next file header then
                // will lead to bloated image files, but all the original bytes will
                // be there...
                // that doesn't work if some block just happens to have data that looks like a header.

                let start_idx = i;
                i += GIF_HEADER.len(); // no need to check the gif header/magic bytes for the trailer
                while guess_format(&bytes[i..]).is_err() {
                    i += 1;
                }

                let mut manual_output_file =
                    fs::File::create(format!("{}/{}.gif", output_folder, num_images))
                        .expect("should be able to create a file");

                manual_output_file
                    .write_all(&bytes[start_idx..i + 1]) // +1 to include the trailer byte
                    .expect("should be able to write segement to disk");

                println!("GIF ended after {} bytes", i - start_idx);
                num_images += 1;
            } else if let Ok(img) = image::load_from_memory_with_format(&bytes[i..], format) {
                img.save_with_format(
                    format!(
                        "{}/{}.{}",
                        output_folder,
                        num_images,
                        format.extensions_str()[0]
                    ),
                    format,
                )
                .expect("image should save properly");
                num_images += 1;
            }
        }
        i += 1;
    }
    println!("Extracted {} images", num_images);

    // let mut png_header_offsets = vec![];

    // let mut jpeg_header_offsets = vec![];

    // // match bytes against PNG header and see how many times it shows up
    // for i in 0..(bytes.len() - PNG_HEADER.len()) {
    //     if bytes[i..i + PNG_HEADER.len()] == PNG_HEADER {
    //         png_header_offsets.push(i);
    //     }
    //     // TODO technically the byte sequence of each file type's header could
    //     // appear as legitimate data in another file, so to do this properly
    //     // we would need to keep track of whether we are already in a file or
    //     // not
    //     if bytes[i..i + JPEG_HEADER.len()] == JPEG_HEADER {
    //         jpeg_header_offsets.push(i);
    //     }
    // }

    // println!("Saw the PNG header {} times", png_header_offsets.len());
    // println!("Saw the JPEG header {} times", jpeg_header_offsets.len());

    // println!("Attempting to read out and save all pngs");
    // for (i, offset) in png_header_offsets.iter().enumerate() {
    //     let img = image::load_from_memory(&bytes[*offset..]) // this gives all the way to end of file, which isn't great
    //         .expect("should find a correct image from the header");

    //     img.save(format!("{}/{}.png", output_folder, i))
    //         .expect("should save image successfully");
    // }

    // println!("Attempting to read out and save all jpegs");
    // let mut non_jpegs = 0;
    // for (i, offset) in jpeg_header_offsets.iter().enumerate() {
    //     if let Ok(img) = image::load_from_memory(&bytes[*offset..]) {
    //         img.save(format!("{}/{}.jpg", output_folder, i))
    //             .expect("should save image successfully");
    //     } else {
    //         non_jpegs += 1;
    //     }
    // }
    // println!(
    //     "Only {} were proper jpegs!",
    //     jpeg_header_offsets.len() - non_jpegs
    // );
}
