use fuse::{self, Filesystem, FileAttr, FileType, Request, ReplyAttr, ReplyDirectory, ReplyEntry, ReplyData};
use disc::PS3Disc;
use std::io::{Read, Seek};
use std::path::Path;
use std::ffi::OsStr;
use libc::{ENOENT};
use time::Timespec;

struct DecryptFilesystem<F> {
    disc: PS3Disc<F>,
    root_attr: FileAttr,
    iso_attr: FileAttr,
    verbose: bool
}

impl<F: Read+Seek> DecryptFilesystem<F> {
    fn new (disc: PS3Disc<F>, verbose: bool) -> Self {
        let ts = Timespec::new(0, 0);
        let root_attr = FileAttr {
            ino: 1,
            size: 0,
            blocks: 0,
            atime: ts,
            mtime: ts,
            ctime: ts,
            crtime: ts,
            kind: FileType::Directory,
            perm: 0o555,
            nlink: 0,
            uid: 0,
            gid: 0,
            rdev: 0,
            flags: 0,
        };
        let iso_attr = FileAttr {
            ino: 2,
            size: disc.total_sectors as u64*2048,
            blocks: disc.total_sectors as u64,
            atime: ts,
            mtime: ts,
            ctime: ts,
            crtime: ts,
            kind: FileType::RegularFile,
            perm: 0o555,
            nlink: 0,
            uid: 0,
            gid: 0,
            rdev: 0,
            flags: 0,
        };
        DecryptFilesystem {
            disc, root_attr, iso_attr, verbose
        }
    }
}

impl<F: Read+Seek> Filesystem for DecryptFilesystem<F> {
    fn getattr(&mut self, _req: &Request, ino: u64, reply: ReplyAttr) {
        //println!("getattr(ino={})", ino);
        let ttl = Timespec::new(1000, 0);
        if ino == 1 {
            reply.attr(&ttl, &self.root_attr);
        } else if ino == 2 {
            reply.attr(&ttl, &self.iso_attr);
        } else {
            reply.error(ENOENT);
        }
    }
    fn readdir(&mut self, _req: &Request, ino: u64, fh: u64, offset: i64, mut reply: ReplyDirectory) {
        if ino != 1 && fh != 0 && offset != 0 {
            if self.verbose {
                println!("readdir(ino={}, fh={}, offset={})", ino, fh, offset);
            }
        }
        if ino == 1 {
            if offset == 0 {
                reply.add(1, 0, FileType::Directory, &Path::new("."));
                reply.add(1, 1, FileType::Directory, &Path::new(".."));
                reply.add(2, 2, FileType::RegularFile, &Path::new("GameDisc.iso"));
            }
            reply.ok();
        } else {
            reply.error(ENOENT);
        }
    }
    fn lookup(&mut self, _req: &Request, parent: u64, name: &OsStr, reply: ReplyEntry) {
        if self.verbose {
            println!("lookup(parent={}, name={:?})", parent, name);
        }
        if name.to_str().unwrap() == "/" {
            let ttl = Timespec::new(1, 0);
            reply.entry(&ttl, &self.root_attr, 0);
        } else if name.to_str().unwrap() == "GameDisc.iso" {
            let ttl = Timespec::new(1, 0);
            reply.entry(&ttl, &self.iso_attr, 0);
        } else {
            reply.error(ENOENT);
        }
    }
    fn read(&mut self, _req: &Request, ino: u64, fh: u64, offset: i64, size: u32, reply: ReplyData) {
        if self.verbose {
            println!("read(ino={}, fh={}, offset={}, size={})", ino, fh, offset, size);
        }
        if ino != 2 {
            reply.error(ENOENT);
            return;
        }
        let mut return_buf: Vec<u8> = Vec::with_capacity(size as usize+2048);
        let starting_sector = offset/2048;
        let offset_from_start = offset%2048;
        let ending_sector = (offset+size as i64)/2048;
        if self.verbose {
            println!("offset: {}, size: {}, starting: {}, offset_from_start: {}, ending: {}",
                     offset, size, starting_sector, offset_from_start, ending_sector);
        }
        for i in starting_sector..ending_sector {
            return_buf.append(&mut self.disc.read_sector(i as u32).unwrap());
        }
        reply.data(&return_buf[offset_from_start as usize..(offset_from_start as usize+size as usize)]);
    }
}

pub fn mount<F: Read+Seek, P: AsRef<Path>>(disc: PS3Disc<F>, mountpoint: P, verbose: bool) {
    fuse::mount(DecryptFilesystem::new(disc, verbose), &mountpoint, &[]).unwrap();
}