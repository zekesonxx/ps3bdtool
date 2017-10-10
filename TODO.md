# TODO

### Infrastructure
* Multithreaded decrypts as a module
* 3k3y IRD files
  * Parsing, print info, check crc32
  * Getting d1 keys out
  * Checking an ISO and/or folder against the hash list
* Move sector decryption checking to multithreading code,
  avoid pointless Vec allocation in decrypt_sector for unencrypted sectors
* Load IRD files or decryption bins from a cache/config dir
* Fetch IRD files straight from jonnysp
* Resume partial rip/decrypt
* In-place decrypt? Is this even practical?
* Less taxing progress reports?

### GUI
* Separate binary? Probably.
* Basic GTK+3 decryptor GUI
  * Doing it at all, with multithreading
  * Progress bar

### Straight-off-disc-playback
* ~~Stage 1: FUSE presenting disc as a .iso, transparently decrypt~~ **DONE!** 
* Stage 2: FUSE presenting the disc's ISO9660 filesystem, transparently decrypt
  * maybe just start with a standalone ISO9660 read-only FUSE driver?
* Stage 3: stage 2 but automatically detecting changing discs
* wtf even is the Windows solution to this, short of integration into rpcs3?