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

pub fn convert_json_to_t_book(path: String) -> io::Result<()> {
    let json_data = fs::read_to_string(&path)?;
    let file_name = util::get_file_name(&path);
    let books: Vec<Book> = serde_json::from_str(&json_data)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

    let dt_data = books_to_byte_data(books);

    if let Some(s) = file_name {
        let mut output = File::create(format!("{}._dt", s))?;
        output.write_all(&dt_data)?;
        output.flush()?;

        Ok(())
    } else {
        println!("Not a valid file path.");
        process::exit(1);
    }
}

//NOTE: code for converting from _dt to json-------------------------------------------------------
//enum for the status of the current book
enum ReadStatus {
    Continue,
    EndPage,
    EndBook,
}

//read one text line from the file and return it with a ReadStatus enum to denote whether the page
//or book has ended
fn read_line(file: &mut File, page: &mut Page, line_id: u8) -> io::Result<(Line, ReadStatus)> {
    let mut line = Line {
        id: line_id,
        text: String::new(),
    };
    let mut buffer = Vec::new();
    let address_bytes: [u8; 2] = (file.stream_position()? as u16).to_le_bytes();

    loop {
        let mut byte = [0u8; 1];
        //end of file midline is treated as EndBook (end of current book as well as end of last
        //book in file)
        if file.read_exact(&mut byte).is_err() {
            //decode the byte vector into a string and add it to the line
            line.text = util::decode_string(&buffer)?;
            return Ok((line, ReadStatus::EndBook));
        }

        //match the byte to known byte codes handled by the sky games or push it to the byte vector
        match byte[0] {
            //end of book
            0x00 => {
                line.text = decode_or_exit(&buffer, address_bytes);
                return Ok((line, ReadStatus::EndBook));
            }
            //end of line
            0x01 => {
                line.text = decode_or_exit(&buffer, address_bytes);
                return Ok((line, ReadStatus::Continue));
            }
            //end of page (actually wait for user input byte code but for all intents and purposes
            //it signals the end of the page and I treat it as such for reproducable data storage)
            0x02 => {
                line.text = decode_or_exit(&buffer, address_bytes);
                return Ok((line, ReadStatus::EndPage));
            }
            //actual end of page byte code but we don't use it
            0x03 => {}
            //color change
            0x07 => {
                let mut color_byte = [0u8; 1];
                file.read_exact(&mut color_byte)?;
                buffer.extend(util::encode_string(&format!("<C:{}>", color_byte[0])));
            }
            //formatting change (face position/id or text size change)
            0x23 => {
                handle_formatting(file, page, &mut buffer, address_bytes)?;
            }
            //anything else is just treated as line text
            _ => buffer.push(byte[0]),
        }
    }
}

//decode a byte vector into a string or print an error and exit
fn decode_or_exit(bytes: &Vec<u8>, addr: [u8; 2]) -> String {
    util::decode_string(bytes).unwrap_or_else(|e| {
        eprintln!(
            "Error reading string starting at {:02X} {:02X}; {}",
            addr[0], addr[1], e
        );
        process::exit(1);
    })
}

//match the formatting type and fill out the relevant page data or add it to the buffer
fn handle_formatting(
    file: &mut File,
    page: &mut Page,
    buffer: &mut Vec<u8>,
    address_bytes: [u8; 2],
) -> io::Result<()> {
    let mut b = [0u8; 1];
    file.read_exact(&mut b)?;

    //F, clear face/image
    if b[0] == 0x46 {
        page.image_id = Some(0xFFF); //represent a face clear with 0xFFF, no change is None/null
        return Ok(());
    }

    let mut value_bytes = vec![b[0]];
    loop {
        file.read_exact(&mut b)?;
        match b[0] {
            //F, image id
            0x46 => {
                let id = parse_u16_string(&value_bytes, address_bytes)?;
                page.image_id = Some(id);
                break;
            }
            //S, text size
            0x53 => {
                buffer.extend(util::encode_string("<S:"));
                buffer.extend(&value_bytes);
                buffer.push(util::encode_string(">")[0]);
                break;
            }
            //x position of face/image
            0x78 => {
                let x = parse_u16_string(&value_bytes, address_bytes)?;
                page.image_x = Some(x);
                break;
            }
            //y position of face/image
            0x79 => {
                let y = parse_u16_string(&value_bytes, address_bytes)?;
                page.image_y = Some(y);
                break;
            }
            //part of the value for the formatting data
            _ => value_bytes.push(b[0]),
        }
    }
    Ok(())
}

