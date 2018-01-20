
use std::fs::File;
use std::io::BufReader;
use std::path::PathBuf;
use bytesize::ByteSize;

use super::super::errors::*;
use super::super::{disc, ird};

pub fn disc_info(matches: &::clap::ArgMatches) -> Result<()> {
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
    Ok(())
}

pub fn ird_info(matches: &::clap::ArgMatches) -> Result<()> {
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
    Ok(())
}