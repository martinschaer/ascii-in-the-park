use clap::Parser;
use image::{GenericImageView, GrayImage, Luma};
use imageproc::{drawing::draw_text_mut, template_matching::match_template};
use rusttype::{Font, Scale};
use std::path::Path;

#[derive(Debug, Clone)]
enum Mode {
    Values,
    Flat,
}

impl std::fmt::Display for Mode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Mode::Values => "values",
            Mode::Flat => "flat",
        };
        s.fmt(f)
    }
}

impl std::str::FromStr for Mode {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "values" => Ok(Mode::Values),
            "flat" => Ok(Mode::Flat),
            _ => Err(format!("Unknown mode: {}", s)),
        }
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

    /// Mode (values, flat)
    #[arg(short, long, default_value_t = Mode::Values)]
    mode: Mode,
}

fn paint_values(img: &image::DynamicImage, cols: u32, rows: u32, invert: bool) -> Vec<char> {
    let char_matrix = img
        .resize_exact(cols, rows, image::imageops::FilterType::Nearest)
        .to_luma8()
        .pixels()
        .map(|p| {
            let mut v = p.0[0];
            v = if invert { 255 - v } else { v };
            match v {
                0..=31 => ' ',
                32..=63 => '.',
                64..=95 => '-',
                96..=127 => '=',
                128..=159 => '+',
                160..=191 => '*',
                192..=223 => '#',
                224..=255 => '@',
            }
        })
        .collect::<Vec<char>>();

    char_matrix
}

fn paint_flat(img: &image::DynamicImage, cols: u32, rows: u32, invert: bool) -> Vec<char> {
    let tile_w = 10;
    let w = cols * tile_w;
    let ar = img.width() as f32 / img.height() as f32;
    let h = (w as f32 / ar) as u32;
    println!("w: {}, h: {}", w, h);

    // TODO: don't make it bigger
    let mut img = img.resize_exact(w, h, image::imageops::FilterType::Nearest);
    if invert {
        img.invert()
    }

    let str = "⠀⠁⠂⠃⠄⠅⠆⠇⠈⠉⠊⠋⠌⠍⠎⠏⠐⠑⠒⠓⠔⠕⠖⠗⠘⠙⠚⠛⠜⠝⠞⠟⠠⠡⠢⠣⠤⠥⠦⠧⠨⠩⠪⠫⠬⠭⠮⠯⠰⠱⠲⠳⠴⠵⠶⠷⠸⠹⠺⠻⠼⠽⠾⠿⡀⡁⡂⡃⡄⡅⡆⡇⡈⡉⡊⡋⡌⡍⡎⡏⡐⡑⡒⡓⡔⡕⡖⡗⡘⡙⡚⡛⡜⡝⡞⡟⡠⡡⡢⡣⡤⡥⡦⡧⡨⡩⡪⡫⡬⡭⡮⡯⡰⡱⡲⡳⡴⡵⡶⡷⡸⡹⡺⡻⡼⡽⡾⡿⢀⢁⢂⢃⢄⢅⢆⢇⢈⢉⢊⢋⢌⢍⢎⢏⢐⢑⢒⢓⢔⢕⢖⢗⢘⢙⢚⢛⢜⢝⢞⢟⢠⢡⢢⢣⢤⢥⢦⢧⢨⢩⢪⢫⢬⢭⢮⢯⢰⢱⢲⢳⢴⢵⢶⢷⢸⢹⢺⢻⢼⢽⢾⢿⣀⣁⣂⣃⣄⣅⣆⣇⣈⣉⣊⣋⣌⣍⣎⣏⣐⣑⣒⣓⣔⣕⣖⣗⣘⣙⣚⣛⣜⣝⣞⣟⣠⣡⣢⣣⣤⣥⣦⣧⣨⣩⣪⣫⣬⣭⣮⣯⣰⣱⣲⣳⣴⣵⣶⣷⣸⣹⣺⣻⣼⣽⣾⣿";
    let chars = str.chars().collect::<Vec<char>>();
    let mut char_matrix = vec!['*'; (cols * rows) as usize];
    let scale = Scale {
        x: tile_w as f32 * 2.0,
        y: tile_w as f32 * 1.0,
    };
    let font = Vec::from(
        include_bytes!("../fonts/Caskaydia Cove Nerd Font Complete Regular.otf") as &[u8],
    );
    let font = Font::try_from_vec(font).unwrap();

    let char_imgs = chars
        .iter()
        .enumerate()
        .map(|(_, c)| {
            let mut char_img = GrayImage::new(tile_w, tile_w);
            char_img.fill(255);
            draw_text_mut(
                &mut char_img,
                Luma([0]),
                0,
                0,
                scale,
                &font,
                &c.to_string(),
            );
            // TODO: use cli option to enable this
            // char_img.save(&format!("out/{}.png", i)).unwrap();
            char_img
        })
        .collect::<Vec<GrayImage>>();

    for i in 0..(cols * rows) {
        let tile = img
            .crop_imm(
                (i % cols) * tile_w,
                (i as f32 / cols as f32).floor() as u32 * tile_w,
                tile_w,
                tile_w,
            )
            .to_luma8();
        if tile.width() != tile_w || tile.height() != tile_w {
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
    let rows = (img.height() as f32 * args.cols as f32 / img.width() as f32) as u32;

    println!("dimensions {:?}", img.dimensions());
    println!("color {:?}", img.color());
    println!("cols {}, rows {}", args.cols, rows);

    let char_matrix = match args.mode {
        Mode::Values => paint_values(&img, args.cols, rows, args.invert),
        Mode::Flat => paint_flat(&img, args.cols, rows, args.invert),
    };

    for (i, c) in char_matrix.iter().enumerate() {
        print!("{}", c);
        if (i + 1) % args.cols as usize == 0 {
            println!();
        }
    }
}
