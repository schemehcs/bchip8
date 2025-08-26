use crossterm::{
    event::{
        self, Event, KeyCode, KeyEventKind, KeyboardEnhancementFlags, PopKeyboardEnhancementFlags,
        PushKeyboardEnhancementFlags,
    },
    execute,
};
use ratatui::style;
use ratatui::{
    DefaultTerminal,
    buffer::Buffer,
    layout::{Position, Rect},
    widgets::{Block, Widget},
};
use std::io;
use std::time::{Duration, Instant};

pub const WIDTH: usize = 64;
pub const HEIGHT: usize = 32;

#[derive(Debug)]
pub enum Key {
    Quit,
    Num(u8),
}

#[derive(Debug)]
pub enum KeyEvent {
    Pressed(Key),
    Released(Key),
}

#[derive(Debug)]
pub struct Screen<'a> {
    display_buffer: &'a [[bool; 64]; 32],
}

impl<'a> Widget for Screen<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let block = Block::default();
        block.render(area, buf);

        // Calculate scaling factors if needed
        let pixel_width = area.width / 64;
        let pixel_height = area.height / 32;

        for y in 0..32 {
            for x in 0..64 {
                if self.display_buffer[y][x] {
                    for py in 0..pixel_height {
                        for px in 0..pixel_width {
                            let rx = area.x + (x as u16) * pixel_width + px;
                            let ry = area.y + (y as u16) * pixel_height + py;
                            buf.cell_mut(Position::new(rx, ry))
                                .unwrap()
                                .set_fg(style::Color::LightGreen)
                                .set_symbol("█");
                        }
                    }
                }
            }
        }
    }
}

impl<'a> Screen<'a> {
    fn new(display_buffer: &'a [[bool; WIDTH]; HEIGHT]) -> Self {
        Screen { display_buffer }
    }
}

pub struct Console {
    terminal: DefaultTerminal,
}

pub fn init() -> anyhow::Result<Console> {
    static mut IS_INIT: bool = false;
    if unsafe { IS_INIT } {
        anyhow::bail!("console can not be initialized twice");
    }
    color_eyre::install().unwrap();
    let terminal = ratatui::init();
    execute!(
        io::stdout(),
        PushKeyboardEnhancementFlags(KeyboardEnhancementFlags::REPORT_EVENT_TYPES)
    )?;

    unsafe { IS_INIT = true };

    Ok(Console::new(terminal))
}

impl Console {
    fn new(terminal: DefaultTerminal) -> Self {
        Console { terminal }
    }

    pub fn restore(&mut self) {
        if let Err(e) = execute!(io::stdout(), PopKeyboardEnhancementFlags) {
            log::error!("err in popping keyboard enhancement flags {}", e);
        }
        ratatui::restore();
    }

    pub fn draw(&mut self, display_buffer: &[[bool; WIDTH]; HEIGHT]) -> anyhow::Result<()> {
        match self
            .terminal
            .draw(|frame| frame.render_widget(Screen::new(display_buffer), frame.area()))
        {
            Ok(_) => Ok(()),
            Err(e) => anyhow::bail!("failed to render screen {}", e),
        }
    }

    pub fn get_key_events(&mut self, timeout: Duration) -> anyhow::Result<Vec<KeyEvent>> {
        let start = Instant::now();
        let no_wait = Duration::from_secs(0);
        let mut keys = vec![];
        while start.elapsed() < timeout {
            if event::poll(no_wait)? {
                match event::read()? {
                    Event::Key(key) => match self.handle_key_code(key.code) {
                        Some(k) => match key.kind {
                            KeyEventKind::Press => keys.push(KeyEvent::Pressed(k)),
                            KeyEventKind::Repeat => keys.push(KeyEvent::Pressed(k)),
                            KeyEventKind::Release => keys.push(KeyEvent::Released(k)),
                        },
                        None => continue,
                    },
                    _ => continue,
                }
            } else {
                return Ok(keys);
            }
        }
        Ok(keys)
    }

    fn handle_key_code(&self, key_code: KeyCode) -> Option<Key> {
        match key_code {
            KeyCode::Esc => Some(Key::Quit),
            KeyCode::Char('1') => Some(Key::Num(1)),
            KeyCode::Char('2') => Some(Key::Num(2)),
            KeyCode::Char('3') => Some(Key::Num(3)),
            KeyCode::Char('4') => Some(Key::Num(0xc)),
            KeyCode::Char('q') => Some(Key::Num(4)),
            KeyCode::Char('w') => Some(Key::Num(5)),
            KeyCode::Char('e') => Some(Key::Num(6)),
            KeyCode::Char('r') => Some(Key::Num(0xd)),
            KeyCode::Char('a') => Some(Key::Num(7)),
            KeyCode::Char('s') => Some(Key::Num(8)),
            KeyCode::Char('d') => Some(Key::Num(9)),
            KeyCode::Char('f') => Some(Key::Num(0xe)),
            KeyCode::Char('z') => Some(Key::Num(0xa)),
            KeyCode::Char('x') => Some(Key::Num(0)),
            KeyCode::Char('c') => Some(Key::Num(0xb)),
            KeyCode::Char('v') => Some(Key::Num(0xf)),
            _ => None,
        }
    }
}
