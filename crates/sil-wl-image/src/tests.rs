use crate::{FORMAT_JPEG, FORMAT_PNG, ImageBatchWorkload, codec::process};
use image::{DynamicImage, ImageFormat, RgbImage};
use sil_batch::{
    FIT_CONTAIN, FIT_COVER, FLAG_GRAYSCALE, Header, Kind, Options, build, frame, unframe,
};
use sil_crypto::sha256;
use sil_workload_api::{MemoryInput, Output, Verdict, Workload, WorkloadError};
use std::io::Cursor;

fn png(width: u32, height: u32, shade: u8) -> Vec<u8> {
    let image = RgbImage::from_fn(width, height, |x, y| {
        image::Rgb([shade, (x % 256) as u8, (y % 256) as u8])
    });
    let mut out = Cursor::new(Vec::new());
    DynamicImage::ImageRgb8(image)
        .write_to(&mut out, ImageFormat::Png)
        .unwrap();
    out.into_inner()
}

fn batch(width: u32, height: u32, format: u8, records: &[Vec<u8>]) -> Vec<u8> {
    build(
        Header {
            kind: Kind::Image,
            format,
            width,
            height,
            count: records.len() as u32,
            options: Options::default(),
        },
        records,
    )
    .unwrap()
}

fn decoded_size(bytes: &[u8]) -> (u32, u32) {
    let image = image::load_from_memory(bytes).unwrap();
    (image.width(), image.height())
}

#[test]
fn fragments_cover_every_image_once() {
    let records: Vec<Vec<u8>> = (0..5).map(|i| png(4, 4, i)).collect();
    let bytes = batch(2, 2, FORMAT_PNG, &records);
    let specs = ImageBatchWorkload.split(&MemoryInput(&bytes), 3).unwrap();
    assert_eq!(specs.len(), 3);
    let mut seen = 0;
    for spec in &specs {
        let output = ImageBatchWorkload
            .execute(&MemoryInput(&bytes), spec)
            .unwrap();
        seen += unframe(&output.result).unwrap().len();
    }
    assert_eq!(seen, 5);
}

#[test]
fn more_fragments_than_images_leaves_the_extra_ones_empty() {
    let records = vec![png(4, 4, 1), png(4, 4, 2)];
    let bytes = batch(0, 0, FORMAT_PNG, &records);
    let input = MemoryInput(&bytes);
    let specs = ImageBatchWorkload.split(&input, 3).unwrap();
    assert_eq!(specs.len(), 3);
    let mut sizes = Vec::new();
    for spec in &specs {
        let output = ImageBatchWorkload.execute(&input, spec).unwrap();
        sizes.push(unframe(&output.result).unwrap().len());
    }
    assert_eq!(sizes, vec![1, 1, 0]);
}

#[test]
fn the_result_is_the_resized_images_in_order() {
    let records = vec![png(8, 6, 10), png(3, 9, 200)];
    let bytes = batch(4, 2, FORMAT_PNG, &records);
    let specs = ImageBatchWorkload.split(&MemoryInput(&bytes), 1).unwrap();
    let output = ImageBatchWorkload
        .execute(&MemoryInput(&bytes), &specs[0])
        .unwrap();
    let parts = unframe(&output.result).unwrap();
    assert_eq!(parts.len(), 2);
    for part in &parts {
        assert_eq!(decoded_size(part), (4, 2));
    }
}

#[test]
fn zero_size_keeps_the_original_dimensions() {
    let records = vec![png(7, 5, 1)];
    let bytes = batch(0, 0, FORMAT_PNG, &records);
    let specs = ImageBatchWorkload.split(&MemoryInput(&bytes), 1).unwrap();
    let output = ImageBatchWorkload
        .execute(&MemoryInput(&bytes), &specs[0])
        .unwrap();
    let parts = unframe(&output.result).unwrap();
    assert_eq!(decoded_size(&parts[0]), (7, 5));
}

#[test]
fn jpeg_output_is_decodable() {
    let records = vec![png(16, 16, 90)];
    let bytes = batch(8, 8, FORMAT_JPEG, &records);
    let specs = ImageBatchWorkload.split(&MemoryInput(&bytes), 1).unwrap();
    let output = ImageBatchWorkload
        .execute(&MemoryInput(&bytes), &specs[0])
        .unwrap();
    let parts = unframe(&output.result).unwrap();
    assert_eq!(decoded_size(&parts[0]), (8, 8));
    assert_eq!(image::guess_format(&parts[0]).unwrap(), ImageFormat::Jpeg);
}

