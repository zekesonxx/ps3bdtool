
use std::fs::File;
use std::path::PathBuf;
use std::ffi::OsStr;
use std::io::{BufReader, BufWriter, Write, Seek, SeekFrom};
use std::sync::{mpsc, Arc, Mutex};
use std::thread;
use bytesize::ByteSize;

use super::super::errors::*;
use super::super::disc;

pub fn decrypt_disc(matches: &::clap::ArgMatches) -> Result<()> {
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


    if !super::find_key_if_possible(&mut disc, matches).chain_err(||"Failed to try and find a key")? && !disc.can_decrypt() {
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

    // Check to make sure we have enough free disk space
    #[cfg(unix)]
    {
        use nix::sys::statvfs::fstatvfs;
        let statvfs = fstatvfs(&fout).chain_err(|| "failed to check disk space")?;
        let free_space = (statvfs.blocks() * statvfs.block_size()) as usize;
        let needed_space = disc.total_sectors as usize * 2048;
        if free_space < needed_space {
            bail!("need {need} bytes free ({needf}), only have {have} bytes ({havef})",
                need=needed_space, needf=ByteSize::b(needed_space).to_string(true),
                have=free_space, havef=ByteSize::b(free_space).to_string(true)
        );
            //::std::process::exit(1);
        }
    }

    // Start the actual decryption/ripping process

    let mut writer = BufWriter::new(fout);
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
            let (writer, disc, tx) = (Arc::clone(&writer), Arc::clone(&disc), tx.clone());
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
    Ok(())
}