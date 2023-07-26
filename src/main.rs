use clap::Parser;
use image::{GenericImageView, GrayImage, Luma};
use imageproc::{drawing::draw_text_mut, template_matching::match_template};
use rusttype::{Font, Scale};
use std::path::Path;

#[derive(Debug, Clone)]
enum Mode {
    Values,
    PixelMatch,
}

impl std::fmt::Display for Mode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Mode::Values => "values",
            Mode::PixelMatch => "pxmatch",
        };
        s.fmt(f)
    }
}

impl std::str::FromStr for Mode {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "values" => Ok(Mode::Values),
            "pxmatch" => Ok(Mode::PixelMatch),
            _ => Err(format!("Unknown mode: {}", s)),
        }
    }
}

// pre-defined palettes
const PALETTE : [&str; 4] = [
    " .-=+*#%@",
    ".,`~|\\/+X#",
    "⠀⠁⠂⠃⠄⠅⠆⠇⠈⠉⠊⠋⠌⠍⠎⠏⠐⠑⠒⠓⠔⠕⠖⠗⠘⠙⠚⠛⠜⠝⠞⠟⠠⠡⠢⠣⠤⠥⠦⠧⠨⠩⠪⠫⠬⠭⠮⠯⠰⠱⠲⠳⠴⠵⠶⠷⠸⠹⠺⠻⠼⠽⠾⠿⡀⡁⡂⡃⡄⡅⡆⡇⡈⡉⡊⡋⡌⡍⡎⡏⡐⡑⡒⡓⡔⡕⡖⡗⡘⡙⡚⡛⡜⡝⡞⡟⡠⡡⡢⡣⡤⡥⡦⡧⡨⡩⡪⡫⡬⡭⡮⡯⡰⡱⡲⡳⡴⡵⡶⡷⡸⡹⡺⡻⡼⡽⡾⡿⢀⢁⢂⢃⢄⢅⢆⢇⢈⢉⢊⢋⢌⢍⢎⢏⢐⢑⢒⢓⢔⢕⢖⢗⢘⢙⢚⢛⢜⢝⢞⢟⢠⢡⢢⢣⢤⢥⢦⢧⢨⢩⢪⢫⢬⢭⢮⢯⢰⢱⢲⢳⢴⢵⢶⢷⢸⢹⢺⢻⢼⢽⢾⢿⣀⣁⣂⣃⣄⣅⣆⣇⣈⣉⣊⣋⣌⣍⣎⣏⣐⣑⣒⣓⣔⣕⣖⣗⣘⣙⣚⣛⣜⣝⣞⣟⣠⣡⣢⣣⣤⣥⣦⣧⣨⣩⣪⣫⣬⣭⣮⣯⣰⣱⣲⣳⣴⣵⣶⣷⣸⣹⣺⣻⣼⣽⣾⣿",
    " !@#$%^&*()-=_+`~qwfpgjluy;[]arstdhneio'zxcvbkm,./\\|QWFPGJLUY:{}ARSTDHNEIO\"ZXCVBKM<>?",
];

fn validate_palette_index(s: &str) -> Result<usize, String> {
    let index = s.parse::<usize>().map_err(|e| e.to_string())?;
    if index >= PALETTE.len() {
        Err(format!("Invalid palette index: {}", index))
    } else {
        Ok(index)
    }
}

/// CLI interface for a virtual park where AI gathers to do ASCII paintings of your images.
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Image file
    #[arg(short, long)]
    img: String,

    /// Number of columns
    #[arg(short, long, default_value = "80")]
    cols: u32,

    /// Invert colors
    #[arg(short = 'I', long)]
    invert: bool,

    /// Mode (values, pxmatch)
    #[arg(short, long, default_value_t = Mode::Values)]
    mode: Mode,

    /// Palette
    #[arg(short, long, default_value = "0", value_parser = validate_palette_index)]
    palette: usize,
}

fn paint_values(
    img: &image::DynamicImage,
    cols: u32,
    line_height: f32,
    invert: bool,
    palette: &str,
) -> Vec<char> {
    let ar = img.width() as f32 / img.height() as f32;
    let rows = (cols as f32 / (ar * line_height)) as u32;
    let char_matrix = img
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

fn paint_flat(
    img: &image::DynamicImage,
    cols: u32,
    line_height: f32,
    invert: bool,
    palette: &str,
) -> Vec<char> {
    let tile_w = 10;
    let tile_h = tile_w * line_height as u32;
    let w = cols * tile_w;
    let ar = img.width() as f32 / img.height() as f32;
    let rows = (cols as f32 / (ar * line_height)) as u32;
    let h = (w as f32 / ar) as u32;
    println!("cols {}, rows {}", cols, rows);
    println!("w: {}, h: {}", w, h);

    // TODO: don't make it bigger
    let mut img = img.resize_exact(w, h, image::imageops::FilterType::Nearest);
    if invert {
        img.invert()
    }

    let chars = palette.chars().collect::<Vec<char>>();
    let mut char_matrix = vec!['*'; (cols * rows) as usize];
    let scale = Scale {
        x: tile_w as f32,
        y: tile_h as f32,
    };
    let font = Vec::from(
        include_bytes!("../fonts/Caskaydia Cove Nerd Font Complete Regular.otf") as &[u8],
    );
    let font = Font::try_from_vec(font).unwrap();

    let char_imgs = chars
        .iter()
        .enumerate()
        .map(|(_, c)| {
            let mut char_img = GrayImage::new(tile_w, tile_h);
            char_img.fill(255);
            draw_text_mut(&mut char_img, Luma([0]), 0, 0, scale, &font, &c.to_string());
            // TODO: use cli option to enable this
            // char_img.save(&format!("out/{}.png", i)).unwrap();
            char_img
        })
        .collect::<Vec<GrayImage>>();

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
            println!("tile size mismatch {} {}", tile.width(), tile.height());
            char_matrix[i as usize] = '_';
            continue;
        }

        // tests all chars agaist tile
        let mut best = 0;
        // let mut best_score = 0;
        let mut best_score = u32::MAX;
        for (ci, char_img) in char_imgs.iter().enumerate() {
            let matched = match_template(
                &char_img,
                &tile,
                // imageproc::template_matching::MatchTemplateMethod::CrossCorrelation,
                imageproc::template_matching::MatchTemplateMethod::SumOfSquaredErrors,
            );
            let score = matched.pixels().map(|p| p[0]).sum::<f32>().abs() as u32;
            // if score > best_score {
            if score < best_score {
                best = ci;
                best_score = score;
            }
        }
        char_matrix[i as usize] = chars[best];
    }

    char_matrix
}

fn main() {
    let args = Args::parse();
    let img = image::open(&Path::new(&args.img)).unwrap();
    let line_height = 2.0;

    println!("dimensions {:?}", img.dimensions());
    println!("color {:?}", img.color());
    println!("palette {:?}: {}", args.palette, PALETTE[args.palette]);

    let char_matrix = match args.mode {
        Mode::Values => paint_values(
            &img,
            args.cols,
            line_height,
            args.invert,
            PALETTE[args.palette],
        ),
        Mode::PixelMatch => paint_flat(
            &img,
            args.cols,
            line_height,
            args.invert,
            PALETTE[args.palette],
        ),
    };

    println!("---");
    for (i, c) in char_matrix.iter().enumerate() {
        print!("{}", c);
        if (i + 1) % args.cols as usize == 0 {
            println!();
        }
    }
}
