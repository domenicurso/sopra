use super::{CACHE_LIMIT, Source};

const MAGIC: &[u8; 8] = b"SOPRACCH";

pub(super) fn encode(sources: &[Source]) -> Option<Vec<u8>> {
    let mut unique: Vec<&Source> = Vec::new();
    for source in sources {
        if !unique.contains(&source) {
            unique.push(source);
        }
    }
    let mut payload = Vec::new();
    for source in unique {
        let (tag, text) = match source {
            Source::Help(text) => (1, text),
            Source::Man(text) => (2, text),
        };
        let length = u32::try_from(text.len()).ok()?;
        if payload.len() + text.len() + 21 > CACHE_LIMIT as usize {
            return None;
        }
        payload.push(tag);
        payload.extend(length.to_le_bytes());
        payload.extend(text.as_bytes());
    }
    let mut bytes = Vec::with_capacity(payload.len() + 16);
    bytes.extend(MAGIC);
    bytes.extend(checksum(&payload).to_le_bytes());
    bytes.extend(payload);
    Some(bytes)
}

pub(super) fn decode(bytes: &[u8]) -> Option<Vec<Source>> {
    if bytes.len() > CACHE_LIMIT as usize {
        return None;
    }
    let body = bytes.strip_prefix(MAGIC)?;
    let expected = u64::from_le_bytes(body.get(..8)?.try_into().ok()?);
    let mut records = body.get(8..)?;
    if checksum(records) != expected {
        return None;
    }
    let mut sources = Vec::new();
    while !records.is_empty() {
        let (&tag, rest) = records.split_first()?;
        let length = u32::from_le_bytes(rest.get(..4)?.try_into().ok()?) as usize;
        let body = rest.get(4..)?;
        let text = body.get(..length)?;
        records = body.get(length..)?;
        let source = match tag {
            1 => Source::Help(String::from_utf8(text.to_vec()).ok()?),
            2 => Source::Man(String::from_utf8(text.to_vec()).ok()?),
            _ => continue, // Length framing allows future source kinds without a schema switch.
        };
        sources.push(source);
    }
    Some(sources)
}

// FNV-1a detects damaged or truncated local bundles; this is not an authenticity check.
fn checksum(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf29ce484222325, |hash, byte| {
        (hash ^ u64::from(*byte)).wrapping_mul(0x100000001b3)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn source_records_round_trip_unicode_and_former_delimiters_without_duplication() {
        let text = "help π\n\x1eSOPRA_COMPLETION_SOURCE\x1e\n".to_string();
        let sources = vec![
            Source::Help(text.clone()),
            Source::Help(text.clone()),
            Source::Man("manual".into()),
        ];
        let encoded = encode(&sources).expect("small bundle");
        assert_eq!(
            decode(&encoded),
            Some(vec![Source::Help(text), Source::Man("manual".into())])
        );
        assert!(
            encoded.len()
                < sources
                    .iter()
                    .map(|source| match source {
                        Source::Help(text) | Source::Man(text) => text.len() + 32,
                    })
                    .sum::<usize>()
        );
    }

    #[test]
    fn corrupt_truncated_and_oversized_records_are_rejected() {
        let encoded =
            encode(&[Source::Help("Commands:\n  build  Build".into())]).expect("small bundle");
        for length in 0..encoded.len() {
            assert!(decode(&encoded[..length]).is_none());
        }
        let mut damaged = encoded.clone();
        damaged[20] ^= 0x40;
        assert!(decode(&damaged).is_none());
        assert!(encode(&[Source::Help("x".repeat(CACHE_LIMIT as usize))]).is_none());
    }

    #[test]
    fn unknown_source_kinds_can_be_skipped_without_rejecting_known_records() {
        let mut bytes = encode(&[Source::Help("help".into()), Source::Man("manual".into())])
            .expect("small bundle");
        bytes[16] = 99;
        let checksum = checksum(&bytes[16..]).to_le_bytes();
        bytes[8..16].copy_from_slice(&checksum);
        assert_eq!(decode(&bytes), Some(vec![Source::Man("manual".into())]));
    }
}
