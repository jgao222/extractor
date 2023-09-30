use std::env::args;
use std::fs;

fn main() {
    const PNG_HEADER: [u8; 8] = [137, 80, 78, 71, 13, 10, 26, 10];
    const JPEG_HEADER: [u8; 2] = [0xFF, 0xD8];

    let args: Vec<String> = args().collect();

    assert!(args.len() == 3);
    let filename = &args[1];
    let mut output_folder = args[2].clone();
    if !output_folder.ends_with('/') {
        output_folder.push('/');
    }

    // try to open file and read bytes from it
    let bytes = fs::read(filename).expect("file path argument should be valid");

    let mut png_header_offsets = vec![];

    let mut jpeg_header_offsets = vec![];

    // match bytes against PNG header and see how many times it shows up
    for i in 0..(bytes.len() - PNG_HEADER.len()) {
        if bytes[i..i + PNG_HEADER.len()] == PNG_HEADER {
            png_header_offsets.push(i);
        }
        // TODO technically the byte sequence of each file type's header could
        // appear as legitimate data in another file, so to do this properly
        // we would need to keep track of whether we are already in a file or
        // not
        if bytes[i..i + JPEG_HEADER.len()] == JPEG_HEADER {
            jpeg_header_offsets.push(i);
        }
    }

    println!("Saw the PNG header {} times", png_header_offsets.len());
    println!("Saw the JPEG header {} times", jpeg_header_offsets.len());

    println!("Attempting to read out and save all pngs");
    for (i, offset) in png_header_offsets.iter().enumerate() {
        let img = image::load_from_memory(&bytes[*offset..]) // this gives all the way to end of file, which isn't great
            .expect("should find a correct image from the header");

        img.save(format!("{}/{}.png", output_folder, i))
            .expect("should save image successfully");
    }

    println!("Attempting to read out and save all jpegs");
    let mut non_jpegs = 0;
    for (i, offset) in jpeg_header_offsets.iter().enumerate() {
        if let Ok(img) = image::load_from_memory(&bytes[*offset..]) {
            img.save(format!("{}/{}.jpg", output_folder, i))
                .expect("should save image successfully");
        } else {
            non_jpegs += 1;
        }
    }
    println!(
        "Only {} were proper jpegs!",
        jpeg_header_offsets.len() - non_jpegs
    );
}
