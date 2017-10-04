// `error_chain!` can recurse deeply
#![recursion_limit = "1024"]

#[macro_use] extern crate error_chain;
//#[macro_use] extern crate nom;
#[macro_use] extern crate clap;
extern crate crypto;
extern crate bytesize;
extern crate hex;

pub mod sector;
pub mod disc;
pub mod decrypt;

use std::fs::File;
use std::path::PathBuf;
use std::io::{BufReader, BufWriter, Write};
use std::ffi::OsStr;
use bytesize::ByteSize;
use hex::FromHex;

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
    let matches: clap::ArgMatches = clap_app!(ps3bdtool =>
        (@setting ArgRequiredElseHelp)
        (version: crate_version!())
        (author: crate_authors!())
        (about: "Tool to manipulate PS3 game discs")
        (@subcommand info =>
            (about: "Print information about a disc")
            (@setting ArgRequiredElseHelp)
            (@arg FILE: +required "File to print information about")
            (@arg id: -i --id "Just print game ID, nothing else")
            (@arg keys: -k --keys "Print the game's decryption keys")
        )
        (@subcommand decrypt =>
            (about: "Decrypt a game iso")
            (@setting ArgRequiredElseHelp)
            (@arg FILE: +required "File to decrypt")
            (@arg OUTFILE: "Output file, defaults to <input>.dec.iso")
            (@arg d1: -d --d1 +takes_value "Game's d1 value as a string of hex bytes, used to calculate the disc key")
            (@arg key: -k --key +takes_value "Decryption key as a string of hex bytes (not implemented)")
            (@arg threads: -j --threads +takes_value "Number of threads to decrypt with. Defaults to num_cpus+1 (not implemented)")
            (@arg irdfile: --irdfile +takes_value "IRD file to extract key from (not implemented)")
        )
    ).get_matches();
    match matches.subcommand() {
        ("info", Some(matches)) => {
            let f = File::open(matches.value_of("FILE").unwrap()).chain_err(|| "Failed to open file")?;
            let reader = BufReader::new(f);

            let disc = disc::PS3Disc::new(reader)?;
            if matches.is_present("id") {
                println!("{}", disc.gameid);
            } else if matches.is_present("keys") {
                print!("      d1: ");
                for &byte in disc.d1.unwrap().as_ref() {
                    print!("{:02X}", byte);
                }
                println!();
                print!("disc_key: ");
                for &byte in disc.disc_key.unwrap().as_ref() {
                    print!("{:02X}", byte);
                }
                println!();
            } else {
                //TODO get the game name
                println!("{filename}: {gameid}, {bytes}, {regions} regions",
                    filename=matches.value_of("FILE").unwrap(),
                    gameid=disc.gameid,
                    bytes=ByteSize::b((disc.total_sectors as usize)*2048).to_string(true),
                    regions=disc.regions.len()
                );
                for region in disc.regions {
                    println!("Region {id}: sectors {start:X}-{end:X} ({start}-{end}), {encrypted}",
                             id=region.id,
                             start=region.start,
                             end=region.end,
                             encrypted=if region.encrypted {"encrypted"} else {"unencrypted"}
                           )
                }
                println!("https://rpcs3.net/compatibility?g={}",
                         disc.gameid.replace('-', ""));
            }
        },
        ("decrypt", Some(matches)) => {
            println!("input: {}", PathBuf::from(matches.value_of("FILE").unwrap()).display());
            let f = File::open(matches.value_of("FILE").unwrap()).chain_err(|| "Failed to open file")?;
            let reader = BufReader::new(f);

            let mut disc = disc::PS3Disc::new(reader)?;

            let output_path = if let Some(outfile) = matches.value_of("OUTFILE") {
                PathBuf::from(outfile)
            } else {
                let mut pathbuf = PathBuf::from(matches.value_of("FILE").unwrap());
                if pathbuf.extension() == Some(OsStr::new("iso")) {
                    pathbuf.set_extension("dec.iso");
                } else {
                    pathbuf = PathBuf::from(format!("{}.dec.iso", disc.gameid.replace('-', "")));
                }
                pathbuf
            };

            println!("output: {}", output_path.display());
            let fout = File::create(output_path).chain_err(|| "Failed to create file")?;
            let mut writer = BufWriter::new(fout);

            if let Some(d1) = matches.value_of("d1") {
                let d1: Vec<u8> = FromHex::from_hex(d1.as_bytes().to_owned()).chain_err(|| "failed to parse key")?;
                disc.set_d1(d1.as_ref())?;
            }

            if let Some(key) = matches.value_of("key") {
                let disc_key: Vec<u8> = FromHex::from_hex(key.as_bytes().to_owned()).chain_err(|| "failed to parse key")?;
                disc.set_disc_key(disc_key.as_ref())?;
            }

            if disc.disc_key.is_none() {
                println!("No 3k3y header found, and no d1 or disc key specified!");
                println!("Disc can't be decrypted without any of those.");
                println!("Consider passing a value to --key or --d1");
                //TODO ird files
                return Ok(());
            }
            if let Some(d1) = disc.d1 {
                print!("using d1: ");
                for &byte in d1.as_ref() {
                    print!("{:02X}", byte);
                }
                println!();
            }
            print!("{} disc key: ", if disc.d1.is_some() {"calculated"} else {"using"});
            for &byte in disc.disc_key.unwrap().as_ref() {
                print!("{:02X}", byte);
            }
            println!();


            println!("sectors: {sectors} ({size}), regions: {regions}",
                     sectors=disc.total_sectors,
                     size=ByteSize::b(disc.total_sectors as usize * 2048).to_string(true),
                     regions=disc.regions.len());

            for i in 0..disc.total_sectors {
                writer.write_all(disc.read_sector(i).chain_err(|| "failed to read something")?.as_ref()).chain_err(|| "failed to write something")?;
                print!("\rsector: {}/{} ({}%)",
                       i,
                       disc.total_sectors,
                       ((i as f64)/(disc.total_sectors as f64)*100f64).floor()
                );
            }
            println!();
        },
        (_, _) => unreachable!()
    }
    Ok(())
}