use sha1::{Digest, Sha1};
use std::fs::File;
use std::io::{BufReader, Read};
use std::path::Path;
pub fn sha1sum(filepath: &Path) -> Result<String, Box<dyn std::error::Error>> {
    let f = File::open(&filepath)?;
    let mut reader = BufReader::new(f);
    let mut buffer = Vec::new();

    // Read file into vector.
    reader.read_to_end(&mut buffer)?;

    let mut hasher = Sha1::new();
    hasher.update(&buffer);
    let hexes = hasher.finalize();
    let mut out = String::new();
    for hex in hexes {
        out.push_str(&format!("{:02x?}", hex));
    }
    Ok(out)
}