#[test]
fn the_same_batch_gives_the_same_bytes_every_time() {
    let records = vec![png(9, 9, 33), png(5, 7, 66)];
    let bytes = batch(4, 4, FORMAT_JPEG, &records);
    let specs = ImageBatchWorkload.split(&MemoryInput(&bytes), 2).unwrap();
    for spec in &specs {
        let first = ImageBatchWorkload
            .execute(&MemoryInput(&bytes), spec)
            .unwrap();
        let second = ImageBatchWorkload
            .execute(&MemoryInput(&bytes), spec)
            .unwrap();
        assert_eq!(first, second);
    }
}

#[test]
fn an_honest_output_is_approved_and_a_tampered_one_is_rejected() {
    let records = vec![png(6, 6, 12)];
    let bytes = batch(3, 3, FORMAT_PNG, &records);
    let input = MemoryInput(&bytes);
    let specs = ImageBatchWorkload.split(&input, 1).unwrap();
    let honest = ImageBatchWorkload.execute(&input, &specs[0]).unwrap();
    assert_eq!(
        ImageBatchWorkload.verify(&input, &specs[0], &honest, &[0]),
        Ok(Verdict::Approve)
    );
    let mut tampered: Output = honest.clone();
    let last = tampered.result.len() - 1;
    tampered.result[last] ^= 0xff;
    assert!(matches!(
        ImageBatchWorkload.verify(&input, &specs[0], &tampered, &[0]),
        Ok(Verdict::Reject(_))
    ));
}

#[test]
fn a_record_that_is_not_an_image_fails_its_fragment() {
    let records = vec![png(4, 4, 1), b"pas une image".to_vec()];
    let bytes = batch(2, 2, FORMAT_PNG, &records);
    let specs = ImageBatchWorkload.split(&MemoryInput(&bytes), 1).unwrap();
    let result = ImageBatchWorkload.execute(&MemoryInput(&bytes), &specs[0]);
    assert!(matches!(result, Err(WorkloadError::BadInput(_))));
}

#[test]
fn an_audio_batch_is_refused_by_the_image_workload() {
    let bytes = build(
        Header {
            kind: Kind::Audio,
            format: 0,
            width: 0,
            height: 0,
            count: 0,
            options: Options::default(),
        },
        &[],
    )
    .unwrap();
    assert!(matches!(
        ImageBatchWorkload.split(&MemoryInput(&bytes), 1),
        Err(WorkloadError::BadInput(_))
    ));
}

#[test]
fn a_block_is_an_image_and_the_sample_catches_a_forged_image() {
    let records = vec![png(4, 4, 1), png(4, 4, 2), png(4, 4, 3)];
    let bytes = batch(0, 0, FORMAT_PNG, &records);
    let input = MemoryInput(&bytes);
    let specs = ImageBatchWorkload.split(&input, 1).unwrap();
    assert_eq!(ImageBatchWorkload.block_count(&specs[0]), Ok(3));
    let honest = ImageBatchWorkload.execute(&input, &specs[0]).unwrap();
    assert_eq!(honest.milestones.len(), 3);

    // Faux résultat cohérent avec ses milestones : l'image 1 est remplacée par une autre image.
    let mut parts = unframe(&honest.result).unwrap();
    parts[1] = process(
        &records[0],
        Header {
            kind: Kind::Image,
            format: FORMAT_PNG,
            width: 0,
            height: 0,
            count: 3,
            options: Options::default(),
        },
    )
    .unwrap();
    let mut forged = honest.clone();
    forged.result = frame(&parts).unwrap();
    forged.milestones[1] = sha256(&parts[1]);

    // Un échantillon qui ne tire pas l'image 1 ne la voit pas ; un échantillon complet la trouve.
    assert_eq!(
        ImageBatchWorkload.verify(&input, &specs[0], &forged, &[0, 2]),
        Ok(Verdict::Approve)
    );
    assert!(matches!(
        ImageBatchWorkload.verify(&input, &specs[0], &forged, &[0, 1, 2]),
        Ok(Verdict::Reject(_))
    ));
}

