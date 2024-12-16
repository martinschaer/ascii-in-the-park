use clap::Parser;
use crossterm::{
    event::KeyEvent,
    terminal::{disable_raw_mode, enable_raw_mode},
};
use ratatui::{
    crossterm::event::{self, KeyCode, KeyEventKind},
    layout::{self, Constraint, Layout},
    style::{Style, Stylize},
    symbols::border,
    text::Span,
    widgets::{Block, Borders, Paragraph},
    DefaultTerminal, Frame,
};

use render::{worker_loop, RenderSettings};
use std::{
    fmt::{self, Display, Formatter},
    path::Path,
    time::Duration,
};
use std::{fs, thread};
use std::{
    io,
    sync::{Arc, Mutex},
};

mod render;

enum CropMode {
    TopLeft,
    WidthHeight,
}

impl Display for CropMode {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        let s = match self {
            CropMode::TopLeft => "Crop top left",
            CropMode::WidthHeight => "Crop width height",
        };
        s.fmt(f)
    }
}

/// CLI interface for a virtual park where AI gathers to do ASCII paintings of your images.
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Image file
    #[arg(short, long)]
    img: Option<String>,
}

struct ImageInfo {
    dimensions: (u32, u32),
    color: image::ColorType,
}

impl Display for ImageInfo {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(
            f,
            "Dimensions: {}x{} / Color: {:?}",
            self.dimensions.0, self.dimensions.1, self.color
        )
    }
}

struct App {
    img_path: Option<String>,
    img_info: Option<ImageInfo>,
    do_render: bool,
    settings: RenderSettings,
    result: String,
    status: String,
    exit: bool,
    crop_mode: CropMode,
}

impl App {
    fn new(img_path: Option<String>) -> Self {
        Self {
            img_path,
            img_info: None,
            do_render: false,
            settings: RenderSettings::new(),
            result: String::default(),
            status: String::default(),
            exit: false,
            crop_mode: CropMode::TopLeft,
        }
    }

    fn tick(&mut self, terminal: &mut DefaultTerminal) -> io::Result<()> {
        terminal.draw(|frame| self.draw(frame))?;
        self.handle_events()?;
        Ok(())
    }

    fn draw(&self, frame: &mut Frame) {
        let vertical = Layout::vertical([
            Constraint::Length(1),
            Constraint::Min(0),
            Constraint::Length(3),
            Constraint::Length(1),
        ]);
        let [title_area, canvas_area, settings_area, footer_area] = vertical.areas(frame.area());

        let canvas =
            Paragraph::new(self.result.clone()).block(Block::bordered().borders(Borders::TOP));

        let footer = Paragraph::new(self.status.clone());
        let btn_style = Style::default().on_white().black();

        frame.render_widget(
            Block::new()
                .title("Ascii in the park")
                .title_alignment(layout::Alignment::Center),
            title_area,
        );
        frame.render_widget(canvas, canvas_area);
        frame.render_widget(
            Paragraph::new(Span::styled(format!("{} [c]", self.crop_mode), btn_style)).block(
                Block::bordered()
                    .title("Settings")
                    .border_set(border::ROUNDED),
            ),
            settings_area,
        );
        frame.render_widget(footer, footer_area);
    }

    fn handle_events(&mut self) -> io::Result<()> {
        if event::poll(Duration::from_millis(16))? {
            if let event::Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    self.handle_key_event(key);
                }
            }
        }
        Ok(())
    }

    fn handle_key_event(&mut self, key_event: KeyEvent) {
        match key_event.code {
            KeyCode::Char('q') => self.exit = true,
            KeyCode::Char('c') => {
                self.crop_mode = match self.crop_mode {
                    CropMode::TopLeft => CropMode::WidthHeight,
                    CropMode::WidthHeight => CropMode::TopLeft,
                };
            }
            KeyCode::Char('i') => {
                self.settings.invert = !self.settings.invert;
                self.do_render = true;
            }
            KeyCode::Char('m') => {
                self.settings.toggle_mode();
                self.do_render = true;
            }
            KeyCode::Left => {
                match self.crop_mode {
                    CropMode::TopLeft => {
                        self.settings.crop.0 = self.settings.crop.0.saturating_add(1);
                    }
                    CropMode::WidthHeight => {
                        self.settings.crop.2 = self.settings.crop.2.saturating_sub(1);
                    }
                }
                self.do_render = true;
            }
            KeyCode::Right => {
                match self.crop_mode {
                    CropMode::TopLeft => {
                        self.settings.crop.0 = self.settings.crop.0.saturating_sub(1);
                    }
                    CropMode::WidthHeight => {
                        self.settings.crop.2 = self.settings.crop.2.saturating_add(1);
                    }
                }
                self.do_render = true;
            }
            KeyCode::Up => {
                match self.crop_mode {
                    CropMode::TopLeft => {
                        self.settings.crop.1 = self.settings.crop.1.saturating_add(1);
                    }
                    CropMode::WidthHeight => {
                        self.settings.crop.3 = self.settings.crop.3.saturating_sub(1);
                    }
                }
                self.do_render = true;
            }
            KeyCode::Down => {
                match self.crop_mode {
                    CropMode::TopLeft => {
                        self.settings.crop.1 = self.settings.crop.1.saturating_sub(1);
                    }
                    CropMode::WidthHeight => {
                        self.settings.crop.3 = self.settings.crop.3.saturating_add(1);
                    }
                }
                self.do_render = true;
            }
            KeyCode::Char(c) => {
                if let Some(palette) = c.to_digit(10) {
                    if !self.settings.set_palette(palette as usize) {
                        self.status = format!("Invalid palette: {}", palette);
                    } else {
                        self.do_render = true;
                    }
                }
            }
            _ => {}
        }
    }
}

fn main() -> io::Result<()> {
    let args = Args::parse();

    // check if /cache dir exists, if not create it
    if !Path::new("cache").exists() {
        fs::create_dir("cache").unwrap();
    }

    enable_raw_mode()?;
    let mut terminal = ratatui::init();
    // terminal.clear()?;

    let app = App::new(args.img);
    let app_state = Arc::new(Mutex::new(app));
    let app_state_clone = Arc::clone(&app_state);

    // ascii render loop
    let worker_thread = thread::spawn(move || {
        worker_loop(&app_state);
    });

    // ui loop
    let ui_thread = thread::spawn(move || {
        loop {
            {
                let mut app = app_state_clone.lock().unwrap();
                if app.exit {
                    break;
                }
                app.tick(&mut terminal)?;
            }
            std::thread::sleep(std::time::Duration::from_millis(16));
        }
        Ok(())
    });

    // join
    let ui_result = ui_thread
        .join()
        .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("UI thread error: {:?}", e)))?;
    let _worker_result = worker_thread.join().map_err(|e| {
        io::Error::new(
            io::ErrorKind::Other,
            format!("Worker thread error: {:?}", e),
        )
    })?;

    disable_raw_mode()?;
    ratatui::restore();
    ui_result
}
