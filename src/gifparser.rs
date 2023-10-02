/// A parser for GIF format, but for my use case just gets the starting indices and lengths of blocks
/// See https://www.w3.org/Graphics/GIF/spec-gif89a.txt

const SIZE_BYTES_MASK: u8 = 0x07;
const FIRST_BIT_MASK: u8 = 0x80;

#[derive(Debug)]
pub struct GifParseError {
    pub message: &'static str,
}
impl GifParseError {
    fn new(message: &'static str) -> Self {
        Self { message }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum GifBlockType {
    Header,
    LogicalScreenDescriptor,
    ColorTable,
    Extension(ExtensionType),
    ImageDescriptor,
    ImageData,
    Trailer,
}

#[derive(Debug, Clone, Copy)]
pub enum ExtensionType {
    GraphicControl,
    Comment,
    PlainText,
    Application,
}

#[derive(Debug, Clone, Copy)]
pub struct Block {
    pub index: u32,
    pub size: u32,
    pub block_type: GifBlockType,
}

#[derive(Debug, Default)]
pub struct GifParser<'a> {
    cur_index: u32,
    bytes: &'a [u8],
    blocks: Vec<Block>,
}

impl<'a> GifParser<'a> {
    pub fn new() -> Self {
        Self {
            ..Default::default()
        }
    }

    pub fn parse_gif_from_bytes(&mut self, bytes: &'a [u8]) -> Result<Vec<Block>, GifParseError> {
        self.bytes = bytes;
        self.blocks = vec![];
        self.cur_index = 0;

        // header and logical screen descriptor are always present
        self.parse_header()?;
        self.parse_logical_screen_descriptor()?;

        // check for global color table
        // on most modern systems casting to usize shouldn't lose data
        let logical_packed_byte = bytes[(self.blocks.last().unwrap().index + 4) as usize];
        if (logical_packed_byte & FIRST_BIT_MASK) != 0 {
            // global color table flag is set to true
            let global_color_table_size = logical_packed_byte & SIZE_BYTES_MASK; // number of entries

            // spec says 3 x 2^(Size of Global Color Table+1), since we are right shifting
            // value 2 already it already has implicit 2^1 so subtract 1 from that
            let global_color_table_bytes = 3 * (2 << (global_color_table_size));

            // parse global color table
            self.parse_color_table(global_color_table_bytes)?;
        }

        // loop and parse extension blocks and image blocks
        // only three characters may appear now, ! for graphic control extension blocks,
        // , for image descriptor and subsequent image data
        // and ; the trailing byte
        loop {
            let idx = self.cur_index as usize;
            if idx >= self.bytes.len() {
                return Err(GifParseError::new("Ran out of bytes while parsing GIF!"));
            }
            match bytes[idx] {
                b'!' => {
                    // 0x21
                    self.parse_extension()?;
                }
                b',' => {
                    // 0x2C
                    // parse descriptor
                    self.parse_image_descriptor()?;

                    // if the local color table bit was set, parse local color table
                    let image_descriptor_packed_byte =
                        self.bytes[(self.blocks.last().unwrap().index + 9) as usize];
                    if image_descriptor_packed_byte & FIRST_BIT_MASK != 0 {
                        // local color table exists
                        let local_color_table_size = image_descriptor_packed_byte & SIZE_BYTES_MASK; // number of entries

                        // see above when getting size of global color table
                        let local_color_table_bytes = 3 * (2 << (local_color_table_size));

                        self.parse_color_table(local_color_table_bytes)?;
                    }

                    // parse image data
                    self.parse_image_data()?;
                }
                b';' => {
                    // 0x3B
                    // we know this will definitely be the end because if we encounter
                    // blocks or graphic control extension blocks we will jump past them
                    self.parse_trailer()?;
                    break;
                }
                0x0 => {
                    self.cur_index += 1;
                    continue;
                } // people be adding weird padding sometimes?
                byte => {
                    println!(
                    "encountered GIF byte {:x?} that isn't a block starter! blocks so far {}\n Index {} surrounding bytes {:x?}",
                    byte, self.blocks.len(), idx, &self.bytes[(idx - 16)..(idx + 16)]
                    );
                    println!("{:#?}", self.blocks);
                    return Err(GifParseError {
                        message: "Invalid bytes in GIF",
                    });
                }
            };
        }

        Ok(self.blocks.clone())
    }

    /// Parse the GIF header from the current block
    fn parse_header(&mut self) -> Result<(), GifParseError> {
        const GIF_HEADER: [u8; 4] = *b"GIF8";
        const HEADER_LEN: u32 = 6;
        if !self.bytes.starts_with(&GIF_HEADER) {
            println!("{:x?}", &self.bytes[0..16]);
            return Err(GifParseError::new("Invalid GIF header bytes"));
        }
        self.blocks.push(Block {
            index: self.cur_index,
            size: HEADER_LEN,
            block_type: GifBlockType::Header,
        });
        self.cur_index += HEADER_LEN; // length of full gif header
        Ok(())
    }

    fn parse_logical_screen_descriptor(&mut self) -> Result<(), GifParseError> {
        // not going to validate anything here, just take the 7 bytes
        const LOGICAL_SCREEN_DESCRIPTOR_LEN: u32 = 7;
        self.blocks.push(Block {
            index: self.cur_index,
            size: LOGICAL_SCREEN_DESCRIPTOR_LEN,
            block_type: GifBlockType::LogicalScreenDescriptor,
        });
        self.cur_index += LOGICAL_SCREEN_DESCRIPTOR_LEN;
        Ok(())
    }

