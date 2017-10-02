
use super::errors::*;
use sector::{Region, VecRegion};
use std::io::{Read, Seek, SeekFrom};
use decrypt;

fn be_u32(i: &[u8]) -> u32 {
    ((i[0] as u32) << 24) + ((i[1] as u32) << 16) + ((i[2] as u32) << 8) + i[3] as u32
}

#[derive(Debug)]
pub struct PS3Disc<F> {
    /// Encrypted and unencrypted areas of the disc, specified by sectors.
    pub regions: Vec<Region>,
    /// Total number of sectors on the disc
    pub total_sectors: u32,
    /// The disc's d1 key, if present. This is used to generate the disc key.
    pub d1: [u8; 16],
    /// 128-bit AES key used to encrypt the sectors, along with a per-sector iV
    pub disc_key: [u8; 16],
    /// PlayStation Game ID, ex BCUS-12345
    pub gameid: String,
    /// 3k3y Tagline, which may or may not be present, ex "Encrypted 3K BLD"
    /// If this is present, it will be automatically rewritten to "Decrypted" when sector 1 (the second sector) is read
    pub tagline_3k3y: Option<String>,
    /// File handle used to read the disc
    reader_handle: F
}

impl<F: Read+Seek> PS3Disc<F> {
    pub fn new(mut handle: F) -> Result<Self> {
        // Read the first two sectors (disc sectors are 2KiB)
        // Sector 0 contains the region information (as in, encrypted sectors, not region coding)
        // Sector 1 contains the PlayStation3 magic number, game ID, and some other data
        // Sector 1 also ends with the 3k3y-injected data, if it's present.
        let mut header = [0; 4096];
        handle.read_exact(&mut header).chain_err(|| "Failed to read disc header")?;

        // Number of normal sector regions according to the disc.
        let num_normal_regions= be_u32(&header[0..4]);

        // Number of total regions.
        let num_regions = (num_normal_regions * 2) - 1;

        // Get the 3k3y tagline, if it exists.
        // This immediately proceeds the ird-injected d1 key, which is used to generate the disc key
        //TODO when IRD files are implemented, fix this part.
        let mut f70 = &header[0xF70..(0xF70+16)];
        let tagline_3k3y = if f70 == &[0u8; 16] {
            None
        } else {
            Some(String::from_utf8_lossy(f70).to_string())
        };

        // Get the game ID and remove the space padding
        let game_id = String::from_utf8_lossy(&header[2064..(2064+20)]);
        let game_id = game_id.trim_right();

        // Get the encrypted (decrypted...?) disc key
        let mut d1 = [0u8; 16];
        d1.copy_from_slice(&header[3968..(3968+16)]);

        // Calculate the disc key from d1
        let mut disc_key_arr = [0u8; 16];
        let disc_key: Vec<u8> = decrypt::disc_key(&d1).chain_err(|| "Failed to generate disc key")?;
        disc_key_arr.copy_from_slice(disc_key.as_slice());

        // Get the region list
        let mut regions: Vec<Region> = vec![];
        let mut flag = true;
        let mut num = 8usize;
        let mut start_sector = be_u32(&header[num..(num+4)]);
        num += 4;
        for num2 in 0..num_regions {
            let num3 = be_u32(&header[num..(num+4)]);
            num += 4;
            regions.push(Region {
                id: num2,
                // This is dumb, but it works and creates the right sector numbers
                // I think the header might be listing unencrypted sector bounds,
                // not the start of each sector.
                start: if flag {start_sector} else {start_sector+1},
                end: if flag {num3} else {num3-1},
                encrypted: !flag
            });
            flag = !flag;
            start_sector = num3;
        }

        Ok(PS3Disc {
            regions: regions,
            total_sectors: start_sector+1,
            d1: disc_key_arr,
            disc_key: disc_key_arr,
            gameid: game_id.to_string(),
            tagline_3k3y: tagline_3k3y,
            reader_handle: handle
        })
    }

    pub fn read_sector(&mut self, sector: u32) -> Result<Vec<u8>> {
        let mut buf = [0u8; 2048];
        &self.reader_handle.seek(SeekFrom::Start((sector as u64)*2048))
            .chain_err(|| "failed to seek")?;
        &self.reader_handle.read_exact(&mut buf).chain_err(|| "failed to read")?;

        if self.regions.region_for_sector(sector).unwrap().encrypted {
            // code courtesy of the PS3DevWiki.
            let mut iV = [0u8; 16];
            iV[12] = ((sector & 0xFF000000)>>24) as u8;
            iV[13] = ((sector & 0x00FF0000)>>16) as u8;
            iV[14] = ((sector & 0x0000FF00)>> 8) as u8;
            iV[15] = ((sector & 0x000000FF)>> 0) as u8;
            decrypt::aes_decrypt(&buf, &self.disc_key, &iV)
        } else {
            if sector == 1 && self.tagline_3k3y.is_some() {
                // Patch the 3k3y tagline if it exists
                // at the end of the second sector (0 indexed so it's sector 1)
                // mainly just so we get byte-for-byte identical decrypts
                // Encrypted taglines: Encrypted 3K ___
                // Decrypted taglines: Decrypted 3K ___
                // so we just change the "En" to "De"
                buf[1904] = b'D';
                buf[1905] = b'e';
            }

            Ok(buf.to_owned())
        }
    }
}