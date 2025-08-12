use indicatif::{ProgressBar, ProgressStyle};
use serde::{Deserialize, Serialize};
use std::fs;
use std::fs::File;
use std::io::{self, Read, Seek, SeekFrom, Write}; //CP932 compatible
use std::process;

use crate::util;

#[derive(Serialize, Deserialize)]
struct Book {
    id: u16,
    name: String,
    pages: Vec<Page>,
}

#[derive(Serialize, Deserialize)]
struct Page {
    id: u8,
    image_x: Option<u16>,
    image_y: Option<u16>,
    image_id: Option<u16>,
    lines: Vec<Line>,
}

#[derive(Serialize, Deserialize)]
struct Line {
    id: u8,
    text: String,
}

pub fn convert_t_book_to_json_file(path: String) -> io::Result<()> {
    let table_data = parse_from_file(&path)?;
    let file_name = util::get_file_name(&path);

    if let Some(s) = file_name {
        let mut output = File::create(format!("{}.json", s))?;
        output.write_all(table_data.as_bytes())?;
        output.flush()?;

        Ok(())
    } else {
        println!("Not a valid file path.");
        process::exit(1);
    }
}

//TODO
pub fn convert_json_to_t_book(path: String) -> io::Result<()> {
    let json_data = fs::read_to_string(path)?;
    let books: Vec<Book> = serde_json::from_str(&json_data)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

    let dt_data = books_to_byte_data(books);

    let mut file = File::create("t_book._dt")?;
    file.write_all(&dt_data)?;

    Ok(())
}

