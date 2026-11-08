use crate::model::Item;
use crate::table::zbin::{header_zone, record, write, zone, RECORD};
use crate::{Error, Format, Result};

pub struct ZNew;

pub const MODES: &[&str] = &["CW", "SSB", "FM", "AM", "RTTY", "FT4", "FT8", "Other", "DV"];

const HEAD: usize = 384;
const EXTRA: usize = 128;
const ZLOX: &[u8] = b"ZLOX";

impl Format for ZNew {
    fn kind(&self) -> &'static str {
        "znew"
    }

    fn name(&self) -> &'static str {
        "zLog (ZLOX)"
    }

    fn extensions(&self) -> &'static [&'static str] {
        &["zlox"]
    }

    fn decode(&self, bytes: &[u8]) -> Result<Vec<Item>> {
        if bytes.len() < HEAD || &bytes[..4] != ZLOX {
            return Err(Error::new("malformed ZLOX sequence"));
        }
        let count = i32::from_le_bytes(bytes[4..8].try_into().expect("4 bytes"));
        let count = usize::try_from(count).map_err(|_| Error::new("malformed ZLOX count"))?;
        let offset = zone(&bytes[0x54..0x56]);
        let (records, _) = bytes[HEAD..].as_chunks::<{ RECORD + EXTRA }>();
        if records.len() < count {
            return Err(Error::new("truncated ZLOX record"));
        }
        let items = records[..count].iter().map(|r| record(r, offset, MODES));
        Ok(items.collect())
    }

    fn encode(&self, items: &[Item]) -> Result<Vec<u8>> {
        let mut out = ZLOX.to_vec();
        out.extend((items.len() as i32).to_le_bytes());
        out.resize(0x54, 0);
        out.extend(header_zone());
        out.resize(HEAD, 0);
        for (id, item) in items.iter().enumerate() {
            out.extend(write(item, id, MODES));
            out.extend([0; EXTRA]);
        }
        Ok(out)
    }
}