//parse a byte array as a string and convert to a u16 value (used for image x and y positions and
//face ids)
fn parse_u16_string(bytes: &[u8], address: [u8; 2]) -> io::Result<u16> {
    let decoded = util::decode_string(&bytes.to_vec()).map_err(|e| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "Failed to decode at {:02X} {:02X}: {}",
                address[0], address[1], e
            ),
        )
    })?;
    decoded.parse::<u16>().map_err(|e| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "Failed to parse u16 at {:02X} {:02X}: {}",
                address[0], address[1], e
            ),
        )
    })
}

//read all of the lines for one page out and add them to a page, return the page and a bool
//determining whether or not the book is done
fn read_page(file: &mut File, page_id: u8) -> io::Result<(Page, bool)> {
    let mut page = Page {
        id: page_id,
        image_x: None,
        image_y: None,
        image_id: None,
        lines: Vec::new(),
    };
    let mut line_id = 0;

    loop {
        //read a line in
        let (line, status) = read_line(file, &mut page, line_id)?;
        //add it as a page in the book
        page.lines.push(line);
        line_id += 1;

        //check the status returned by read_line to see if it was end of page or end of file
        match status {
            ReadStatus::Continue => {}
            ReadStatus::EndPage => return Ok((page, false)),
            ReadStatus::EndBook => return Ok((page, true)),
        }
    }
}

//loop through and read all of the pages of a book, return the resulting book
fn read_book(file: &mut File, book_id: u16, title: String) -> io::Result<Book> {
    let mut book = Book {
        id: book_id,
        name: title,
        pages: Vec::new(),
    };
    let mut page_id = 0;

    loop {
        let (page, book_done) = read_page(file, page_id)?;
        book.pages.push(page);
        page_id += 1;
        if book_done {
            break;
        }
    }

    Ok(book)
}

//parse the given bookXX file into json
fn parse_from_file(path: &str) -> io::Result<String> {
    let mut file = File::open(path)?;
    let mut addr_bytes = [0u8; 2];
    file.read_exact(&mut addr_bytes)?;
    let addr_first = u16::from_le_bytes(addr_bytes);

    let style = ProgressStyle::default_bar()
        .template("[{bar:40.cyan/blue}] {prefix} {pos}/{len}")
        .unwrap()
        .progress_chars("█🮆🮅🮄▀🮃🮂▔ ");
    let bar = ProgressBar::new((addr_first as u64 - 2) / 4).with_style(style);

    let mut books = Vec::new();
    let mut index = 0u16;
    let mut book_id = 0;

    while index != addr_first {
        //seek to the current book's name pointer
        file.seek(SeekFrom::Start(index as u64))?;
        //read the name address value from the pointer
        file.read_exact(&mut addr_bytes)?;
        let name_addr = u16::from_le_bytes(addr_bytes);
        //read the content address value from the next pointer
        file.read_exact(&mut addr_bytes)?;
        let content_addr = u16::from_le_bytes(addr_bytes);

        //seek to the name
        file.seek(SeekFrom::Start(name_addr as u64))?;
        let title = util::parse_string(&mut file)?;
        //seek to the content
        file.seek(SeekFrom::Start(content_addr as u64))?;
        let book = read_book(&mut file, book_id, title)?;
        books.push(book);

        index += 4;
        book_id += 1;
        bar.inc(1);
    }

    serde_json::to_string_pretty(&books).map_err(std::io::Error::other)
}

