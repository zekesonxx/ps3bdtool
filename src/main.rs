// `error_chain!` can recurse deeply
#![recursion_limit = "1024"]

#[macro_use] extern crate error_chain;
#[macro_use] extern crate nom;

mod sector;

use std::fs::File;
use std::io;
use std::io::{BufReader, Write, Read};

use sector::{SectorRange, VecSectorRange};

mod errors {
    // Create the Error, ErrorKind, ResultExt, and Result types
    error_chain!{}
}

use errors::*;

quick_main!(run);

fn be_u32(i: &[u8]) -> u32 {
    ((i[0] as u32) << 24) + ((i[1] as u32) << 16) + ((i[2] as u32) << 8) + i[3] as u32
}


fn run() -> Result<()> {

    let mut f = File::open("LegendsOfRock.iso").chain_err(|| "Failed to open file")?;
    let mut reader = BufReader::new(f);

    let mut header = [0; 4096];
    reader.read_exact(&mut header);

    let normal_sectors= be_u32(&header[0..4]);
    println!("num normal sectors: {}", normal_sectors);
    let num_sectors = (normal_sectors * 2) - 1;
    println!("num total sectors: {}", num_sectors);

    let game_id = String::from_utf8_lossy(&header[2064..(2064+20)]);
    println!("game id: \"{}\"", game_id);

    let mut num = 8usize;
    let mut start_sector = be_u32(&header[num..(num+4)]) as u64;
    num += 4;

    let mut next_sector_encrypted = false;
    let mut last_sector_ended_at = 0;

    let mut sectors: Vec<SectorRange> = vec![];

    for num2 in 0..num_sectors {
        let sector_start = if last_sector_ended_at == 0 {0} else {last_sector_ended_at+1};
        let sector_end= be_u32(&header[num..(num+4)]);
        last_sector_ended_at = sector_end;
        num += 4;
        sectors.push(SectorRange {
            id: num2,
            start: sector_start,
            end: sector_end,
            encrypted: next_sector_encrypted
        });
        next_sector_encrypted = !next_sector_encrypted;
    }
    println!("sectors: {:?}", sectors);

    println!("sector for 5000: {:?}", sectors.range_for_sector(5000));

    println!("0xF70 tagline: {}", String::from_utf8_lossy(&header[0xF70..0xFC4]));


    Ok(())
}