fn parse_from_file(path: &str) -> io::Result<String> {
    let mut file = File::open(path)?;
    let mut address_bytes = [0u8; 2];

    //get the address of the first datum (the beginning of the file is a collection of addresses
    //that refer to each datum)
    file.read_exact(&mut address_bytes)?;
    let address_first_datum = u16::from_le_bytes(address_bytes);

    //set up the progress bar
    let style = ProgressStyle::default_bar()
        .template("[{bar:40.cyan/blue}] {prefix} {pos}/{len}")
        .unwrap()
        .progress_chars("█🮆🮅🮄▀🮃🮂▔ ");
    let bar = ProgressBar::new((address_first_datum as u64 - 2) / 2).with_style(style);

    let mut index_current_datum: u16 = 0;

    let mut books: Vec<Book> = Vec::new();

    let mut book_id = 0;

    //until the address of the first datum is reached, keep reading datums from pointers at the
    //beginning of the file
    //book loop
    while index_current_datum != address_first_datum {
        println!("Book ID: {}", book_id);
        //seek to the pointer area for the current book
        file.seek(SeekFrom::Start(index_current_datum as u64))?;
        //read the address of the name of the current datum from the pointer list
        file.read_exact(&mut address_bytes)?;
        let address_current_datum_name: u16 = u16::from_le_bytes(address_bytes);
        //read the address of the contents of the current datum from the pointer list
        file.read_exact(&mut address_bytes)?;
        let address_current_datum_content: u16 = u16::from_le_bytes(address_bytes);

        //move to the name address previously read and parse the name from the datum
        file.seek(SeekFrom::Start(address_current_datum_name as u64))?;
        let datum_title = util::parse_string(&mut file)?;

        //create a book
        let mut book: Book = Book {
            id: book_id,
            name: datum_title,
            pages: Vec::new(),
        };

        let mut book_content_done = false;

        //seek to the beginning of the book's contents
        file.seek(SeekFrom::Start(address_current_datum_content as u64))?;

        let mut page_id: u8 = 0;
        //loop that fills a page
        while !book_content_done {
            println!("Page ID: {}", page_id);

            //create a page
            let mut page: Page = Page {
                id: page_id,
                image_x: None,  //no change
                image_y: None,  //no change
                image_id: None, //no change
                lines: Vec::new(),
            };

            let mut page_content_done = false;

            let mut line_id: u8 = 0;
            //loop that fills a line
            while !page_content_done {
                println!("Line ID: {}", line_id);
                //single byte buffer to read page data byte by byte and separate it into lines
                let mut byte = [0u8; 1];

                //keep track of address of start of line for error reporting
                let address_line_start_bytes = file.stream_position()?.to_le_bytes();

                //create a line
                let mut line: Line = Line {
                    id: line_id,
                    text: String::new(),
                };

                let mut buffer = Vec::new();

                let mut line_content_done = false;

                //match the current byte against the data rules
                while !line_content_done {
                    //read a single byte into the byte buffer
                    file.read_exact(&mut byte)?;

                    match byte[0] {
                        //string end byte
                        0x00 => {
                            println!("String end byte.");
                            //attempt to decode the string and if successful, set the line text to
                            //the string
                            let decoded_line_string = util::decode_string(&buffer);
                            match decoded_line_string {
                                Ok(decoded) => line.text = decoded,
                                Err(e) => {
                                    println!(
                                        "Error reading string starting at {:02X} {:02X}, {}",
                                        address_line_start_bytes[0], address_line_start_bytes[1], e
                                    );
                                    process::exit(1);
                                }
                            }
                            line_content_done = true;
                            book_content_done = true;
                            page_content_done = true;
                        }
                        //new line byte
                        0x01 => {
                            println!("Line end byte.");
                            let decoded_line_string = util::decode_string(&buffer);
                            match decoded_line_string {
                                Ok(decoded) => line.text = decoded,
                                Err(e) => {
                                    println!(
                                        "Error reading string starting at {:02X} {:02X}; {}",
                                        address_line_start_bytes[0], address_line_start_bytes[1], e
                                    );
                                    process::exit(1);
                                }
                            }
                            line_content_done = true;
                        }
                        //wait for input byte, treat as new page
                        0x02 => {
                            println!("Page end (wait for input) byte.");
                            let decoded_line_string = util::decode_string(&buffer);
                            match decoded_line_string {
                                Ok(decoded) => line.text = decoded,
                                Err(e) => {
                                    println!(
                                        "Error reading string starting at {:02X} {:02X}; {}",
                                        address_line_start_bytes[0], address_line_start_bytes[1], e
                                    );
                                    process::exit(1);
                                }
                            }
                            line_content_done = true;
                            page_content_done = true;
                        }
                        //actual new page byte, ignore as we treat 0x02 as the start of a new line for
                        //data handling purposes
                        0x03 => {}
                        //color change
                        0x07 => {
                            println!("Color change byte.");
                            let mut next_byte = [0u8; 1];
                            file.read_exact(&mut next_byte)?;
                            //line.color = next_byte[0];

                            for byte in util::encode_string("<C:") {
                                buffer.push(byte);
                            }
                            for byte in next_byte[0].to_string().as_bytes() {
                                buffer.push(*byte);
                            }
                            buffer.push(util::encode_string(">")[0]);
                        }
                        //formatting change
                        0x23 => {
                            println!("Formatting byte.");
                            //read the following bytes until x, y, F, or S
                            let mut next_byte = [0u8; 1];
                            file.read_exact(&mut next_byte)?;
                            if next_byte[0] != 0x46 {
                                let address_image_details = file.stream_position()? as u16 - 1;
                                let address_image_details_bytes =
                                    address_image_details.to_le_bytes();
                                let mut image_value_bytes: Vec<u8> = Vec::new();

                                //push the already read next_byte to the value vector
                                image_value_bytes.push(next_byte[0]);

                                let mut value_done = false;
                                while !value_done {
                                    file.read_exact(&mut next_byte)?;
                                    match next_byte[0] {
                                        //F; face change
                                        0x46 => {
                                            println!("Image change byte.");
                                            //image values are numbers stored as strings, so we need to decode to a string and then parse as a u16
                                            let image_id = util::decode_string(&image_value_bytes)?;
                                            let image_id_int = image_id.parse::<u16>();
                                            match image_id_int {
                                                Ok(num) => page.image_id = Some(num),
                                                Err(e) => {
                                                    eprintln!(
                                                        "Failed to parse image id at {:02X} {:02X}; {}",
                                                        address_image_details_bytes[0],
                                                        address_image_details_bytes[1],
                                                        e
                                                    );
                                                    process::exit(1);
                                                }
                                            }
                                            value_done = true;
                                        }
                                        //S; size change
                                        0x53 => {
                                            for byte in util::encode_string("<S:") {
                                                buffer.push(byte);
                                            }
                                            for byte in &image_value_bytes {
                                                buffer.push(*byte);
                                            }
                                            //let size_value =
                                            //    util::decode_string(&image_value_bytes)?;
                                            buffer.push(util::encode_string(">")[0]);

                                            value_done = true;
                                        }
                                        //x; x position change
                                        0x78 => {
                                            println!("Image x byte.");
                                            let image_x = util::decode_string(&image_value_bytes)?;
                                            let image_x_int = image_x.parse::<u16>();
                                            match image_x_int {
                                                Ok(num) => page.image_x = Some(num),
                                                Err(e) => {
                                                    eprintln!(
                                                        "Failed to parse image x at {:02X} {:02X}; {}",
                                                        address_image_details_bytes[0],
                                                        address_image_details_bytes[1],
                                                        e
                                                    );
                                                    process::exit(1);
                                                }
                                            }
                                            value_done = true;
                                        }
                                        //y; y position change
                                        0x79 => {
                                            println!("Image y byte.");
                                            let image_y = util::decode_string(&image_value_bytes)?;
                                            let image_y_int = image_y.parse::<u16>();
                                            match image_y_int {
                                                Ok(num) => page.image_y = Some(num),
                                                Err(e) => {
                                                    eprintln!(
                                                        "Failed to parse image y at {:02X} {:02X}; {}",
                                                        address_image_details_bytes[0],
                                                        address_image_details_bytes[1],
                                                        e
                                                    );
                                                    process::exit(1);
                                                }
                                            }
                                            value_done = true;
                                        }
                                        //anything else, push to image value bytes
                                        _ => {
                                            image_value_bytes.push(next_byte[0]);
                                        }
                                    }
                                }
                            } else {
                                page.image_id = None; //using this value to represent face reset
                            }
                        }
                        //anything else, treat as a character and add to buffer
                        _ => {
                            buffer.push(byte[0]);
                        }
                    }
                } //NOTE: end byte read loop

                println!("Line text: {}", line.text);

                //push the line to the page
                page.lines.push(line);

                line_id += 1;
            } //NOTE: end line contents loop

            book.pages.push(page);

            //line gets added to page in conditionals above
            page_id += 1;
        } //NOTE: end page contents loop

        //TODO: add the book data to the book list
        books.push(book);

        //increment things as necessary
        index_current_datum += 4;
        bar.inc(1);
        book_id += 1;
    }

    //serialize the finalized item list as json data
    let json = serde_json::to_string_pretty(&books).unwrap();

    Ok(json)
}

