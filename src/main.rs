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

// FUSE mounting support
#[cfg(unix)] extern crate fuse;
#[cfg(unix)] extern crate libc;

pub mod sector;
pub mod disc;
pub mod decrypt;
#[cfg(unix)] pub mod mountvfs;
pub mod ird;

use std::fs::File;
use std::path::PathBuf;
use std::io::{BufReader, BufWriter, Write, Seek, SeekFrom};
use std::ffi::OsStr;
use std::sync::{Arc, Mutex};
use std::thread;
use std::sync::mpsc;
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

macro_rules! hex_println {
    ($a: expr) => {
        for &byte in $a {
            print!("{:02X}", byte);
        }
        println!();
    };
}

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
            //(@arg threads: -j --threads +takes_value "Number of threads to decrypt with. Defaults to 1. Set to 1 to switch to singlethreaded mode")
            (@arg irdfile: --ird +takes_value "IRD file to extract key from")
        ));
    }
    let matches = app.get_matches();
    match matches.subcommand() {
        ("info", Some(matches)) => {
            let f = File::open(matches.value_of("FILE").unwrap()).chain_err(|| "Failed to open file")?;
            let reader = BufReader::new(f);

            let disc = disc::PS3Disc::new(reader)?;
            if matches.is_present("id") {
                println!("{}", disc.gameid);
            } else if matches.is_present("keys") {
                if let Some(disc_key) = disc.disc_key {
                    if let Some(d1) = disc.d1 {
                        print!("      d1: ");
                        hex_println!(d1.as_ref());
                    }
                    print!("disc_key: ");
                    hex_println!(disc_key.as_ref());
                } else {
                    println!("No keys present");
                }
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
                if let Some(tagline) = disc.tagline_3k3y {
                    println!("3k3y tagline present: \"{}\"", tagline);
                }
            }
        },
        ("decrypt", Some(matches)) => {
            println!("input: {}", PathBuf::from(matches.value_of("FILE").unwrap()).display());
            let f = File::open(matches.value_of("FILE").unwrap()).chain_err(|| "Failed to open file")?;
            let reader = BufReader::new(f);

            let mut disc = disc::PS3Disc::new(reader)?;

            // Calculate output filename
            let output_path = if let Some(outfile) = matches.value_of("OUTFILE") {
                // User specified a file, so we do what they tell us
                PathBuf::from(outfile)
            } else {
                // No output specified
                let mut pathbuf = PathBuf::from(matches.value_of("FILE").unwrap());
                if pathbuf.extension() == Some(OsStr::new("iso")) {
                    // It's an .iso, so let's do orig.dec.iso
                    pathbuf.set_extension("dec.iso");
                } else {
                    // It's not an .iso, so let's do BCUS12345.dec.iso
                    pathbuf = PathBuf::from(format!("{}.dec.iso", disc.gameid.replace('-', "")));
                }
                pathbuf
            };

            println!("output: {}", output_path.display());
            let fout = File::create(output_path).chain_err(|| "Failed to create file")?;
            let mut writer = BufWriter::new(fout);

            if let Some(ird_path) = matches.value_of("irdfile") {
                let parsed = ird::read_ird(ird_path)?;
                disc.import_from_ird(parsed)?;
            }

            if matches.is_present("d1") && matches.is_present("key") {
                println!("warning: --key takes precedence over --d1");
            }
            // Check if the user passed us --d1 and/or --key, and set them accordingly
            if let Some(d1) = matches.value_of("d1") {
                let d1: Vec<u8> = FromHex::from_hex(d1.as_bytes().to_owned()).chain_err(|| "failed to parse key")?;
                disc.set_d1(d1.as_ref())?;
            }
            if let Some(key) = matches.value_of("key") {
                let disc_key: Vec<u8> = FromHex::from_hex(key.as_bytes().to_owned()).chain_err(|| "failed to parse key")?;
                disc.set_disc_key(disc_key.as_ref())?;
            }

            // Gracefully and helpfully fail if we don't have a disc_key
            // rather than throwing an error later because there's no disc_key.
            if !disc.can_decrypt() {
                println!("No 3k3y header found, and no d1, disc key, or ird file specified!");
                println!("Disc can't be decrypted without any of those.");
                println!("Consider passing a value to --d1 or --ird");
                return Ok(());
            }
            if let Some(d1) = disc.d1 {
                print!("using d1: ");
                hex_println!(d1.as_ref());
            }
            print!("{} disc key: ", if disc.d1.is_some() {"calculated"} else {"using"});
            hex_println!(disc.disc_key.unwrap().as_ref());


            println!("sectors: {sectors} ({size}), regions: {regions}",
                     sectors=disc.total_sectors,
                     size=ByteSize::b(disc.total_sectors as usize * 2048).to_string(true),
                     regions=disc.regions.len());

            // Start the actual decryption/ripping process

            let threads = matches.value_of("threads").unwrap_or("1").parse::<usize>().unwrap();
            if threads == 1 {
                // Singlethreaded Decrypt
                for i in 0..disc.total_sectors {
                    writer.write_all(disc.read_sector(i).chain_err(|| "failed to read something")?.as_ref()).chain_err(|| "failed to write something")?;
                    print!("\rsector: {}/{} ({}%)",
                           i,
                           disc.total_sectors,
                           ((i as f64)/(disc.total_sectors as f64)*100f64).floor()
                    );
                }
                println!();
            } else if threads > 1 {
                // Multithreaded Decrypt
                let total_sectors = disc.total_sectors;
                let decryptor = disc.get_decryptor().chain_err(|| "Failed to get standalone disc decryptor")?;
                let writer = Arc::new(Mutex::new(writer));
                let disc = Arc::new(Mutex::new((0u32, disc)));
                let (tx, rx) = mpsc::channel();

                for _ in 0..threads {
                    let (writer, disc, tx) = (writer.clone(), disc.clone(), tx.clone());
                    let decryptor = decryptor.clone();
                    thread::spawn(move || {
                        let mut encrypted: Vec<u8>; //TODO switch one or both of these to [u8; 2048]?
                        let mut decrypted: Vec<u8>;
                        let mut cur_sec: u32;
                        loop {
                            {
                                let (ref mut current_sector, ref mut disc) = *disc.lock().unwrap();
                                cur_sec = *current_sector;
                                if *current_sector >= disc.total_sectors {
                                    // Why are you doing this like this?
                                    // Well, we need to inform the main thread that we're done
                                    // But if this thread isn't the first to inform,
                                    // then the mpsc channel will have already been dropped,
                                    // causing a panic:
                                    // > thread '<unnamed>' panicked at 'called `Result::unwrap()`
                                    // > on an `Err` value: "SendError(..)"'
                                    // This throws out the result in a way rustc is happy with.
                                    // (no unused_must_use warning)
                                    match tx.send(true) {
                                        _ => break
                                    };
                                } else {
                                    tx.send(false).unwrap();
                                }
                                encrypted = disc.read_sector_raw(*current_sector).unwrap();
                                *current_sector += 1;
                            }
                            decrypted = decryptor.decrypt_sector(&encrypted, cur_sec).unwrap();
                            {
                                let mut writer = writer.lock().unwrap();
                                writer.seek(SeekFrom::Start(cur_sec as u64 * 2048)).unwrap();
                                writer.write_all(decrypted.as_ref()).unwrap();
                            }
                        }
                    });
                }

                let mut progress = 0;
                while rx.recv().unwrap() != true {
                    progress += 1;
                    print!("\rsector: {}/{} ({}%)",
                           progress,
                           total_sectors,
                           ((progress as f64) / (total_sectors as f64) * 100f64).floor()
                    );
                }
                println!();
            } else {
                println!("must specify a -j/--threads value of 1 or more");
            }
        },
        #[cfg(unix)]
        ("mount", Some(matches)) => {
            println!("disc: {}", PathBuf::from(matches.value_of("FILE").unwrap()).display());
            let f = File::open(matches.value_of("FILE").unwrap()).chain_err(|| "Failed to open file")?;
            let reader = BufReader::new(f);

            let mut disc = disc::PS3Disc::new(reader)?;

            if let Some(ird_path) = matches.value_of("irdfile") {
                let parsed = ird::read_ird(ird_path)?;
                disc.import_from_ird(parsed)?;
            }

            if matches.is_present("d1") && matches.is_present("key") {
                println!("warning: --key takes precedence over --d1");
            }
            // Check if the user passed us --d1 and/or --key, and set them accordingly
            if let Some(d1) = matches.value_of("d1") {
                let d1: Vec<u8> = FromHex::from_hex(d1.as_bytes().to_owned()).chain_err(|| "failed to parse key")?;
                disc.set_d1(d1.as_ref())?;
            }
            if let Some(key) = matches.value_of("key") {
                let disc_key: Vec<u8> = FromHex::from_hex(key.as_bytes().to_owned()).chain_err(|| "failed to parse key")?;
                disc.set_disc_key(disc_key.as_ref())?;
            }

            mountvfs::mount(disc, matches.value_of("MOUNTPOINT").unwrap(), matches.is_present("verbose"));
        },
        ("irdinfo", Some(matches)) => {
            println!("file: {}", PathBuf::from(matches.value_of("FILE").unwrap()).display());
            let parsed = ird::read_ird(matches.value_of("FILE").unwrap())?;
            if matches.is_present("filehashes") {
                for hash in parsed.file_hashes {
                    print!("{}: ", hash.0);
                    hex_println!(hash.1.as_ref());
                }
            } else {
                println!("IRDv{} file for {} - {}", parsed.version, parsed.game_id, parsed.game_name);
                println!("versions: {} game, {} app, {} update", parsed.game_ver, parsed.app_ver, parsed.update_ver);

                print!("data1: ");
                hex_println!(parsed.data1.as_ref());
                print!("data2: ");
                hex_println!(parsed.data2.as_ref());

                println!();
                println!("Region MD5 hashes:");
                let mut i = 0;
                for hash in parsed.region_hashes {
                    print!("Region {}: ", i);
                    hex_println!(hash.as_ref());
                    i += 1;
                }
            }

        }
        (_, _) => unreachable!()
    }
    Ok(())
}