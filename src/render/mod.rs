use image::{DynamicImage, GenericImageView, ImageBuffer, Luma};
use std::{
    collections::HashMap,
    path::Path,
    sync::{Arc, Mutex},
    time::Instant,
};

use crate::{App, ImageInfo};

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
    " ⠁⠂⠃⠄⠅⠆⠇⠈⠉⠊⠋⠌⠍⠎⠏⠐⠑⠒⠓⠔⠕⠖⠗⠘⠙⠚⠛⠜⠝⠞⠟⠠⠡⠢⠣⠤⠥⠦⠧⠨⠩⠪⠫⠬⠭⠮⠯⠰⠱⠲⠳⠴⠵⠶⠷⠸⠹⠺⠻⠼⠽⠾⠿⡀⡁⡂⡃⡄⡅⡆⡇⡈⡉⡊⡋⡌⡍⡎⡏⡐⡑⡒⡓⡔⡕⡖⡗⡘⡙⡚⡛⡜⡝⡞⡟⡠⡡⡢⡣⡤⡥⡦⡧⡨⡩⡪⡫⡬⡭⡮⡯⡰⡱⡲⡳⡴⡵⡶⡷⡸⡹⡺⡻⡼⡽⡾⡿⢀⢁⢂⢃⢄⢅⢆⢇⢈⢉⢊⢋⢌⢍⢎⢏⢐⢑⢒⢓⢔⢕⢖⢗⢘⢙⢚⢛⢜⢝⢞⢟⢠⢡⢢⢣⢤⢥⢦⢧⢨⢩⢪⢫⢬⢭⢮⢯⢰⢱⢲⢳⢴⢵⢶⢷⢸⢹⢺⢻⢼⢽⢾⢿⣀⣁⣂⣃⣄⣅⣆⣇⣈⣉⣊⣋⣌⣍⣎⣏⣐⣑⣒⣓⣔⣕⣖⣗⣘⣙⣚⣛⣜⣝⣞⣟⣠⣡⣢⣣⣤⣥⣦⣧⣨⣩⣪⣫⣬⣭⣮⣯⣰⣱⣲⣳⣴⣵⣶⣷⣸⣹⣺⣻⣼⣽⣾⣿",
    " !@#$%^&*()-=_+`~qwfpgjluy;[]arstdhneio'zxcvbkm,./\\|QWFPGJLUY:{}ARSTDHNEIO\"ZXCVBKM<>?",
];

#[derive(Clone)]
pub struct RenderSettings {
    /// Number of columns
    pub cols: u32,

    /// Invert colors
    pub invert: bool,

    /// Mode (values, pxmatch)
    mode: Mode,

    /// Palette (1-based index)
    palette: usize,

    /// Image offset
    pub offset: (u32, u32),

    /// Image size
    pub size: (u32, u32),
}

impl RenderSettings {
    pub fn new() -> Self {
        Self {
            cols: 80,
            invert: false,
            mode: Mode::Values,
            palette: 1,
            offset: (0, 0),
            size: (0, 0),
        }
    }

    pub fn toggle_mode(&mut self) {
        self.mode = match self.mode {
            Mode::Values => Mode::PixelMatch,
            Mode::PixelMatch => Mode::Values,
        };
    }

    /// Palette index is 1-based
    pub fn set_palette(&mut self, palette: usize) -> bool {
        if palette > 0 && palette <= PALETTE.len() {
            self.palette = palette;
            true
        } else {
            false
        }
    }
}

pub fn render(settings: &RenderSettings, img: &DynamicImage) -> String {
    let line_height = 2.0;
    // memory cache will be usefull to process batches of images or video stream
    let mut char_img_cache: HashMap<char, ImageBuffer<Luma<u8>, Vec<u8>>> = HashMap::new();
    let char_matrix = match settings.mode {
        Mode::Values => render_by_value::render_by_value(
            &img,
            settings.cols,
            line_height,
            settings.invert,
            PALETTE[settings.palette - 1],
            settings,
        ),
        Mode::PixelMatch => render_by_match::render_by_match(
            &img,
            settings.cols,
            line_height,
            settings.invert,
            PALETTE[settings.palette - 1],
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

pub fn worker_loop(app_state: &Arc<Mutex<App>>) {
    let mut img_path = None;
    let mut img = None;
    let mut render_time = 42;
    let mut result = String::new();
    let mut settings;
    let mut do_render;
    loop {
        {
            let mut app = app_state.lock().unwrap();
            if app.exit {
                break;
            }
            do_render = app.do_render;

            if img_path != app.img_path {
                if let Some(path) = &app.img_path {
                    img_path = Some(path.clone());
                    match image::open(&Path::new(&path)) {
                        Ok(i) => {
                            let img_info = ImageInfo {
                                dimensions: i.dimensions(),
                                color: i.color(),
                            };
                            img = Some(i);
                            app.settings.size = img_info.dimensions;
                            app.img_info = Some(img_info);
                        }
                        Err(e) => {
                            app.status = format!("Error: {}", e);
                            img = None;
                        }
                    }
                    do_render = true;
                }
            }

            settings = app.settings.clone();
        }

        if do_render {
            let start = Instant::now();
            match &img {
                Some(i) => {
                    result = render(&settings, &i);
                }
                None => result = "No image".to_string(),
            }
            render_time = start.elapsed().as_millis();
        }

        {
            let mut app = app_state.lock().unwrap();
            app.result = result.clone();
            app.do_render = false;
            // TODO: app status should go in the ui thread, so we can draw offset changes while rendering
            // and show a spinner icon while rendering
            app.status = match &app.img_info {
                Some(img_info) => format!(
                    "{} / x={} y={} / {}ms / {}",
                    img_info, app.settings.offset.0, app.settings.offset.1, render_time, do_render
                ),
                None => "No image".to_string(),
            };
        }

        std::thread::sleep(std::time::Duration::from_millis(16));
    }
}
