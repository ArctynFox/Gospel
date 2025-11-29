use encoding_rs::SHIFT_JIS;
use std::io::{self, Read};
use std::num::ParseIntError;
use std::path::Path; //CP932 compatible

//parse an arbitrary length u16 from a char array given the starting index,
//return the tuple of (value, length)
pub fn parse_u16_from_chars(
    char_array: &[char],
    start_index: u16,
) -> Result<(u16, usize), ParseIntError> {
    let mut end_index = start_index;

    //find the end of the number
    while end_index < char_array.len() as u16 && char_array[end_index as usize].is_ascii_digit() {
        end_index += 1;
    }

    //concatenate the chars of the number subset into a string
    let values: String = char_array[start_index as usize..end_index as usize]
        .iter()
        .collect();

    //parse the string as a u16
    Ok((values.parse::<u16>()?, values.len()))
}

//parse a string from the current position in a given buffer, ending at the given byte
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
    let (decoded, _, error) = SHIFT_JIS.decode(buffer);
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

//NOTE: tests

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn try_parse_u16() {
        let test_string: Vec<char> = "af-fs#241Fawx".chars().collect();
        let start_index = 6;
        let value = parse_u16_from_chars(&test_string, start_index).unwrap();
        assert_eq!(value, (241, 3));
    }

    #[test]
    fn try_parse_u16_jp() {
        let test_string: Vec<char> = "何とか#241Fうんたらかんたら".chars().collect();
        let start_index = 4;
        let value = parse_u16_from_chars(&test_string, start_index).unwrap();
        assert_eq!(value, (241, 3));
    }

    #[test]
    fn cp932_round_trip() {
        let test_string = "なんといっても日本語だぜ/No matter what you say, it's English";
        let test_encoded = encode_string(test_string);
        let test_decoded = decode_string(&test_encoded).unwrap();
        assert_eq!(test_string, test_decoded);
    }
}
