
use super::errors::*;
use std::io::{Read, Seek, SeekFrom};
use std::fmt;

pub const SECTOR_SIZE: usize = 2048;

pub struct Sector(pub [u8; SECTOR_SIZE]);

impl Sector {
    pub fn read_sector<R: Read + Seek>(reader: &mut R, sector: usize) -> Result<Sector>{
        let mut output = Sector([0; SECTOR_SIZE]);
        reader.seek(SeekFrom::Start(((sector-1)*SECTOR_SIZE) as u64)).chain_err(|| "Failed to seek")?;
        reader.read_exact(&mut output.0).chain_err(|| "Failed to read sector")?;
        Ok(output)
    }
}

pub struct Region {
    /// Number of the region, starting from 0
    pub id: u32,
    /// Start of the region. Inclusive
    pub start: u32,
    /// End of the region. Inclusive
    pub end: u32,
    /// Is the range encrypted or not
    pub encrypted: bool
}

impl Region {
    pub fn within_region(&self, sector: u32) -> bool {
        (self.start <= sector) && (sector <= self.end)
    }
}

impl fmt::Debug for Region {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Region({}, [{}, {}], {})",
               self.id,
               self.start,
               self.end,
               if self.encrypted {"encrypted"} else {"unencrypted"})
    }
}

pub trait VecRegion {
    fn region_for_sector(&self, sector: u32) -> Option<&Region>;
}

impl VecRegion for Vec<Region> {
    fn region_for_sector(&self, sector: u32) -> Option<&Region> {
        for range in self {
            if range.within_region(sector) {
                return Some(range);
            }
        }
        return None;
    }
}