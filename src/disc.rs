
use super::errors::*;
use sector::{Region, VecRegion};
use std::io::{Read, Seek, SeekFrom};

fn be_u32(i: &[u8]) -> u32 {
    ((i[0] as u32) << 24) + ((i[1] as u32) << 16) + ((i[2] as u32) << 8) + i[3] as u32
}

#[derive(Debug)]
pub struct PS3Disc<F> {
    pub region_count: u32,
    pub regions: Vec<Region>,
    pub total_sectors: u32,
    pub disc_key_maybe_idunno: [u8; 16],
    pub gameid: String,
    pub f70_tagline: String,
    pub reader_handle: F
}

impl<F: Read+Seek> PS3Disc<F> {
    pub fn new(mut handle: F) -> Result<Self> {
        let mut header = [0; 4096];
        handle.read_exact(&mut header).chain_err(|| "Failed to read disc header")?;

        let num_normal_regions= be_u32(&header[0..4]);

        let num_regions = (num_normal_regions * 2) - 1;

        let f70_tagline = String::from_utf8_lossy(&header[0xF70..(0xF70+16)]);

        let game_id = String::from_utf8_lossy(&header[2064..(2064+20)]);
        let game_id = game_id.trim_right();

        let mut d1 = [0u8; 16];
        d1.copy_from_slice(&header[3968..(3968+16)]);

        // Get the sectors

        let mut num = 8usize;
        num += 4;

        let mut next_sector_encrypted = false;
        let mut last_sector_ended_at = 0;
        let mut regions: Vec<Region> = vec![];
        for num2 in 0..num_regions {
            let sector_start = if last_sector_ended_at == 0 {0} else {last_sector_ended_at+1};
            let sector_end= be_u32(&header[num..(num+4)]);
            last_sector_ended_at = sector_end;
            num += 4;
            regions.push(Region {
                id: num2,
                start: sector_start,
                end: sector_end,
                encrypted: next_sector_encrypted
            });
            next_sector_encrypted = !next_sector_encrypted;
        }

        Ok(PS3Disc {
            region_count: regions.len() as u32,
            regions: regions,
            total_sectors: last_sector_ended_at,
            disc_key_maybe_idunno: d1,
            gameid: game_id.to_string(),
            f70_tagline: f70_tagline.to_string(),
            reader_handle: handle
        })
    }
}