    /// Since color tables can be variable length specified by the preceding
    /// blocks, pass the length in here of how many bytes to include as the table
    fn parse_color_table(&mut self, length: u32) -> Result<(), GifParseError> {
        // again, not doing any validation
        self.blocks.push(Block {
            index: self.cur_index,
            size: length,
            block_type: GifBlockType::ColorTable,
        });
        self.cur_index += length;
        Ok(())
    }

    /// There are many different types of graphic control extension, which may have their
    /// data set in different ways.
    fn parse_extension(&mut self) -> Result<(), GifParseError> {
        if self.bytes[self.cur_index as usize] != 0x21 {
            println!(
                "{:x?}",
                &self.bytes[(self.cur_index as usize - 8)..(self.cur_index as usize + 8)]
            );
            return Err(GifParseError::new("Invalid extension block declaration"));
        }
        // Graphic control, plain text, and application extensions all have the format:
        //  extension introducer byte -> label byte -> block size
        // but plain text and application also have sub blocks of arbitrary length
        // Comment extensions only have sub blocks, extension introducer -> label byte -> sub blocks of comment data
        // they all have a zero byte block terminator that we should check though
        let idx = self.cur_index as usize;
        let (total_size, ext_type) = match self.bytes[idx + 1] {
            0xF9 => {
                // graphic control extension
                // only block size
                // let block_size = self.bytes[idx + 2];
                let block_size = 4; // TODO I changed this to hardcoded since it is/should be fixed, but not sure about implication

                // three extra bytes since introducer, label, and size aren't included in the size
                (3 + block_size as u32, ExtensionType::GraphicControl)
            }
            0xFE => {
                // comment data extension
                // only sub blocks
                let data_size = self.length_of_sub_blocks(idx + 2);
                // extension introducer byte and label byte
                (2 + data_size, ExtensionType::Comment)
            }
            0x01 => {
                // plain text and application extension have
                // block size for metadata and sub blocks with their own size
                let metadata_size = 12;
                let sub_blocks_start = idx + 3 + metadata_size as usize;
                let sub_blocks_size = self.length_of_sub_blocks(sub_blocks_start);
                (
                    3 + metadata_size as u32 + sub_blocks_size,
                    ExtensionType::PlainText,
                )
            }
            0xFF => {
                let metadata_size = 11;
                let sub_blocks_start = idx + 3 + metadata_size as usize;
                let sub_blocks_size = self.length_of_sub_blocks(sub_blocks_start);
                (
                    3 + metadata_size as u32 + sub_blocks_size,
                    ExtensionType::Application,
                )
            }
            _ => return Err(GifParseError::new("Invalid extension block type")),
        };
        self.blocks.push(Block {
            index: self.cur_index,
            size: total_size,
            block_type: GifBlockType::Extension(ext_type),
        });
        self.cur_index += total_size;

        if self.bytes[self.cur_index as usize] != 0x0 {
            Err(GifParseError::new("Invalid extension block terminator"))
        } else {
            Ok(())
        }
    }

    fn parse_image_descriptor(&mut self) -> Result<(), GifParseError> {
        const IMAGE_DESCRIPTOR_LEN: u32 = 10;
        if self.bytes[self.cur_index as usize] != 0x2C {
            return Err(GifParseError::new(
                "Invalid image descriptor block declaration",
            ));
        }
        self.blocks.push(Block {
            index: self.cur_index,
            size: IMAGE_DESCRIPTOR_LEN,
            block_type: GifBlockType::ImageDescriptor,
        });
        self.cur_index += IMAGE_DESCRIPTOR_LEN;
        Ok(())
    }

    fn parse_image_data(&mut self) -> Result<(), GifParseError> {
        const LZW_MIN_CODE_SIZE_LEN: u32 = 1;

        let data_len = self.length_of_sub_blocks((self.cur_index + LZW_MIN_CODE_SIZE_LEN) as usize);

        self.blocks.push(Block {
            index: self.cur_index,
            size: data_len + LZW_MIN_CODE_SIZE_LEN,
            block_type: GifBlockType::ImageData,
        });
        self.cur_index += data_len + LZW_MIN_CODE_SIZE_LEN;

        if self.bytes[self.cur_index as usize] != 0x0 {
            Err(GifParseError::new(
                "Invalid termination of image data blocks",
            ))
        } else {
            Ok(())
        }
    }

    /// Get the length of a list of sub-blocks starting at index, not including the terminating 0 byte
    /// This might yield ridiculous results if the index isn't actually the start of
    /// a list of blocks!
    fn length_of_sub_blocks(&mut self, index: usize) -> u32 {
        // data sub block format is just length, data bytes, null (0)
        // if the next byte after data is not null, then it is another sub-block in the list
        let mut num_bytes: u32 = 0;

        let mut block_len = self.bytes[index];
        // if index > 3075080 && block_len == 12 {
        //     println!("{}", block_len);
        //     println!("!! {:x?}", &self.bytes[index - 16..index + 16]);
        // }
        while block_len != 0 {
            num_bytes += 1 + block_len as u32; // add one to include the length byte as well
            block_len = self.bytes[index + (num_bytes as usize)];
            // if index > 3075080 {
            //     println!("{}", block_len);
            //     println!(
            //         "{:x?}",
            //         &self.bytes
            //             [index - 16 + (num_bytes as usize)..index + 16 + (num_bytes as usize)]
            //     );
            // }
        }

        num_bytes
    }

    fn parse_trailer(&mut self) -> Result<(), GifParseError> {
        if self.bytes[self.cur_index as usize] != 0x3B {
            // the GIF trailer byte
            Err(GifParseError::new("Invalid GIF trailer byte!"))
        } else {
            self.blocks.push(Block {
                index: self.cur_index,
                size: 1,
                block_type: GifBlockType::Trailer,
            });
            self.cur_index += 1;
            Ok(())
        }
    }
}
