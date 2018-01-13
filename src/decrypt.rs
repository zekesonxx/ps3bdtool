
use super::errors::*;
use crypto::{buffer, aes, blockmodes};
use crypto::buffer::{ReadBuffer, WriteBuffer, BufferResult};

/// High-level, simple function to do an AES128 CBC encrypt using rust-crypto
pub fn aes_encrypt(data: &[u8], key: &[u8], iv: &[u8]) -> Result<Vec<u8>> {
    let mut encryptor = aes::cbc_encryptor(
        aes::KeySize::KeySize128,
        key,
        iv,
        blockmodes::NoPadding);

    let mut final_result = Vec::<u8>::new();
    let mut read_buffer = buffer::RefReadBuffer::new(data);
    let mut buffer = [0; 2048];
    let mut write_buffer = buffer::RefWriteBuffer::new(&mut buffer);

    loop {
        let result = match encryptor.encrypt(&mut read_buffer, &mut write_buffer, true) {
            Ok(k) => k,
            Err(e) => return Err(Error::from_kind(ErrorKind::SymmetricCipherError(e)))
        };

        final_result.extend(write_buffer.take_read_buffer().take_remaining().iter().cloned());

        match result {
            BufferResult::BufferUnderflow => break,
            BufferResult::BufferOverflow => { }
        }
    }

    Ok(final_result)
}

/// High-level, simple function to do an AES128 CBC decrypt using rust-crypto
pub fn aes_decrypt(encrypted_data: &[u8], key: &[u8], iv: &[u8]) -> Result<Vec<u8>> {
    let mut decryptor = aes::cbc_decryptor(
        aes::KeySize::KeySize128,
        key,
        iv,
        blockmodes::NoPadding);

    let mut final_result = Vec::<u8>::new();
    let mut read_buffer = buffer::RefReadBuffer::new(encrypted_data);
    let mut buffer = [0; 2048];
    let mut write_buffer = buffer::RefWriteBuffer::new(&mut buffer);

    loop {
        let result = match decryptor.decrypt(&mut read_buffer, &mut write_buffer, true) {
            Ok(k) => k,
            Err(e) => return Err(Error::from_kind(ErrorKind::SymmetricCipherError(e)))
        };
        final_result.extend(write_buffer.take_read_buffer().take_remaining().iter().cloned());
        match result {
            BufferResult::BufferUnderflow => break,
            BufferResult::BufferOverflow => { }
        }
    }

    Ok(final_result)
}

/// Calculate a disc encryption key, given the disc's d1
#[allow(non_upper_case_globals)]
pub fn disc_key(d1: &[u8]) -> Result<Vec<u8>> {
    //keys obtained from PS3DevWiki
    // key_2:   380BCF0B53455B3C7817AB4FA3BA90ED
    const key: [u8; 16] = [56, 11, 207, 11, 83, 69, 91, 60, 120, 23, 171, 79, 163, 186, 144, 237];
    // iv_2:    69474772AF6FDAB342743AEFAA186287
    const iV: [u8; 16] = [105, 71, 71, 114, 175, 111, 218, 179, 66, 116, 58, 239, 170, 24, 98, 135];
    aes_encrypt(d1, &key, &iV)
}