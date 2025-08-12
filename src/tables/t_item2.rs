use indicatif::{ProgressBar, ProgressStyle};
use serde::{Deserialize, Serialize};
use std::fs;
use std::fs::File;
use std::io::{self, Read, Seek, SeekFrom, Write}; //CP932 compatible
use std::process;

use crate::util;

#[derive(Serialize, Deserialize)]
struct Item {
    //FC item table doesn't have item IDs but I add them to the json to make it more readable
    item_id: u16,
    item_name: String,
    item_desc: String,
}

pub fn convert_t_items2_to_json_file(path: String) -> io::Result<()> {
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

pub fn convert_json_to_t_items2(path: String) -> io::Result<()> {
    let json_data = fs::read_to_string(&path)?;
    let items: Vec<Item> = serde_json::from_str(&json_data)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

    let dt_data = items_to_byte_data(items);

    let mut file = File::create("t_item2._dt")?;
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

    let mut items = Vec::new();

    let mut id = 0;

    //until the address of the first datum is reached, keep reading datums from pointers at the
    //beginning of the file
    while index_current_datum != address_first_datum {
        //read the address of the current datum from the pointer list
        file.seek(SeekFrom::Start(index_current_datum as u64))?;
        file.read_exact(&mut address_bytes)?;
        let address_current_datum: u16 = u16::from_le_bytes(address_bytes);

        //move to the address previously read and parse the name and description from the datum
        file.seek(SeekFrom::Start(address_current_datum as u64 + 4))?;

        let datum_name = util::parse_string(&mut file)?;
        let datum_desc = util::parse_string(&mut file)?;

        //add the item data to the item list
        items.push(Item {
            item_id: id,
            item_name: datum_name,
            item_desc: datum_desc,
        });

        //increment things as necessary
        index_current_datum += 2;
        bar.inc(1);
        id += 1;
    }

    //serialize the finalized item list as json data
    let json = serde_json::to_string_pretty(&items).unwrap();

    Ok(json)
}

fn items_to_byte_data(items: Vec<Item>) -> Vec<u8> {
    let mut bytes = Vec::new();
    let mut item_addresses = Vec::new();

    let item_count = items.len();
    let item_header_length = 2 * item_count;

    //reserve the item address space
    bytes.resize(item_header_length, 0);

    //set up the progress bar
    let style = ProgressStyle::default_bar()
        .template("[{bar:40.cyan/blue}] {prefix} {pos}/{len}")
        .unwrap()
        .progress_chars("█🮆🮅🮄▀🮃🮂▔ ");
    let bar = ProgressBar::new(item_count as u64).with_style(style);

    for item in items {
        //record the starting address for this item's data
        let address = bytes.len() as u16;
        item_addresses.push(address);

        //reserve the name and desc address space for this item
        let item_data_address_pos = bytes.len();
        bytes.resize(item_data_address_pos + 4, 0);

        //encode the name and description in CP932
        let name_bytes = util::encode_string(&item.item_name);
        let desc_bytes = util::encode_string(&item.item_desc);

        //write the name to the buffer followed by a null byte
        let name_address = util::write_bytes_to_buffer(&mut bytes, name_bytes);

        //write the description to the buffer right after the name, again followed by a null byte
        let desc_address = util::write_bytes_to_buffer(&mut bytes, desc_bytes);

        //fill the address space for this item
        bytes[item_data_address_pos..item_data_address_pos + 2]
            .copy_from_slice(&name_address.to_le_bytes());
        bytes[item_data_address_pos + 2..item_data_address_pos + 4]
            .copy_from_slice(&desc_address.to_le_bytes());

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
