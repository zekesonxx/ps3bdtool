// `error_chain!` can recurse deeply
#![recursion_limit = "1024"]

#[macro_use] extern crate error_chain;
//#[macro_use] extern crate nom;
extern crate crypto;

pub mod sector;
pub mod disc;
pub mod decrypt;

use std::fs::File;
use std::io::{BufReader, BufWriter, Write};

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
    let f = File::open("LegendsOfRock.iso").chain_err(|| "Failed to open file")?;
    let reader = BufReader::new(f);

    let mut disc = disc::PS3Disc::new(reader)?;

    let fout = File::create("LegendsOfRock.dec.iso").chain_err(|| "Failed to create file")?;
    let mut writer = BufWriter::new(fout);

    println!("sectors: {} ({} bytes)", disc.total_sectors, (disc.total_sectors) as u64*2048);

    for i in 0..disc.total_sectors {
        writer.write_all(disc.read_sector(i).chain_err(|| "failed to read something")?.as_ref()).chain_err(|| "failed to write something")?;
        print!("\r{}/{} ({}%)", i, disc.total_sectors, ((i as f64)/(disc.total_sectors as f64)*100f64).floor());
    }
    println!();

//    println!("{:?}", disc);
//    println!("region for 2954048: {:?}", disc.regions.region_for_sector(2954048));
//    let sec5000: Vec<u8> = disc.read_sector(2954048).chain_err(|| "shit fucked up")?;
//    io::stdout().write(sec5000.as_ref());

    Ok(())
}