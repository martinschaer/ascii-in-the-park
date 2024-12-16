use std::{
    collections::HashMap,
    hash::{DefaultHasher, Hash, Hasher},
    path::Path,
};

use image::{GrayImage, ImageBuffer, Luma};
use imageproc::{drawing::draw_text_mut, template_matching::match_template};
use rusttype::{Font, Scale};

use super::RenderSettings;

fn generate_char_imgs(
    chars: &Vec<char>,
    tile_w: u32,
    tile_h: u32,
    char_img_cache: &mut HashMap<char, ImageBuffer<Luma<u8>, Vec<u8>>>,
) -> Vec<ImageBuffer<Luma<u8>, Vec<u8>>> {
    // let start = Instant::now();

    let scale = Scale {
        x: tile_w as f32,
        y: tile_h as f32,
    };

    let char_imgs = chars
        .iter()
        .enumerate()
        .map(|(_, c)| {
            // check memory cache
            if let Some(img) = char_img_cache.get(c) {
                return img.clone();
            }

            // load font
            let font = Vec::from(include_bytes!(
                "../../fonts/Caskaydia Cove Nerd Font Complete Regular.otf"
            ) as &[u8]);
            let font = Font::try_from_vec(font).unwrap();

            // check disk cache
            let mut hasher = DefaultHasher::new();
            c.hash(&mut hasher);
            tile_w.hash(&mut hasher);
            tile_h.hash(&mut hasher);
            let hash = hasher.finish();
            let cache_file = format!("cache/{}.png", hash);

            if Path::new(&cache_file).exists() {
                return image::open(cache_file).unwrap().to_luma8();
            }

            let mut char_img = GrayImage::new(tile_w, tile_h);
            char_img.fill(255);
            draw_text_mut(&mut char_img, Luma([0]), 0, 0, scale, &font, &c.to_string());

            char_img.save(&cache_file).unwrap();
            char_img_cache.insert(*c, char_img.clone());

            char_img
        })
        .collect::<Vec<GrayImage>>();

    // let duration = start.elapsed();
    // println!("generate_char_imgs() took: {:?}", duration);

    char_imgs
}

pub fn render_by_match(
    img: &image::DynamicImage,
    cols: u32,
    line_height: f32,
    invert: bool,
    palette: &str,
    settings: &RenderSettings,
    char_img_cache: &mut HashMap<char, ImageBuffer<Luma<u8>, Vec<u8>>>,
) -> Vec<char> {
    let tile_w = 10;
    let tile_h = tile_w * line_height as u32;
    let w = cols * tile_w;

    let ar = settings.crop.2 as f32 / settings.crop.3 as f32;
    let rows = (cols as f32 / (ar * line_height)) as u32;
    let h = (w as f32 / ar) as u32;

    // TODO: don't make it bigger
    let mut img = img.resize_exact(w, h, image::imageops::FilterType::Nearest);
    if invert {
        img.invert()
    }

    let chars = palette.chars().collect::<Vec<char>>();
    let mut char_matrix = vec!['*'; (cols * rows) as usize];
    let char_imgs = generate_char_imgs(&chars, tile_w, tile_h, char_img_cache);

    for i in 0..(cols * rows) {
        let tile = img
            .crop_imm(
                (i % cols) * tile_w,
                (i as f32 / cols as f32).floor() as u32 * tile_h,
                tile_w,
                tile_h,
            )
            .to_luma8();
        if tile.width() != tile_w || tile.height() != tile_h {
            char_matrix[i as usize] = '_';
            continue;
        }

        // tests all chars agaist tile
        let mut best = 0;
        let mut best_score = u32::MAX;
        for (ci, char_img) in char_imgs.iter().enumerate() {
            let matched = match_template(
                &char_img,
                &tile,
                // imageproc::template_matching::MatchTemplateMethod::CrossCorrelation,
                imageproc::template_matching::MatchTemplateMethod::SumOfSquaredErrors,
            );
            let score = matched.pixels().map(|p| p[0]).sum::<f32>().abs() as u32;
            if score < best_score {
                best = ci;
                best_score = score;
            }
        }
        char_matrix[i as usize] = chars[best];
    }

    char_matrix
}
