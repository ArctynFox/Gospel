use encoding_rs::SHIFT_JIS;
use std::io::{self, Read};
use std::path::Path; //CP932 compatible

//parse a string from the current position in a given buffer, assuming the end of the string is
//denoted with b'\0'
pub fn parse_string_until_byte<R: Read>(reader: &mut R, end_byte: u8) -> io::Result<String> {
    let mut buffer = Vec::new();
    let mut byte = [0u8; 1];

    //read bytes from starting position until null byte
    loop {
        let n = reader.read(&mut byte)?;
        if n == 0 {
            break;
        }
        if byte[0] == end_byte {
            break;
        }
        buffer.push(byte[0]);
    }

    decode_string(&buffer)
}

pub fn parse_string<R: Read>(reader: &mut R) -> io::Result<String> {
    parse_string_until_byte(reader, b'\0')
}

pub fn decode_string(buffer: &Vec<u8>) -> io::Result<String> {
    //decode the string from CP932
    let (decoded, _, error) = SHIFT_JIS.decode(&buffer);
    if error {
        eprintln!("Error during decoding string. Bytes: {:?}", buffer);
    }

    Ok(decoded.into_owned())
}

//encode a string into a byte array using CP932
pub fn encode_string(s: &str) -> Vec<u8> {
    //encode a string into a byte array using CP932
    let (bytes, _, _) = SHIFT_JIS.encode(s);
    bytes.into_owned()
}

//write an array of bytes to a buffer, followed by a null byte, and return the address of the first
//byte written
pub fn write_bytes_to_buffer(buffer: &mut Vec<u8>, bytes: Vec<u8>) -> u16 {
    let address = bytes.len() as u16;
    buffer.extend_from_slice(&bytes);
    buffer.push(0);

    address
}

pub fn get_file_name(filepath: &str) -> Option<&str> {
    Path::new(filepath)
        .file_name()
        .and_then(|f| f.to_str())
        .map(|filename| filename.split('.').next().unwrap_or(filename))
}
