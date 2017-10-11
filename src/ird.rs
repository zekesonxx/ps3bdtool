use nom::{be_u8, le_u32, le_i32, le_i64};
use super::errors::*;
use std::path::Path;
use std::fs::File;
use std::io::{BufReader, Read};
use flate2::bufread::GzDecoder;

#[derive(Debug, Clone)]
pub struct IRDFile {
    pub version: u8,
    pub game_id: String,
    pub game_name: String,
    pub update_ver: String,
    pub game_ver: String,
    pub app_ver: String,
    pub header_comp: Vec<u8>,
    pub footer_comp: Vec<u8>,
    pub region_hashes: Vec<[u8; 16]>,
    pub pic_data: Vec<u8>, //TODO: switch to [u8; 0x73] when practical, Rust 1.21?
    pub data1: [u8; 16],
    pub data2: [u8; 16],
    pub unique_identifier: u32,
    pub crc32: [u8; 4]
}

// This is a hack because do_parse doesn't allow types
// and rustc can't seem to infer Vec<[u8; 16]> from length_count!(be_u8, count_fixed!(u8, be_u8, 16))
// So, we split it off. (and it looks nicer I guess)
named!(u8_16<[u8; 16]>, count_fixed!(u8, be_u8, 16));

named!(pub parse_ird<IRDFile>, do_parse!(
    tag!("3IRD") >>
    version: be_u8 >>
    game_id: take_str!(9) >>
    namelen: be_u8 >>
    game_name: take_str!(namelen) >>
    update_ver: take_str!(4) >>
    game_ver: take_str!(5) >>
    app_ver: take_str!(5) >>
    //TODO: ver==7 unique ID needs to go here
    headerlen: le_u32 >>
    header_comp: take!(headerlen) >>
    footerlen: le_u32 >>
    footer_comp: take!(footerlen) >>
    region_hashes: length_count!(be_u8, u8_16) >> //region MD5 hashes
    length_count!(le_i32, tuple!(le_i64, u8_16)) >> // file MD5 hashes, thrown out for now.
    take!(4) >> //no joke, there's literally two ReadUInt16()s in a row here that don't feed to anything.
    pic_data: take!(0x73) >> //TODO: version gate behind ver>=9
    data1: u8_16 >>
    data2: u8_16 >>
    //TODO: pic data here for ver<9
    unique_identifier: le_u32 >> //TODO: version gate this behind ver>7
    crc32: count_fixed!(u8, be_u8, 4) >> //TODO verify crc32
    (IRDFile {
        version, data1, data2, unique_identifier, crc32,
        game_id: game_id.to_string(),
        game_name: game_name.to_string(),
        update_ver: update_ver.to_string(),
        game_ver: game_ver.to_string(),
        app_ver: app_ver.to_string(),
        header_comp: header_comp.to_owned(),
        footer_comp: footer_comp.to_owned(),
        region_hashes: region_hashes.to_owned(),
        pic_data: pic_data.to_owned()
    })
));

pub fn read_ird<P: AsRef<Path>>(fpath: P) -> Result<IRDFile> {
    let f = File::open(fpath).chain_err(|| "Failed to open IRD file")?;
    let reader = BufReader::new(f);
    let mut reader = GzDecoder::new(reader).chain_err(|| "Failed to decompress IRD file")?;
    let mut buf = vec![];
    reader.read_to_end(&mut buf).chain_err(|| "Failed to read IRD file")?;

    let parsed: IRDFile = parse_ird(buf.as_ref()).to_result().chain_err(|| "Failed to parse IRD file")?;
    Ok(parsed)
}
