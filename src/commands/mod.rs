pub mod decrypt;
pub mod info;

use std::io::prelude::*;
use hex::FromHex;
use super::{disc, ird, config};
use super::errors::*;

pub fn find_key_if_possible<F: Read+Seek>(disc: &mut disc::PS3Disc<F>, matches: &::clap::ArgMatches) -> Result<bool> {
    // Check if a IRD file has been passed on the command line
    if matches.is_present("irdfile") && (matches.is_present("d1") || matches.is_present("key")) {
        println!("warning: --ird takes precedence over --key/--d1");
    }

    if let Some(ird_path) = matches.value_of("irdfile") {
        let parsed = ird::read_ird(ird_path)?;
        disc.import_from_ird(&parsed)?;
        return Ok(true);
    }


    // Check if the user passed us --d1 and/or --key, and set them accordingly
    if matches.is_present("d1") && matches.is_present("key") {
        println!("warning: --key takes precedence over --d1");
    }

    if let Some(key) = matches.value_of("key") {
        let disc_key: Vec<u8> = FromHex::from_hex(key.as_bytes().to_owned()).chain_err(|| "failed to parse key")?;
        disc.set_disc_key(disc_key.as_ref())?;
        return Ok(true);
    } else if let Some(d1) = matches.value_of("d1") {
        let d1: Vec<u8> = FromHex::from_hex(d1.as_bytes().to_owned()).chain_err(|| "failed to parse key")?;
        disc.set_d1(d1.as_ref())?;
        return Ok(true);
    }


    // If nothing was specified by the user, check their folders for it.
    if let Some(ird_path) = config::find_ird_file(disc.gameid.replace('-', "").as_ref()).chain_err(||"argh")? {
        let parsed = ird::read_ird(ird_path)?;
        disc.import_from_ird(&parsed)?;
        return Ok(true);
    }


    // If nothing worked, give up
    Ok(false)
}