//NOTE: code for converting from json to _dt-------------------------------------------------------
fn books_to_byte_data(books: Vec<Book>) -> Vec<u8> {
    let mut bytes = Vec::new();
    let mut book_addresses: Vec<u16> = Vec::new();

    let book_count = books.len();
    let book_header_length = 4 * book_count;

    //reserve the item address space
    bytes.resize(book_header_length, 0);

    //set up the progress bar
    let style = ProgressStyle::default_bar()
        .template("[{bar:40.cyan/blue}] {prefix} {pos}/{len}")
        .unwrap()
        .progress_chars("█🮆🮅🮄▀🮃🮂▔ ");
    let bar = ProgressBar::new(book_count as u64).with_style(style);

    for book in books {
        //record the starting address for this book's name
        let name_address = bytes.len() as u16;
        //write the book's name at that address
        let name_bytes = util::encode_string(&book.name);
        bytes.extend(name_bytes);
        //end of string
        bytes.push(0x00);

        //record the starting address for this book's content
        let content_address = bytes.len() as u16;

        //encode each page line by line
        for (page_idx, page) in book.pages.iter().enumerate() {
            //encode image info at start of page if present
            //image x data
            if let Some(x) = page.image_x {
                bytes.push(0x23);
                for b in x.to_string().as_bytes() {
                    bytes.push(*b);
                }
                bytes.push(0x78); // x position
            }
            //image y data
            if let Some(y) = page.image_y {
                bytes.push(0x23);
                for b in y.to_string().as_bytes() {
                    bytes.push(*b);
                }
                bytes.push(0x79); // y position
            }
            //image face data
            if let Some(image_id) = page.image_id {
                bytes.push(0x23); // formatting change
                if image_id != 0xFFF {
                    for b in image_id.to_string().as_bytes() {
                        bytes.push(*b);
                    }
                }
                bytes.push(0x46); // 'F' for face/image
            }
            for (line_idx, line) in page.lines.iter().enumerate() {
                let mut i = 0;
                while i < line.text.len() {
                    let remainder = &line.text[i..];
                    if let Some(rest) = remainder.strip_prefix("<C:") {
                        // push color change byte
                        bytes.push(0x07);

                        // find number after <C:
                        let num_str: String =
                            rest.chars().take_while(|c| c.is_ascii_digit()).collect();

                        let number = num_str.parse::<u8>().unwrap_or_else(|e| {
                            println!("Failed to parse string to u8: From {}; {}", line.text, e);
                            process::exit(1);
                        });

                        bytes.push(number);

                        // advance i past the whole <C:n> tag
                        i += remainder
                            .chars()
                            .take_while(|c| *c != '>')
                            .map(|c| c.len_utf8())
                            .sum::<usize>()
                            + 1; // +1 for '>'
                    } else if remainder.starts_with("<S:") {
                        // existing size change handling
                        bytes.push(0x23);
                        bytes.push(remainder.as_bytes()[3]);
                        bytes.push(0x53);
                        i += 5; // same as before
                    } else {
                        // push normal character as CP932-encoded byte
                        let mut iter = remainder.chars();
                        if let Some(c) = iter.next() {
                            let b = util::encode_string(&c.to_string());
                            bytes.extend(b);
                            i += c.len_utf8();
                        }
                    }
                }
                // end of line if not last line of page
                if line_idx + 1 != page.lines.len() {
                    bytes.push(0x01);
                }
            }
            //end of page if not last page of book
            if page_idx + 1 != book.pages.len() {
                bytes.push(0x02);
                bytes.push(0x03);
            }
        }
        //end of book
        bytes.push(0x00);

        //add the address of the book name and book content to the addresses list
        book_addresses.push(name_address);
        book_addresses.push(content_address);

        //increment the progress bar
        bar.inc(1);
    }

    //fill the address space for the book list
    for (i, &address) in book_addresses.iter().enumerate() {
        let start = i * 2;
        bytes[start..start + 2].copy_from_slice(&address.to_le_bytes());
    }

    bytes
}
