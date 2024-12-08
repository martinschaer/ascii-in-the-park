use image::{DynamicImage, ImageBuffer, Luma};
use std::collections::HashMap;

mod render_by_match;
mod render_by_value;

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

pub struct PaintSettings {
    /// Number of columns
    pub cols: u32,

    /// Invert colors
    pub invert: bool,

    /// Mode (values, pxmatch)
    mode: Mode,

    /// Palette
    palette: usize,

    /// Image offset
    pub offset: (u32, u32),

    /// Image size
    pub size: (u32, u32),
}

impl PaintSettings {
    pub fn new(size: (u32, u32)) -> Self {
        Self {
            cols: 80,
            invert: false,
            mode: Mode::Values,
            palette: 0,
            offset: (0, 0),
            size,
        }
    }

    pub fn toggle_mode(&mut self) {
        self.mode = match self.mode {
            Mode::Values => Mode::PixelMatch,
            Mode::PixelMatch => Mode::Values,
        };
    }

    pub fn set_palette(&mut self, palette: usize) -> bool {
        if palette < PALETTE.len() {
            self.palette = palette;
            true
        } else {
            false
        }
    }
}

pub fn paint(settings: &PaintSettings, img: &DynamicImage) -> String {
    let line_height = 2.0;
    // memory cache will be usefull to process batches of images or video stream
    let mut char_img_cache: HashMap<char, ImageBuffer<Luma<u8>, Vec<u8>>> = HashMap::new();
    let char_matrix = match settings.mode {
        Mode::Values => render_by_value::render_by_value(
            &img,
            settings.cols,
            line_height,
            settings.invert,
            PALETTE[settings.palette],
            settings,
        ),
        Mode::PixelMatch => render_by_match::render_by_match(
            &img,
            settings.cols,
            line_height,
            settings.invert,
            PALETTE[settings.palette],
            settings,
            &mut char_img_cache,
        ),
    };

    let mut result = String::new();
    for (i, c) in char_matrix.iter().enumerate() {
        result.push(c.clone());
        if (i + 1) % settings.cols as usize == 0 {
            result.push('\n');
        }
    }
    result
}
