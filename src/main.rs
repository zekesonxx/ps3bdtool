// `error_chain!` can recurse deeply
#![recursion_limit = "1024"]

#[macro_use] extern crate error_chain;
//#[macro_use] extern crate nom;
extern crate crypto;

mod sector;
mod disc;
mod decrypt;

use std::fs::File;
use std::io;
use std::io::{BufReader, Write, Read};

use sector::{Region, VecRegion};

pub mod errors {
    // Create the Error, ErrorKind, ResultExt, and Result types
    error_chain!{
        errors {
            SymmetricCipherError(t: ::crypto::symmetriccipher::SymmetricCipherError) {
                description("rust-crypto SymmetricCipherError")
                display("{:?}", t)
            }
        }
    }
}

use errors::*;

quick_main!(run);

fn run() -> Result<()> {
    let mut f = File::open("LegendsOfRock.iso").chain_err(|| "Failed to open file")?;
    let mut reader = BufReader::new(f);

    let mut disc = disc::PS3Disc::new(reader)?;
    //println!("{:?}", disc);
    let sec5000: Vec<u8> = disc.read_sector(4352).chain_err(|| "shit fucked up")?;
    io::stdout().write(sec5000.as_ref());

    Ok(())
}