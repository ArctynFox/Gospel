use indicatif::{ProgressBar, ProgressStyle};
use serde::{Deserialize, Serialize};
use std::fs;
use std::fs::File;
use std::io::{self, Read, Seek, SeekFrom, Write};
//CP932 compatible
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

pub fn convert_t_book_to_json_file(input_path: String) -> io::Result<()> {
    let table_data = parse_from_file(&input_path)?;
    let file_name = util::get_file_name(&input_path);

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

pub fn convert_json_to_t_book(input_path: String) -> io::Result<()> {
    let json_data = fs::read_to_string(&input_path)?;
    let file_name = util::get_file_name(&input_path);
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

fn read_line(file: &mut File, page: &mut Page, line_id: u8) -> io::Result<(Line, ReadStatus)> {
    let mut line = Line {
        id: line_id,
        text: String::new(),
    };

    //byte address of the start of the line
    let address_bytes: [u8; 2] = (file.stream_position()? as u16).to_le_bytes();

    //get the string data of the line.
    let (line_string_unformatted, read_status) = parse_line_bytes(file).unwrap();
    println!("{}", line_string_unformatted);

    let line_chars_unformatted: Vec<char> = line_string_unformatted.chars().collect();

    let mut i = 0;
    while i < line_chars_unformatted.len() {
        let c: char = line_chars_unformatted[i];
        match c {
            '\u{0003}' => {}
            '\u{07}' => {
                line.text
                    .extend(format!("<C:{}>", line_chars_unformatted[i + 1] as u8).chars());
                i += 1;
            }
            '\u{0023}' => {
                let length = handle_formatting(
                    page,
                    &mut line,
                    &line_chars_unformatted,
                    &address_bytes,
                    i as u16 + 1,
                )?;
                i += length;
            }
            _ => line.text.push(c),
        }

        i += 1;
    }

    Ok((line, read_status))
}

//read the line of the file byte by byte to check for byte commands that the game uses
fn parse_line_bytes(reader: &mut File) -> io::Result<(String, ReadStatus)> {
    let mut buffer = Vec::new();
    let mut byte = [0u8; 1];

    //read bytes from starting position until an end of line, page, book, or file occurs
    loop {
        let n = reader.read(&mut byte)?;
        //end of file
        if n == 0 {
            return Ok((util::decode_string(&buffer).unwrap(), ReadStatus::EndBook));
        }
        //end of book
        if byte[0] == 0x00 {
            return Ok((util::decode_string(&buffer).unwrap(), ReadStatus::EndBook));
        }
        //end of line
        if byte[0] == 0x01 {
            return Ok((util::decode_string(&buffer).unwrap(), ReadStatus::Continue));
        }
        //end of page
        if byte[0] == 0x02 {
            return Ok((util::decode_string(&buffer).unwrap(), ReadStatus::EndPage));
        }
        buffer.push(byte[0]);
    }
}

//check the formatting information and handle it appropriately, then return the length of what was
//read so that the segment can be skipped past in the calling function
fn handle_formatting(
    page: &mut Page,
    line: &mut Line,
    line_chars_unformatted: &[char],
    address_bytes: &[u8; 2],
    start_index: u16,
) -> io::Result<usize> {
    println!(
        "formatting char: {}",
        line_chars_unformatted[start_index as usize]
    );
    if line_chars_unformatted[start_index as usize] == 'F' {
        page.image_id = Some(0xFFF);
        return Ok(1);
    }

    let (value, length) = util::parse_u16_from_chars(line_chars_unformatted, start_index).unwrap();

    match line_chars_unformatted[start_index as usize + length] {
        //F, image/face id
        '\u{0046}' => {
            page.image_id = Some(value);
            Ok(length + 1)
        }
        //S, text size
        '\u{0053}' => {
            line.text.extend(format!("<S:{}>", value).chars());
            Ok(2)
        }
        //R, katakana, only used in JP
        '\u{0052}' => {
            util::parse_u16_from_chars(line_chars_unformatted, start_index).unwrap();
            let (katakana, k_length) =
                read_substring_until_hash(line_chars_unformatted, start_index + length as u16 + 1);
            line.text
                .extend(format!("<R:{};{}>", value, katakana).chars());
            Ok(length + k_length + 1)
        }
        //x position of face/image
        '\u{0078}' => {
            page.image_x = Some(value);
            Ok(length + 1)
        }
        //y position of face/image
        '\u{0079}' => {
            page.image_y = Some(value);
            Ok(length + 1)
        }
        _ => panic!(
            "Expected a known formatting type in string but received '#...{}' with line starting at {:02X} {:02X}",
            line_chars_unformatted[start_index as usize + length + 1],
            address_bytes[0],
            address_bytes[1]
        ),
    }
}

//read a substring of the chars list until the '#' character
//return tuple (substring, length)
fn read_substring_until_hash(chars: &[char], start_index: u16) -> (String, usize) {
    let mut katakana_string: String = String::new();
    let mut length: usize = 0;
    for i in start_index as usize..chars.len() {
        if chars[i] == '#' {
            length += 1;
            return (katakana_string, length);
        } else {
            katakana_string.push(chars[i]);
            length += 1;
        }
    }
    return (katakana_string, length);
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
                    } else if remainder.starts_with("<R:") {
                        //katakana
                        bytes.push(0x23);
                        let (value, v_length) = util::parse_u16_from_chars(
                            &remainder.chars().collect::<Vec<char>>(),
                            3,
                        )
                        .unwrap();
                        bytes.extend(util::encode_string(&value.to_string()));
                        bytes.push(0x52);
                        let katakana_then_remainder = remainder.split(';').collect::<Vec<_>>()[1];
                        let katakana = katakana_then_remainder.split('>').collect::<Vec<_>>()[0];
                        let katakana_encoded = util::encode_string(katakana);
                        bytes.extend(katakana_encoded);
                        bytes.push(0x23);
                        i += katakana.len() + v_length + 5;
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
