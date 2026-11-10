use image::{DynamicImage, imageops::FilterType};
use sil_batch::{
    FIT_CONTAIN, FIT_COVER, FLAG_FLIP_HORIZONTAL, FLAG_FLIP_VERTICAL, FLAG_GRAYSCALE, Header,
};

/// Applique les options d'une image, dans cet ordre : rotation, miroirs, niveaux de gris, puis
/// redimensionnement. Chaque étape est déterministe : même entrée, mêmes pixels.
pub fn apply(image: DynamicImage, header: Header) -> DynamicImage {
    let options = header.options;
    let image = match options.rotate {
        1 => image.rotate90(),
        2 => image.rotate180(),
        3 => image.rotate270(),
        _ => image,
    };
    let image = if options.flags & FLAG_FLIP_HORIZONTAL != 0 {
        image.fliph()
    } else {
        image
    };
    let image = if options.flags & FLAG_FLIP_VERTICAL != 0 {
        image.flipv()
    } else {
        image
    };
    let image = if options.flags & FLAG_GRAYSCALE != 0 {
        image.grayscale()
    } else {
        image
    };
    resize(image, header)
}

/// Redimensionne à la taille de l'en-tête selon son mode. Une dimension absente (0) prend la
/// valeur de l'image source ; en mode « tenir » ou « remplir », elle se déduit des proportions.
fn resize(image: DynamicImage, header: Header) -> DynamicImage {
    if header.width == 0 && header.height == 0 {
        return image;
    }
    let (source_w, source_h) = (image.width(), image.height());
    match header.options.fit {
        FIT_CONTAIN | FIT_COVER => fit_proportionally(image, header),
        _ => {
            let width = if header.width == 0 {
                source_w
            } else {
                header.width
            };
            let height = if header.height == 0 {
                source_h
            } else {
                header.height
            };
            if (width, height) == (source_w, source_h) {
                return image;
            }
            image.resize_exact(width, height, FilterType::Triangle)
        }
    }
}

/// « Tenir » : la plus grande image aux proportions de la source qui entre dans la boîte.
/// « Remplir » : la plus petite qui couvre la boîte, puis recadrée au centre.
fn fit_proportionally(image: DynamicImage, header: Header) -> DynamicImage {
    let (source_w, source_h) = (image.width(), image.height());
    let (box_w, box_h) = match (header.width, header.height) {
        (0, height) => (scaled(source_w, height, source_h), height),
        (width, 0) => (width, scaled(source_h, width, source_w)),
        (width, height) => (width, height),
    };
    let ratio_w = f64::from(box_w) / f64::from(source_w);
    let ratio_h = f64::from(box_h) / f64::from(source_h);
    let ratio = if header.options.fit == FIT_COVER {
        ratio_w.max(ratio_h)
    } else {
        ratio_w.min(ratio_h)
    };
    let width = rounded(f64::from(source_w) * ratio).max(1);
    let height = rounded(f64::from(source_h) * ratio).max(1);
    if header.options.fit == FIT_COVER {
        let (width, height) = (width.max(box_w), height.max(box_h));
        let resized = image.resize_exact(width, height, FilterType::Triangle);
        let (x, y) = ((width - box_w) / 2, (height - box_h) / 2);
        return resized.crop_imm(x, y, box_w, box_h);
    }
    if (width, height) == (source_w, source_h) {
        return image;
    }
    image.resize_exact(width, height, FilterType::Triangle)
}

/// `part` mis à l'échelle de `of` vers `to`, arrondi, au moins 1.
fn scaled(part: u32, to: u32, of: u32) -> u32 {
    rounded(f64::from(part) * f64::from(to) / f64::from(of)).max(1)
}

fn rounded(value: f64) -> u32 {
    // Les dimensions d'une image sont bornées par son décodeur : la conversion ne déborde pas.
    value.round() as u32
}
