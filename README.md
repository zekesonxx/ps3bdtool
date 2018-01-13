# ps3bdtool

ps3bdtool is a (reasonably) simple tool to decrypt Sony PS3 game discs written in Rust.

This tool is built with the express and sole purpose of running games in the rpcs3 emulator.
I don't know how easy or hard it is to make a decrypted rip run on a real PS3. I don't care.

This tool technically allows piracy. Considering that PS3 games have been pirated for years using 
3k3y's ripper, cracked PS3 firmware, and other methods; this isn't bringing anything new to the table.


## Building
Standard `cargo build --release`. The debug build has `-O2` set, as without it it's painfully slow.

## Overview
```text
ps3bdtool 0.1.0
Tool to manipulate PS3 game discs

USAGE:
    ps3bdtool [SUBCOMMAND]

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

SUBCOMMANDS:
    decrypt    Decrypt a game iso
    help       Prints this message or the help of the given subcommand(s)
    info       Print information about a disc
    irdinfo    Print information about a 3k3y IRD file
    mount      Use FUSE to mount a filesystem containing a transparently-decrypted iso
```

## Usage
### Decrypting a disc
1. Insert the disc into your Blu-ray drive
2. choose your OS:
   * Windows: You'll need to rip the game to an iso file using an external tool, we'll refer to it as `$GAMEDISC`
   * Linux: You can read straight off the BR drive by using `/dev/sr0`. Or you can rip to an iso first. Either way works.
   Whichever way you choose, iso or `/dev/sr0`, we'll refer to it as `$GAMEDISC`
3. Run `ps3bdtool info $GAMEDISC`. This will get you the game ID
4. Now, you'll need the disc's IRD file, which you can get from [the usual suspect](http://jonnysp.bplaced.net/).
5. Run `ps3bdtool decrypt --ird path/to/the/irdfile.ird $GAMEDISC`. This will decrypt the disc to a file.
   * I'd recommend adding `-j3` to the options to do a multithreaded (3 threads) decrypt instead. Much faster.
6. Extract the game in your archive manager of choice, and play away!

### Fixing broken pirated rips
No. Go buy the game legally. Most PS3 games are pretty cheap these days.

### Running games straight off the disc
This is a conveluted process because there isn't a good ISO9660 implementation in Rust,
and I can't be arsed to write one yet.

**NOTE** Linux only. Probably works with macOS, but rpcs3 doesn't, so \*shrug\*.

1. Get an IRD file for the disc, see above.
2. Make an empty folder somewhere, we'll call it `$MOUNTPOINT`.
   * We won't actually store anything here, but it has to exist and be empty.
3. Make an empty folder in `$RPCS3/dev_hdd0/disc`, ex `$RPCS3/dev_hdd0/disc/discgame` 
3. Run `ps3bdtool mount --ird path/to/the/irdfile.ird /dev/sr0 $MOUNTPOINT`  
4. Then, to actually mount the game, run `fuseiso $MOUNTPOINT/GameDisc.iso $RPCS3/dev_hdd0/disc/discgame`
5. Open rpcs3 and run your game!

I've tested this with several games and it seems to work alright.
It's convoluted and annoying, but it works.

You'll need to `fusermount -u` the fuseiso mount, and then `fusermount -u` the game disc mount.

If you remove a disc while things are running something will probably implode.

### How ps3bdtool finds decryption keys
ps3bdtool goes through a chain to find the decryption key, that is as follows:

1. A 3k3y-injected header on the disc, at the end of the second sector.
2. `--ird`, `--key`, or `--d1` options passed on the command line, with precedence in that order.
3. Looking for an IRD file containing the game ID in `$XDG_DATA_HOME/ps3bdtool/ird_files`
  * So, if you're trying to decrypt an American Red Dead Redemption release, it'll look for any file containing `BLUS30418` in the filename.
    If you got the IRD file from jonnysp, this file will be named `BLUS30418-501E79332EEF57D0B64186826CD15D65.ird`. 


## Misc Notes
* ps3bdtool is built on the assumption that the bulk of a PS3 disc will be encrypted,
  and as such priority should be given to decryption speed, not raw transfer speed.
  Meaning, if you have a disc that is largely unencrypted,
  ps3bdtool will be slower than dd or similar due to ps3bdtool's excessive buffering. 
  As only the basic disc info and update files are unencrypted on basically every retail
  disc, this assumption is probably a reasonable one.
* The FUSE mount has no explicit caching at all. We're relying on the kernel to avoid excessive reads here.
* I suggest 3 threads for decryption above because 3 threads is enough that, with my (reasonably old) quad-core
  i5-2320 reading from an LG WH16NS40 and writing to an SSD, I/O speed becomes the bottleneck.
* FUSE mounting doesn't support multithreaded decryption, and probably won't ever because I don't care enough. 


## License
GPLv3.