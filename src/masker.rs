use image::{
    codecs::png::{CompressionType, FilterType, PngEncoder},
    io::Reader as ImageReader,
    ImageFormat, Pixel, RgbaImage,
};
use std::collections::BTreeMap;
use std::io::{Cursor, Error, ErrorKind, Read};
use std::path::PathBuf;
use std::sync::OnceLock;
use std::time::Duration;
use ureq::{Agent, AgentBuilder};
use url::Url;

/// Generate a masked image according to the given mask.
fn mask_image(
    img: &mut RgbaImage,
    mask: BTreeMap<u32, (u8, u8, u8, u8)>,
) -> Result<Vec<u8>, Error> {
    for pixel in img.pixels_mut() {
        let data = pixel.channels_mut();
        let color = ((data[0] as u32) << 16) + ((data[1] as u32) << 8) + (data[2] as u32);
        if let Some((r, g, b, a)) = mask.get(&color) {
            data[0] = *r;
            data[1] = *g;
            data[2] = *b;
            data[3] = *a;
        }
    }
    let mut bytes: Vec<u8> = Vec::new();
    img.write_with_encoder(PngEncoder::new_with_quality(
        &mut Cursor::new(&mut bytes),
        CompressionType::Best,
        FilterType::NoFilter,
    ))
    .map_err(|e| Error::new(ErrorKind::InvalidData, e))?;
    Ok(bytes)
}

static AGENT: OnceLock<Agent> = OnceLock::new();

const USER_AGENT: &str = concat!(env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION"));

const MAX_REMOTE_FILE_SIZE: u64 = 10_485_760; // 10 MB

/// Process a remote file, masking any color found in the given mask
/// with transparency.
pub fn process_remote(url: Url, mask: BTreeMap<u32, (u8, u8, u8, u8)>) -> Result<Vec<u8>, Error> {
    let agent = AGENT.get_or_init(|| {
        AgentBuilder::new()
            .timeout(Duration::from_secs(30))
            .user_agent(USER_AGENT)
            .build()
    });
    let response = agent
        .get(url.as_str())
        .call()
        .map_err(|e| Error::new(ErrorKind::NotFound, e))?;
    let mut input_bytes: Vec<u8> = Vec::new();
    response
        .into_reader()
        .take(MAX_REMOTE_FILE_SIZE)
        .read_to_end(&mut input_bytes)?;

    let mut img_buffer = Cursor::new(&mut input_bytes);
    let mut img = ImageReader::with_format(&mut img_buffer, ImageFormat::Png)
        .decode()
        .map_err(|e| Error::new(ErrorKind::InvalidData, e))?
        .into_rgba8();
    mask_image(&mut img, mask)
}

/// Process a local file, masking any color found in the given mask
/// with transparency.
pub fn process_local(
    file: PathBuf,
    mask: BTreeMap<u32, (u8, u8, u8, u8)>,
) -> Result<Vec<u8>, Error> {
    let mut img = ImageReader::open(file)?
        .decode()
        .map_err(|e| Error::new(ErrorKind::InvalidData, e))?
        .into_rgba8();
    mask_image(&mut img, mask)
}
