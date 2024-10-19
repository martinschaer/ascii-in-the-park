use clap::Parser;
use crossterm::event::{Event, KeyEvent};
use image::{GenericImageView, GrayImage, ImageBuffer, Luma};
use imageproc::{drawing::draw_text_mut, template_matching::match_template};
use ratatui::{
    crossterm::event::{self, KeyCode, KeyEventKind},
    style::Stylize,
    symbols::border,
    widgets::{block::Title, Block, Paragraph},
    DefaultTerminal, Frame,
};
use rusttype::{Font, Scale};
use std::collections::HashMap;
use std::fs;
use std::io;
use std::time::Instant;
use std::{
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
    path::Path,
};

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

fn generate_char_imgs(
    chars: &Vec<char>,
    tile_w: u32,
    tile_h: u32,
    char_img_cache: &mut HashMap<char, ImageBuffer<Luma<u8>, Vec<u8>>>,
) -> Vec<ImageBuffer<Luma<u8>, Vec<u8>>> {
    let start = Instant::now();

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
            // check memory cache
            if let Some(img) = char_img_cache.get(c) {
                return img.clone();
            }

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

    let duration = start.elapsed();
    println!("generate_char_imgs() took: {:?}", duration);

    char_imgs
}

fn paint_flat(
    img: &image::DynamicImage,
    cols: u32,
    line_height: f32,
    invert: bool,
    palette: &str,
    char_img_cache: &mut HashMap<char, ImageBuffer<Luma<u8>, Vec<u8>>>,
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

#[derive(Debug, Default)]
struct App {
    result: String,
    exit: bool,
}

impl App {
    fn run(&mut self, terminal: &mut DefaultTerminal) -> io::Result<()> {
        while !self.exit {
            terminal.draw(|frame| self.draw(frame))?;
            self.handle_events()?;
        }
        Ok(())
    }

    fn draw(&self, frame: &mut Frame) {
        let block = Block::bordered()
            .title(Title::from("Canvas"))
            .border_set(border::ROUNDED);
        let greeting = Paragraph::new(self.result.clone())
            .white()
            .on_black()
            .block(block);
        frame.render_widget(greeting, frame.area());
    }

    /// updates the application's state based on user input
    fn handle_events(&mut self) -> io::Result<()> {
        match event::read()? {
            // it's important to check that the event is a key press event as
            // crossterm also emits key release and repeat events on Windows.
            Event::Key(key_event) if key_event.kind == KeyEventKind::Press => {
                self.handle_key_event(key_event)
            }
            _ => {}
        };
        Ok(())
    }

    fn handle_key_event(&mut self, key_event: KeyEvent) {
        match key_event.code {
            KeyCode::Char('q') => self.exit(),
            _ => {}
        }
    }

    fn exit(&mut self) {
        self.exit = true;
    }
}

fn paint(args: Args, img: image::DynamicImage) -> String {
    let line_height = 2.0;
    // memory cache will be usefull to process batches of images or video stream
    let mut char_img_cache: HashMap<char, ImageBuffer<Luma<u8>, Vec<u8>>> = HashMap::new();
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
            &mut char_img_cache,
        ),
    };

    let mut result = String::new();
    for (i, c) in char_matrix.iter().enumerate() {
        result.push(c.clone());
        if (i + 1) % args.cols as usize == 0 {
            result.push('\n');
        }
    }
    result
}

fn main() -> io::Result<()> {
    let args = Args::parse();
    let img = image::open(&Path::new(&args.img)).unwrap();

    // check if /cache dir exists, if not create it
    if !Path::new("cache").exists() {
        fs::create_dir("cache").unwrap();
    }

    println!("dimensions {:?}", img.dimensions());
    println!("color {:?}", img.color());
    println!("palette {:?}: {}", args.palette, PALETTE[args.palette]);

    let result = paint(args, img);

    let mut terminal = ratatui::init();
    // terminal.clear()?;
    let app_result = App {
        result,
        exit: false,
    }
    .run(&mut terminal);
    ratatui::restore();
    app_result
}
