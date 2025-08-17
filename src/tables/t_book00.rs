use indicatif::{ProgressBar, ProgressStyle};
use serde::{Deserialize, Serialize};
use std::fs;
use std::fs::File;
use std::io::{self, Read, Seek, SeekFrom, Write}; //CP932 compatible
use std::process;

use crate::util;

#[derive(Serialize, Deserialize)]
struct BookReference {
    id: u8,
    item_id: u16,
    dt_id: u16,
    archive_id: u16,
    book_id: u16,
}

pub fn convert_t_book00_to_json_file(input_path: String) -> io::Result<()> {
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

pub fn convert_json_to_t_book00(input_path: String) -> io::Result<()> {
    let json_data = fs::read_to_string(&input_path)?;
    let file_name = util::get_file_name(&input_path);
    let book_references: Vec<BookReference> = serde_json::from_str(&json_data)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

    let dt_data = book00_to_byte_data(book_references);

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

fn parse_from_file(path: &str) -> io::Result<String> {
    let mut file = File::open(path).unwrap();
    let mut buffer = [0u8; 2];

    //set up the progress bar
    let metadata = fs::metadata(path).unwrap();
    let file_byte_length = metadata.len();
    let style = ProgressStyle::default_bar()
        .template("[{bar:40.cyan/blue}] {prefix} {pos}/{len}")
        .unwrap()
        .progress_chars("█🮆🮅🮄▀🮃🮂▔ ");
    let bar = ProgressBar::new(file_byte_length).with_style(style);

    let mut book_references: Vec<BookReference> = Vec::new();

    let mut index: u8 = 0;
    let mut file_done = false;
    while !file_done {
        let mut book_reference = BookReference {
            id: index,
            item_id: 0,
            dt_id: 0,
            archive_id: 0,
            book_id: 0,
        };

        index += 1;

        //read item id
        file.read_exact(&mut buffer).unwrap();
        book_reference.item_id = u16::from_le_bytes(buffer);

        //2 null bytes
        file.seek(SeekFrom::Current(2)).unwrap();

        //read dt file id
        file.read_exact(&mut buffer).unwrap();
        book_reference.dt_id = u16::from_le_bytes(buffer);

        //read dt archive id
        file.read_exact(&mut buffer).unwrap();
        book_reference.archive_id = u16::from_le_bytes(buffer);

        //read within-dt-file book id
        file.read_exact(&mut buffer).unwrap();
        book_reference.book_id = u16::from_le_bytes(buffer);

        file.seek(SeekFrom::Current(2)).unwrap();

        //end of file is denoted by a 12 byte set of 0xFF
        if book_reference.item_id == 0xFFFF
            && book_reference.dt_id == 0xFFFF
            && book_reference.archive_id == 0xFFFF
            && book_reference.book_id == 0xFFFF
        {
            file_done = true;
        } else {
            book_references.push(book_reference);
        }
        bar.inc(1);
    }

    serde_json::to_string_pretty(&book_references).map_err(std::io::Error::other)
}

fn book00_to_byte_data(book_references: Vec<BookReference>) -> Vec<u8> {
    let mut bytes: Vec<u8> = Vec::new();

    //set up the progress bar
    let style = ProgressStyle::default_bar()
        .template("[{bar:40.cyan/blue}] {prefix} {pos}/{len}")
        .unwrap()
        .progress_chars("█🮆🮅🮄▀🮃🮂▔ ");
    let bar = ProgressBar::new(book_references.len() as u64).with_style(style);

    for book_reference in book_references {
        bytes.extend(book_reference.item_id.to_le_bytes());
        bytes.extend([0x00, 0x00]);
        bytes.extend(book_reference.dt_id.to_le_bytes());
        bytes.extend(book_reference.archive_id.to_le_bytes());
        bytes.extend(book_reference.book_id.to_le_bytes());
        bytes.extend([0x00, 0x00]);

        bar.inc(1);
    }

    bytes.extend([
        0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
    ]);

    bytes
}
