use clap::Parser;
use crossterm::event::{Event, KeyEvent};
use image::{DynamicImage, GenericImageView};
use ratatui::{
    crossterm::event::{self, KeyCode, KeyEventKind},
    layout::{self, Constraint, Layout},
    style::Stylize,
    symbols::border,
    widgets::{block::Title, Block, Paragraph},
    DefaultTerminal, Frame,
};

use render::{paint, PaintSettings};
use std::fs;
use std::io;
use std::path::Path;

mod render;

/// CLI interface for a virtual park where AI gathers to do ASCII paintings of your images.
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Image file
    #[arg(short, long)]
    img: String,
}

struct App {
    img: DynamicImage,
    img_dimensions: (u32, u32),
    img_color: image::ColorType,
    settings: PaintSettings,
    result: String,
    status: String,
    exit: bool,
}

impl App {
    fn new(img: DynamicImage) -> Self {
        let img_dimensions = img.dimensions();
        let img_color = img.color();
        Self {
            img,
            img_dimensions,
            img_color,
            settings: PaintSettings::new(img_dimensions),
            result: String::default(),
            status: String::default(),
            exit: false,
        }
    }

    fn run(&mut self, terminal: &mut DefaultTerminal) -> io::Result<()> {
        while !self.exit {
            terminal.draw(|frame| self.draw(frame))?;
            self.handle_events()?;
        }
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

        let canvas = Paragraph::new(self.result.clone())
            .white()
            .on_black()
            .block(
                Block::bordered()
                    .title(Title::from("Canvas"))
                    .border_set(border::ROUNDED),
            );

        let footer = Paragraph::new(format!(
            "Image: {}x{} {:?}",
            self.img_dimensions.0, self.img_dimensions.1, self.img_color
        ))
        .white()
        .on_black();

        frame.render_widget(
            Block::new()
                // .borders(Borders::TOP)
                .title("Ascii in the park")
                .title_alignment(layout::Alignment::Center),
            title_area,
        );
        frame.render_widget(canvas, canvas_area);
        frame.render_widget(
            Block::bordered()
                .title("Settings")
                .border_set(border::ROUNDED),
            settings_area,
        );
        frame.render_widget(footer, footer_area);
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
            KeyCode::Enter => self.result = paint(&self.settings, &self.img),
            KeyCode::Char('i') => {
                self.settings.invert = !self.settings.invert;
                self.result = paint(&self.settings, &self.img);
            }
            KeyCode::Char('m') => {
                self.settings.toggle_mode();
                self.result = paint(&self.settings, &self.img);
            }
            KeyCode::Char(c) => {
                if let Some(palette) = c.to_digit(10) {
                    if !self.settings.set_palette(palette as usize) {
                        self.status = format!("Invalid palette: {}", palette);
                    } else {
                        self.result = paint(&self.settings, &self.img);
                    }
                }
            }
            KeyCode::Left => {
                self.settings.offset.0 = self.settings.offset.0.saturating_add(1);
                self.result = paint(&self.settings, &self.img);
            }
            KeyCode::Right => {
                self.settings.offset.0 = self.settings.offset.0.saturating_sub(1);
                self.result = paint(&self.settings, &self.img);
            }
            KeyCode::Up => {
                self.settings.offset.1 = self.settings.offset.1.saturating_add(1);
                self.result = paint(&self.settings, &self.img);
            }
            KeyCode::Down => {
                self.settings.offset.1 = self.settings.offset.1.saturating_sub(1);
                self.result = paint(&self.settings, &self.img);
            }
            _ => {}
        }
    }

    fn exit(&mut self) {
        self.exit = true;
    }
}

fn main() -> io::Result<()> {
    let args = Args::parse();

    // check if /cache dir exists, if not create it
    if !Path::new("cache").exists() {
        fs::create_dir("cache").unwrap();
    }

    let mut terminal = ratatui::init();
    // terminal.clear()?;

    let img = image::open(&Path::new(&args.img)).unwrap();
    let app_result = App::new(img).run(&mut terminal);
    ratatui::restore();
    app_result
}
