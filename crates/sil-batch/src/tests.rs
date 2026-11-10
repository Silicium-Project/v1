use super::*;

fn header(kind: Kind) -> Header {
    Header {
        kind,
        format: 0,
        width: 4,
        height: 3,
        count: 0,
        options: Options::default(),
    }
}

/// Un lot version 1 écrit à la main : pas de zone d'options.
fn version_1(kind: u8, records: &[&[u8]]) -> Vec<u8> {
    let mut out = b"SILBATCH".to_vec();
    out.extend_from_slice(&[1, kind, 0, 0]);
    out.extend_from_slice(&4u32.to_le_bytes());
    out.extend_from_slice(&3u32.to_le_bytes());
    out.extend_from_slice(&(records.len() as u32).to_le_bytes());
    for record in records {
        out.extend_from_slice(&(record.len() as u32).to_le_bytes());
        out.extend_from_slice(record);
    }
    out
}

#[test]
fn a_version_1_batch_still_reads_with_default_options() {
    let bytes = version_1(1, &[b"abc"]);
    let batch = parse(&bytes).unwrap();
    assert_eq!(batch.header.kind, Kind::Image);
    assert_eq!(batch.header.options, Options::default());
    assert_eq!(batch.records, vec![b"abc".as_slice()]);
}

#[test]
fn options_are_written_and_read_back() {
    let options = Options {
        quality: 70,
        fit: FIT_CONTAIN,
        rotate: 1,
        flags: FLAG_GRAYSCALE,
        ..Options::default()
    };
    let bytes = build(
        Header {
            options,
            ..header(Kind::Image)
        },
        &[],
    )
    .unwrap();
    assert_eq!(parse(&bytes).unwrap().header.options, options);
}

#[test]
fn an_invalid_option_is_refused_when_building_and_when_reading() {
    let bad = Options {
        channels: 2,
        ..Options::default()
    };
    assert!(
        build(
            Header {
                options: bad,
                ..header(Kind::Image)
            },
            &[]
        )
        .is_err()
    );
    let mut bytes = build(header(Kind::Image), &[]).unwrap();
    bytes[36] = 2; // canaux, à l'offset 36 : la zone d'options commence à 24
    assert_eq!(
        parse(&bytes),
        Err(BatchError::BadOption("option audio sur un lot d'images"))
    );
}

#[test]
fn a_reserved_option_byte_must_be_zero() {
    let mut bytes = build(header(Kind::Image), &[]).unwrap();
    bytes[37] = 1;
    assert!(matches!(parse(&bytes), Err(BatchError::BadOption(_))));
}

#[test]
fn a_built_batch_reads_back_the_same_records() {
    let records = vec![b"un".to_vec(), Vec::new(), b"trois".to_vec()];
    let bytes = build(
        Header {
            count: 3,
            ..header(Kind::Image)
        },
        &records,
    )
    .unwrap();
    let batch = parse(&bytes).unwrap();
    assert_eq!(batch.header.kind, Kind::Image);
    assert_eq!((batch.header.width, batch.header.height), (4, 3));
    assert_eq!(batch.header.count, 3);
    let read: Vec<&[u8]> = records.iter().map(Vec::as_slice).collect();
    assert_eq!(batch.records, read);
}

#[test]
fn a_wrong_magic_or_version_is_refused() {
    let mut bytes = build(header(Kind::Audio), &[]).unwrap();
    bytes[0] = b'X';
    assert_eq!(parse(&bytes), Err(BatchError::Magic));
    let mut bytes = build(header(Kind::Audio), &[]).unwrap();
    bytes[8] = 9;
    assert_eq!(parse(&bytes), Err(BatchError::Version(9)));
}

#[test]
fn an_unknown_kind_is_refused() {
    let mut bytes = build(header(Kind::Audio), &[]).unwrap();
    bytes[9] = 7;
    assert_eq!(parse(&bytes), Err(BatchError::Kind(7)));
}

#[test]
fn a_truncated_or_padded_batch_is_refused() {
    let bytes = build(header(Kind::Image), &[b"abc".to_vec()]).unwrap();
    assert_eq!(parse(&bytes[..bytes.len() - 1]), Err(BatchError::Truncated));
    let mut padded = bytes.clone();
    padded.push(0);
    assert_eq!(parse(&padded), Err(BatchError::TrailingBytes));
}

#[test]
fn a_count_above_the_limit_is_refused_before_reading_records() {
    let mut bytes = build(header(Kind::Image), &[]).unwrap();
    bytes[20..24].copy_from_slice(&(MAX_RECORDS + 1).to_le_bytes());
    assert_eq!(
        parse(&bytes),
        Err(BatchError::TooManyRecords(MAX_RECORDS + 1))
    );
}

#[test]
fn ranges_are_checked_against_the_record_count() {
    assert_eq!(parse_range(&range_params(1, 3), 3), Ok((1, 3)));
    assert_eq!(
        parse_range(&range_params(2, 1), 3),
        Err(BatchError::BadRange)
    );
    assert_eq!(
        parse_range(&range_params(0, 4), 3),
        Err(BatchError::BadRange)
    );
    assert_eq!(parse_range(&[1, 2, 3], 3), Err(BatchError::BadRange));
}

#[test]
fn a_frame_reads_back_the_same_parts() {
    let parts = vec![b"x".to_vec(), Vec::new(), b"yz".to_vec()];
    assert_eq!(unframe(&frame(&parts).unwrap()).unwrap(), parts);
    assert_eq!(unframe(&[]).unwrap(), Vec::<Vec<u8>>::new());
}

#[test]
fn a_frame_with_a_length_past_its_end_is_refused() {
    let mut bytes = frame(&[b"abc".to_vec()]).unwrap();
    bytes.pop();
    assert_eq!(unframe(&bytes), Err(BatchError::BadFrame));
}
