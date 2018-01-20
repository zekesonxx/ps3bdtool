// Clippy lints
#![allow(unknown_lints)]
#![allow(cast_lossless)]

// `error_chain!` can recurse deeply
#![recursion_limit = "1024"]

#[macro_use] extern crate error_chain;
#[macro_use] extern crate nom;
#[macro_use] extern crate clap;
extern crate crypto;
extern crate bytesize;
extern crate hex;
extern crate time;
extern crate flate2;

// Free disk space checking
#[cfg(unix)] extern crate nix;

// FUSE mounting support
#[cfg(unix)] extern crate fuse;
#[cfg(unix)] extern crate libc;

// XDG config dir support
#[cfg(unix)] extern crate xdg;

macro_rules! hex_println {
    ($a: expr) => {
        for &byte in $a {
            print!("{:02X}", byte);
        }
        println!();
    };
}

pub mod sector;
pub mod disc;
pub mod decrypt;
#[cfg(unix)] pub mod mountvfs;
pub mod config;
pub mod ird;
pub mod commands;

use std::fs::File;
use std::path::PathBuf;
use std::io::BufReader;

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
    let mut app: clap::App = clap_app!(ps3bdtool =>
        (@setting ArgRequiredElseHelp)
        (version: crate_version!())
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
            (@arg key: -k --key +takes_value "Decryption key as a string of hex bytes")
            (@arg threads: -j --threads +takes_value "Number of threads to decrypt with. Defaults to 1. Set to 1 to switch to singlethreaded mode")
            (@arg irdfile: --ird +takes_value "IRD file to extract key from")
        )
        (@subcommand irdinfo =>
            (about: "Print information about a 3k3y IRD file")
            (@setting ArgRequiredElseHelp)
            (@arg FILE: +required "Path to 3k3y IRD file")
            (@arg filehashes: --filehashes "Print file inode numbers and their hashes")
        )
    );
    if cfg!(unix) {
        app = app.subcommand(clap_app!(@subcommand mount =>
            (about: "Use FUSE to mount a filesystem containing a transparently-decrypted iso")
            (@setting ArgRequiredElseHelp)
            (@arg FILE: +required "Path to game image or disc drive")
            (@arg MOUNTPOINT: +required "Path to mount to")
            (@arg verbose: -v --verbose "Output debugging information")
            (@arg d1: -d --d1 +takes_value "Game's d1 value as a string of hex bytes, used to calculate the disc key")
            (@arg key: -k --key +takes_value "Decryption key as a string of hex bytes")
            (@arg irdfile: --ird +takes_value "IRD file to extract key from")
        ));
    }
    let matches = app.get_matches();
    match matches.subcommand() {
        ("info", Some(matches)) => commands::info::disc_info(matches)?,
        ("decrypt", Some(matches)) => commands::decrypt::decrypt_disc(matches)?,
        #[cfg(unix)]
        ("mount", Some(matches)) => {
            println!("disc: {}", PathBuf::from(matches.value_of("FILE").unwrap()).display());
            let f = File::open(matches.value_of("FILE").unwrap()).chain_err(|| "Failed to open file")?;
            let reader = BufReader::new(f);

            let mut disc = disc::PS3Disc::new(reader)?;

            if !commands::find_key_if_possible(&mut disc, matches).chain_err(||"Failed to try and find a key")? && !disc.can_decrypt() {
                println!("No 3k3y header found, and no d1, disc key, or ird file specified!");
                println!("Disc can't be decrypted without any of those.");
                println!("Consider passing a value to --d1 or --ird");
                return Ok(());
            }

            mountvfs::mount(disc, matches.value_of("MOUNTPOINT").unwrap(), matches.is_present("verbose"));
        },
        ("irdinfo", Some(matches)) => commands::info::ird_info(matches)?,
        (_, _) => unreachable!()
    }
    Ok(())
}