#[test]
fn a_result_that_breaks_its_own_milestones_is_rejected_without_rendering() {
    let records = vec![png(4, 4, 1), png(4, 4, 2)];
    let bytes = batch(0, 0, FORMAT_PNG, &records);
    let input = MemoryInput(&bytes);
    let specs = ImageBatchWorkload.split(&input, 1).unwrap();
    let mut output = ImageBatchWorkload.execute(&input, &specs[0]).unwrap();
    output.milestones.pop();
    assert!(matches!(
        ImageBatchWorkload.verify(&input, &specs[0], &output, &[]),
        Ok(Verdict::Reject(_))
    ));
}

#[test]
fn combining_keeps_the_fragments_in_order() {
    let a = Output {
        result: sil_batch::frame(&[b"1".to_vec()]).unwrap(),
        milestones: Vec::new(),
    };
    let b = Output {
        result: sil_batch::frame(&[b"2".to_vec(), b"3".to_vec()]).unwrap(),
        milestones: Vec::new(),
    };
    let joined = ImageBatchWorkload.combine(&[a, b]).unwrap();
    assert_eq!(
        unframe(&joined).unwrap(),
        vec![b"1".to_vec(), b"2".to_vec(), b"3".to_vec()]
    );
}

/// Un lot d'une image, avec les options données.
fn one_image_with(options: Options, width: u32, height: u32, fit_size: (u32, u32)) -> (u32, u32) {
    let bytes = build(
        Header {
            kind: Kind::Image,
            format: FORMAT_PNG,
            width: fit_size.0,
            height: fit_size.1,
            count: 1,
            options,
        },
        &[png(width, height, 9)],
    )
    .unwrap();
    let specs = ImageBatchWorkload.split(&MemoryInput(&bytes), 1).unwrap();
    let output = ImageBatchWorkload
        .execute(&MemoryInput(&bytes), &specs[0])
        .unwrap();
    decoded_size(&unframe(&output.result).unwrap()[0])
}

#[test]
fn contain_keeps_the_proportions_inside_the_box() {
    let options = Options {
        fit: FIT_CONTAIN,
        ..Options::default()
    };
    // 400x200 dans une boîte 100x100 : 100x50.
    assert_eq!(one_image_with(options, 400, 200, (100, 100)), (100, 50));
}

#[test]
fn cover_fills_the_box_and_crops_to_it() {
    let options = Options {
        fit: FIT_COVER,
        ..Options::default()
    };
    assert_eq!(one_image_with(options, 400, 200, (100, 100)), (100, 100));
}

#[test]
fn a_missing_side_follows_the_proportions_in_contain_mode() {
    let options = Options {
        fit: FIT_CONTAIN,
        ..Options::default()
    };
    // Largeur 200 seule : 400x200 devient 200x100.
    assert_eq!(one_image_with(options, 400, 200, (200, 0)), (200, 100));
}

#[test]
fn rotation_swaps_the_sides() {
    let options = Options {
        rotate: 1,
        ..Options::default()
    };
    assert_eq!(one_image_with(options, 40, 20, (0, 0)), (20, 40));
}

#[test]
fn grayscale_output_has_no_colour() {
    let options = Options {
        flags: FLAG_GRAYSCALE,
        ..Options::default()
    };
    let bytes = build(
        Header {
            kind: Kind::Image,
            format: FORMAT_PNG,
            width: 0,
            height: 0,
            count: 1,
            options,
        },
        &[png(8, 8, 200)],
    )
    .unwrap();
    let output = ImageBatchWorkload
        .execute(
            &MemoryInput(&bytes),
            &ImageBatchWorkload.split(&MemoryInput(&bytes), 1).unwrap()[0],
        )
        .unwrap();
    let decoded = image::load_from_memory(&unframe(&output.result).unwrap()[0]).unwrap();
    assert!(matches!(
        decoded.color(),
        image::ColorType::L8 | image::ColorType::La8
    ));
}

#[test]
fn a_lower_jpeg_quality_gives_a_smaller_file() {
    let records = vec![png(64, 64, 7)];
    let encode = |quality: u8| {
        let bytes = build(
            Header {
                kind: Kind::Image,
                format: FORMAT_JPEG,
                width: 0,
                height: 0,
                count: 1,
                options: Options {
                    quality,
                    ..Options::default()
                },
            },
            &records,
        )
        .unwrap();
        let specs = ImageBatchWorkload.split(&MemoryInput(&bytes), 1).unwrap();
        let output = ImageBatchWorkload
            .execute(&MemoryInput(&bytes), &specs[0])
            .unwrap();
        unframe(&output.result).unwrap()[0].len()
    };
    assert!(encode(20) < encode(95));
}
