use std::io::{Read, Write};
use flate2::read::GzDecoder;

pub fn schem_to_schematic<R: Read, W: Write>(input: R, mut output: W) -> Result<(), Box<dyn std::error::Error>> {
    // Decompress the gzipped input
    let mut decoder = GzDecoder::new(input);

    // Copy the decompressed data directly to the output
    std::io::copy(&mut decoder, &mut output)?;

    Ok(())
}

