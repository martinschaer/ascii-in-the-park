use super::RenderSettings;

pub fn render_by_value(
    img: &image::DynamicImage,
    cols: u32,
    line_height: f32,
    invert: bool,
    palette: &str,
    settings: &RenderSettings,
) -> Vec<char> {
    let cropped_size = (
        settings.size.0 - settings.offset.0,
        settings.size.1 - settings.offset.1,
    );
    let ar = cropped_size.0 as f32 / cropped_size.1 as f32;
    let rows = (cols as f32 / (ar * line_height)) as u32;
    let char_matrix = img
        .crop_imm(
            settings.offset.0,
            settings.offset.1,
            cropped_size.0,
            cropped_size.1,
        )
        .resize_exact(cols, rows, image::imageops::FilterType::Nearest)
        .to_luma8()
        .pixels()
        .map(|p| {
            let mut v = p.0[0];
            v = if invert { 255 - v } else { v };
            palette
                .chars()
                .nth((palette.len() as f32 * (v as f32 / 256.0)) as usize)
                .unwrap()
        })
        .collect::<Vec<char>>();

    char_matrix
}
