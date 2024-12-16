use super::RenderSettings;

pub fn render_by_value(
    img: &image::DynamicImage,
    cols: u32,
    line_height: f32,
    invert: bool,
    palette: &str,
    settings: &RenderSettings,
) -> Vec<char> {
    let ar = settings.crop.2 as f32 / settings.crop.3 as f32;
    let rows = (cols as f32 / (ar * line_height)) as u32;
    let char_matrix = img
        .crop_imm(
            settings.crop.0,
            settings.crop.1,
            settings.crop.2,
            settings.crop.3,
        )
        .resize_exact(cols, rows, image::imageops::FilterType::Nearest)
        .to_luma8()
        .pixels()
        .map(|p| {
            let mut v = p.0[0];
            v = if invert { 255 - v } else { v };
            palette
                .chars()
                .nth(((palette.len() - 1) as f32 * (v as f32 / 256.0)) as usize)
                .unwrap_or(palette.chars().nth(0).unwrap())
        })
        .collect::<Vec<char>>();

    char_matrix
}
