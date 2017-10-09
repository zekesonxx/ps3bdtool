
use super::errors::*;
use sector::{Region, VecRegion};
use std::io::{Read, Seek, SeekFrom};
use decrypt;

/// given a four-element &[u8], calculate the big-endian u32 that they represent
/// shamelessly taken out of nom
fn be_u32(i: &[u8]) -> u32 {
    debug_assert_eq!(i.len(), 4, "a u32 is 4 bytes and yet I didn't get 4 bytes");
    ((i[0] as u32) << 24) + ((i[1] as u32) << 16) + ((i[2] as u32) << 8) + i[3] as u32
}

/// Wrapped PS3 disc
///
/// Using read_sector, will transparently decrypt sectors as needed.
#[derive(Debug)]
pub struct PS3Disc<F> {
    /// Encrypted and unencrypted areas of the disc, specified by sectors.
    pub regions: Vec<Region>,
    /// Total number of sectors on the disc
    pub total_sectors: u32,
    /// The disc's d1 key, if present. This is used to generate the disc key.
    pub d1: Option<[u8; 16]>,
    /// 128-bit AES key used to decrypt the sectors, along with a per-sector iV
    pub disc_key: Option<[u8; 16]>,
    /// PlayStation Game ID, ex BCUS-12345
    pub gameid: String,
    /// 3k3y Tagline, which may or may not be present, ex "Encrypted 3K BLD"
    ///
    /// If this is present, it will be automatically rewritten to "Decrypted" when sector 1 (the second sector) is read
    pub tagline_3k3y: Option<String>,
    /// File handle used to read the disc
    reader_handle: F
}

/// Standalone struct to decrypt PS3 disc regions
///
/// This only requires an immutable reference to decrypt a region,
/// and while so does PS3Disc, it's also being used mutably to read sectors.
///
/// This make it a lot easier do multithreaded decrypts.
#[derive(Debug, Clone)]
pub struct PS3DiscDecryptor {
    /// Disc's regions
    pub regions: Vec<Region>,
    /// Disc key
    pub disc_key: [u8; 16],
    /// Whether the 3k3y tagline needs to be patched or not.
    pub has_3k3y_tagline: bool
}