//TODO
fn books_to_byte_data(books: Vec<Book>) -> Vec<u8> {
    let mut bytes = Vec::new();
    let mut item_addresses = Vec::new();

    let item_count = books.len();
    let item_header_length = 4 * item_count;

    //reserve the item address space
    bytes.resize(item_header_length, 0);

    //set up the progress bar
    let style = ProgressStyle::default_bar()
        .template("[{bar:40.cyan/blue}] {prefix} {pos}/{len}")
        .unwrap()
        .progress_chars("█🮆🮅🮄▀🮃🮂▔ ");
    let bar = ProgressBar::new(item_count as u64).with_style(style);

    for book in books {
        //record the starting address for this item's data
        let address = bytes.len() as u16;
        item_addresses.push(address);

        //reserve the name and desc address space for this item
        let item_data_address_pos = bytes.len();
        bytes.resize(item_data_address_pos + 4, 0);

        //encode the name and description in CP932
        let name_bytes = util::encode_string(&book.name);

        //write the name to the buffer followed by a null byte
        let name_address = util::write_bytes_to_buffer(&mut bytes, name_bytes);

        //fill the address space for this item
        bytes[item_data_address_pos..item_data_address_pos + 2]
            .copy_from_slice(&name_address.to_le_bytes());

        //increment the progress bar
        bar.inc(1);
    }

    //fill the address space for the item list
    for (i, &address) in item_addresses.iter().enumerate() {
        let start = i * 2;
        bytes[start..start + 2].copy_from_slice(&address.to_le_bytes());
    }

    bytes
}
