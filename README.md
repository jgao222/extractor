# Extractor
## Motivation
I came across a Qt executable with images and other resources compiled directly into the `.exe`.
I wanted to know if there was some way to get the image files back out, but there didn't seem
to be one that worked for me, so I tried to see if there were any existing PNG headers in the
executable. There were, meaning uncompressed and unencrypted image data existed in the file,
so by leveraging the [image](https://github.com/image-rs/image/) crate, this program can to extract those images.

## Usage
This program extracts *uncompressed unencrypted* images from arbitrary files by looking for
their header bytes and using the [image](https://github.com/image-rs/image/) crate to read and save them to their own files. It should support the file types that the aforementioned crate supports.

Clone the project repository and run:
```
cargo run --release -- [FILE] [TARGET_DIRECTORY]
```