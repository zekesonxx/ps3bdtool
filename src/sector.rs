
//use super::errors::*;
use std::fmt;

#[derive(Clone, Copy)]
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
        write!(f, "Region({}, [{:#X}, {:#X}], {})",
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