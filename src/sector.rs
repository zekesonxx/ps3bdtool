
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

pub struct SectorRange {
    /// Number of the range, starting from 0
    pub id: u32,
    /// Start of the range. Inclusive
    pub start: u32,
    /// End of the range. Inclusive
    pub end: u32,
    /// Is the range encrypted or not
    pub encrypted: bool
}

impl SectorRange {
    pub fn within_range(&self, sector: u32) -> bool {
        (self.start <= sector) && (sector <= self.end)
    }
}

impl fmt::Debug for SectorRange {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "SectorRange({}, [{}, {}], {})",
               self.id,
               self.start,
               self.end,
               if self.encrypted {"encrypted"} else {"unencrypted"})
    }
}

pub trait VecSectorRange {
    fn range_for_sector(&self, sector: u32) -> Option<&SectorRange>;
}

impl VecSectorRange for Vec<SectorRange> {
    fn range_for_sector(&self, sector: u32) -> Option<&SectorRange> {
        for range in self {
            if range.within_range(sector) {
                return Some(range);
            }
        }
        return None;
    }
}