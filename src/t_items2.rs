use encoding_rs::SHIFT_JIS;
use indicatif::{ProgressBar, ProgressStyle};
use serde::{Deserialize, Serialize};
use std::fs;
use std::fs::File;
use std::io::{self, Read, Seek, SeekFrom, Write}; //CP932 compatible

#[derive(Serialize, Deserialize)]
struct Item {
    //item_id: u16,
    item_name: String,
    item_desc: String,
}

pub fn convert_t_items2_to_json_file(path: String) -> io::Result<()> {
    let table_data = parse_from_file(path)?;

    let mut output = File::create("t_item2.json")?;
    output.write_all(table_data.as_bytes())?;
    output.flush()?;

    Ok(())
}

pub fn convert_json_to_t_items2(path: String) -> io::Result<()> {
    let json_data = fs::read_to_string(path)?;
    let items: Vec<Item> = serde_json::from_str(&json_data)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

    let dt_data = items_to_byte_data(items);

    let mut file = File::create("t_item2._dt")?;
    file.write_all(&dt_data)?;

    Ok(())
}

fn parse_from_file(path: String) -> io::Result<String> {
    let mut file = File::open(path)?;
    let mut buffer = [0u8; 2];

    //get the address of the first datum (the beginning of the file is a collection of addresses
    //that refer to each datum)
    file.read_exact(&mut buffer)?;
    let address_first_datum = u16::from_le_bytes(buffer);

    //set up the progress bar
    let style = ProgressStyle::default_bar()
        .template("[{bar:40.cyan/blue}] {prefix} {pos}/{len}")
        .unwrap()
        .progress_chars("█🮆🮅🮄▀🮃🮂▔ ");
    let bar = ProgressBar::new((address_first_datum as u64 - 2) / 2).with_style(style);

    let mut index_current_datum: u16 = 0;

    let mut items = Vec::new();

    //let mut id = 0;

    //until the address of the first datum is reached, keep reading datums from pointers at the
    //beginning of the file
    while index_current_datum != address_first_datum {
        //read the address of the current datum from the pointer list
        file.seek(SeekFrom::Start(index_current_datum as u64))?;
        file.read_exact(&mut buffer)?;
        let address_current_datum: u16 = u16::from_le_bytes(buffer);

        //move to the address previously read and parse the name and description from the datum
        file.seek(SeekFrom::Start(address_current_datum as u64 + 4))?;

        let datum_name = parse_string(&mut file)?;
        let datum_desc = parse_string(&mut file)?;

        //add the item data to the item list
        items.push(Item {
            //item_id: id,
            item_name: datum_name,
            item_desc: datum_desc,
        });

        //increment things as necessary
        index_current_datum += 2;
        bar.inc(1);
        //id += 1;
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
        let name_bytes = encode_string(&item.item_name);
        let desc_bytes = encode_string(&item.item_desc);

        //write the name to the buffer followed by a null byte
        let name_address = bytes.len() as u16;
        bytes.extend_from_slice(&name_bytes);
        bytes.push(0);

        //write the description to the buffer right after the name, again followed by a null byte
        let desc_address = bytes.len() as u16;
        bytes.extend_from_slice(&desc_bytes);
        bytes.push(0);

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

fn parse_string<R: Read>(reader: &mut R) -> io::Result<String> {
    let mut buffer = Vec::new();
    let mut byte = [0u8; 1];

    //read bytes from starting position until null byte
    loop {
        let n = reader.read(&mut byte)?;
        if n == 0 {
            break;
        }
        if byte[0] == 0 {
            break;
        }
        buffer.push(byte[0]);
    }

    //decode the string from CP932
    let (decoded, _, error) = SHIFT_JIS.decode(&buffer);
    if error {
        eprintln!("Error during decoding string.");
    }

    Ok(decoded.into_owned())
}

fn encode_string(s: &str) -> Vec<u8> {
    //encode a string into a byte array using CP932
    let (bytes, _, _) = SHIFT_JIS.encode(s);
    bytes.into_owned()
}
