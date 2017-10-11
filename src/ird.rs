use nom::{be_u8, le_u32, le_i32, le_i64};

#[derive(Debug, Clone)]
pub struct IRDFile<'a> {
    pub version: u8,
    pub game_id: &'a str,
    pub game_name: &'a str,
    pub update_ver: &'a str,
    pub game_ver: &'a str,
    pub app_ver: &'a str,
    pub header_comp: &'a [u8],
    pub footer_comp: &'a [u8],
    pub region_hashes: Vec<&'a [u8]>,
    pub pic_data: &'a [u8], //TODO: switch to [u8; 0x73] when practical, Rust 1.21?
    pub data1: [u8; 16],
    pub data2: [u8; 16],
    pub unique_identifier: u32
}

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
    region_hashes: length_count!(be_u8, take!(16)) >> //region MD5 hashes
    length_count!(le_i32, tuple!(le_i64, take!(16))) >> // file MD5 hashes, thrown out for now.
    take!(4) >> //no joke, there's literally two ReadUInt16()s in a row here that don't feed to anything.
    pic_data: take!(0x73) >> //TODO: version gate behind ver>=9
    data1: count_fixed!(u8, be_u8, 16) >>
    data2: count_fixed!(u8, be_u8, 16) >>
    //TODO: pic data here for ver<9
    unique_identifier: le_u32 >> //TODO: version gate this behind ver>7
    (IRDFile {
        version, region_hashes, pic_data, data1, data2, unique_identifier,
        game_id, game_name, update_ver, game_ver, app_ver,
        header_comp, footer_comp,
    })
));