impl<F: Read+Seek> PS3Disc<F> {
    /// Create a new PS3Disc
    pub fn new(mut handle: F) -> Result<Self> {
        // Read the first two sectors (disc sectors are 2KiB)
        // Sector 0 contains the region information (as in, encrypted sectors, not region coding)
        // Sector 1 contains the PlayStation3 magic number, game ID, and some other data
        // Sector 1 also ends with the 3k3y-injected data, if it's present.
        let mut header = [0; 4096];
        handle.read_exact(&mut header).chain_err(|| "Failed to read disc header")?;

        // Check for the magic number, and bail if it's not present.
        if &header[2048..(2048+12)] != b"PlayStation3" {
            bail!("Magic number PlayStation3 not found. Are you sure this is a game disc?");
        }

        // Number of normal sector regions according to the disc.
        let num_normal_regions= be_u32(&header[0..4]);

        // Number of total regions.
        let num_regions = (num_normal_regions * 2) - 1;

        // Get the 3k3y tagline, if it exists.
        // This immediately proceeds the ird-injected d1 key, which is used to generate the disc key
        //TODO when IRD files are implemented, fix this part.
        let f70 = &header[0xF70..(0xF70+16)];
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
        let (d1, disc_key) = if d1 == [0; 16] {
            (None, None)
        } else {
            let mut disc_key_arr = [0u8; 16];
            let disc_key: Vec<u8> = decrypt::disc_key(&d1).chain_err(|| "Failed to generate disc key")?;
            disc_key_arr.copy_from_slice(disc_key.as_slice());
            (Some(d1), Some(disc_key_arr))
        };

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
            regions, d1, disc_key, tagline_3k3y,
            total_sectors: start_sector+1,
            gameid: game_id.to_string(),
            reader_handle: handle
        })
    }

    /// Read a sector, automatically decrypting if needed
    ///
    /// Remember that sector is 0 indexed, so the first sector is #0.
    #[allow(non_snake_case)]
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
            decrypt::aes_decrypt(&buf, &self.disc_key.unwrap(), &iV)
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


    /// Read a sector, but don't automatically decrypt
    ///
    /// Generally speaking, you only want to use this to then feed into a PS3DiscDecryptor
    pub fn read_sector_nodecrypt(&mut self, sector: u32) -> Result<Vec<u8>> {
        let mut buf = [0u8; 2048];
        &self.reader_handle.seek(SeekFrom::Start((sector as u64)*2048))
            .chain_err(|| "failed to seek")?;
        &self.reader_handle.read_exact(&mut buf).chain_err(|| "failed to read")?;
        Ok(buf.to_owned())
    }

    /// Returns a standalone struct that can be used to decrypt individual sectors.
    ///
    /// See struct documentation for more information.
    pub fn get_decryptor(&self) -> PS3DiscDecryptor {
        PS3DiscDecryptor {
            regions: self.regions.clone(),
            disc_key: self.disc_key.unwrap(),
            has_3k3y_tagline: self.tagline_3k3y.is_some()
        }
    }

    /// Safely set the d1 decryption key, used to compute the disc key.
    ///
    /// This function will compute the disc key on execution.
    pub fn set_d1(&mut self, d1: &[u8]) -> Result<()> {
        if d1.len() != 16 {
            bail!("expected d1 length 16, got length {}", d1.len());
        }
        let mut d1_arr = [0u8; 16];
        d1_arr.copy_from_slice(d1);

        let mut disc_key_arr = [0u8; 16];
        let disc_key: Vec<u8> = decrypt::disc_key(&d1_arr).chain_err(|| "Failed to generate disc key")?;
        disc_key_arr.copy_from_slice(disc_key.as_slice());
        self.d1 = Some(d1_arr);
        self.disc_key = Some(disc_key_arr);
        Ok(())
    }

    /// Safely set the disc decryption key (this is *not* the d1 key used to compute the disc key)
    ///
    /// This is not the key the BR drive will give you, nor is it the data1 key in IRD files.
    pub fn set_disc_key(&mut self, disc_key: &[u8]) -> Result<()> {
        if disc_key.len() != 16 {
            bail!("expected disc_key length 16, got length {}", disc_key.len());
        }
        let mut disc_key_arr = [0u8; 16];
        disc_key_arr.copy_from_slice(disc_key);
        self.disc_key = Some(disc_key_arr);
        Ok(())
    }
}

impl PS3DiscDecryptor {
    /// Standalone sector decryption function
    ///
    /// `ps3discdecryptor.decrypt_sector(ps3disc.read_sector_nodecrypt(4), 4)`
    /// is functionally identical to
    /// `ps3disc.read_sector(4)`
    #[allow(non_snake_case)]
    pub fn decrypt_sector(&self, buf: &[u8], sector: u32) -> Result<Vec<u8>> {
        if buf.len() != 2048 {
            bail!("PS3 disc sectors are always exactly 2048 bytes. No partial decrypts.");
        }
        if self.regions.region_for_sector(sector).unwrap().encrypted { //TODO fix this
            // code courtesy of the PS3DevWiki.
            let mut iV = [0u8; 16];
            iV[12] = ((sector & 0xFF000000)>>24) as u8;
            iV[13] = ((sector & 0x00FF0000)>>16) as u8;
            iV[14] = ((sector & 0x0000FF00)>> 8) as u8;
            iV[15] = ((sector & 0x000000FF)>> 0) as u8;
            decrypt::aes_decrypt(&buf, &self.disc_key, &iV)
        } else {
            let mut buf = buf.to_owned();
            if sector == 1 && self.has_3k3y_tagline {
                // Patch the 3k3y tagline if it exists
                // at the end of the second sector (0 indexed so it's sector 1)
                // mainly just so we get byte-for-byte identical decrypts
                // Encrypted taglines: Encrypted 3K ___
                // Decrypted taglines: Decrypted 3K ___
                // so we just change the "En" to "De"
                buf[1904] = b'D';
                buf[1905] = b'e';
            }

            Ok(buf)
        }
    }
}