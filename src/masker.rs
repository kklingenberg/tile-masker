use image::{io::Reader as ImageReader, Pixel};
use std::collections::BTreeSet;
use std::io::{Cursor, Error, ErrorKind};
use std::path::PathBuf;

/// Process a file, masking any color found in the given mask with
/// transparency.
pub fn process(file: PathBuf, mask: BTreeSet<u32>) -> Result<Vec<u8>, Error> {
    let mut img = ImageReader::open(file)?
        .decode()
        .map_err(|e| Error::new(ErrorKind::InvalidData, e))?
        .into_rgba8();
    for pixel in img.pixels_mut() {
        let data = pixel.channels_mut();
        let color = ((data[0] as u32) << 16) + ((data[1] as u32) << 8) + (data[2] as u32);
        if mask.contains(&color) {
            data[0] = 0;
            data[1] = 0;
            data[2] = 0;
            data[3] = 0;
        }
    }
    let mut bytes: Vec<u8> = Vec::new();
    img.write_to(&mut Cursor::new(&mut bytes), image::ImageFormat::Png)
        .map_err(|e| Error::new(ErrorKind::InvalidData, e))?;
    Ok(bytes)
}
