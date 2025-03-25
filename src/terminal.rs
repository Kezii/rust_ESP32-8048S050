use std::ffi::c_void;

use embedded_gfx::framebuffer::DmaReadyFramebuffer;
use embedded_graphics::mono_font::MonoFont;
use embedded_graphics::Drawable;
use embedded_graphics::{
    draw_target::DrawTarget,
    geometry::Point,
    mono_font::{ascii::FONT_9X15, MonoTextStyle},
    pixelcolor::{Rgb565, Rgb888},
    primitives::{Line, Primitive, PrimitiveStyle},
    text::Text,
};

use crate::display_driver::FramebufferTarget;
use crate::SCREEN_HEIGHT;

const FONT: MonoFont = FONT_9X15;
const FONT_HEIGHT: usize = 15;
const FONT_WIDTH: usize = 9;

pub struct TerminalState {
    pub history: [String; SCREEN_HEIGHT / FONT_HEIGHT + 2],
    screen_height: usize,
    screen_width: usize,
    pub command_line: String,
    pub previous_command_line: String,
}

impl TerminalState {
    pub fn new(screen_width: usize, screen_height: usize) -> Self {
        const INIT: String = String::new();
        Self {
            history: [INIT; SCREEN_HEIGHT / FONT_HEIGHT + 2],
            screen_height,
            screen_width,
            command_line: String::new(),
            previous_command_line: String::new(),
        }
    }

    fn push(&mut self, line: String) {
        for i in 0..self.history.len() - 1 {
            self.history[i] = self.history[i + 1].clone();
        }
        self.history[self.history.len() - 1] = line;
    }

    fn enter(&mut self) {
        self.println(&format!("> {}", self.command_line));
        self.previous_command_line = self.command_line.clone();
        self.command_line.clear();
    }

    fn arrow_up(&mut self) {
        self.command_line = self.previous_command_line.clone();
    }

    pub fn println(&mut self, line: &str) {
        let max_width = self.screen_width / FONT_WIDTH - 2;

        for l in line.split('\n') {
            let mut line = String::new();
            for c in l.chars() {
                if line.len() > max_width {
                    self.push(line);
                    line = String::new();
                }
                line.push(c);
            }
            self.push(line);
        }
    }
}

pub struct TerminalRenderer<'a, const W: usize, const H: usize> {
    fbuf: DmaReadyFramebuffer<W, H>,
    display: &'a mut dyn FramebufferTarget,
}

impl<'a, const W: usize, const H: usize> TerminalRenderer<'a, W, H> {
    pub fn new(
        fb: *mut u16,
        display: &'a mut impl FramebufferTarget,
    ) -> TerminalRenderer<'a, W, H> {
        let fbuf = DmaReadyFramebuffer::<W, H>::new(fb as *mut c_void, false);

        TerminalRenderer { fbuf, display }
    }

    // drawing is split in 3 parts, to avoid taking a lock on TerminalState for more than necessary

    pub fn draw_graphics(&mut self) {
        self.fbuf.clear(Rgb565::new(0, 1, 0)).unwrap();

        Line::new(
            Point::new(0, H as i32 - 18),
            Point::new(W as i32, H as i32 - 18),
        )
        .into_styled(PrimitiveStyle::with_stroke(
            Rgb888::new(77 >> 3, 85 >> 2, 94 >> 3).into(),
            1,
        ))
        .draw(&mut self.fbuf)
        .unwrap();
    }

    pub fn draw_text(&mut self, state: &TerminalState) {
        let text_style = MonoTextStyle::new(&FONT, Rgb565::new(252, 252, 252));

        Text::new(
            &format!("> {}", state.command_line),
            Point::new(3, H as i32 - 5),
            text_style,
        )
        .draw(&mut self.fbuf)
        .unwrap();

        for (i, row) in state.history.iter().enumerate() {
            let _ = Text::new(
                row,
                Point::new(3, 10 + i as i32 * 13),
                MonoTextStyle::new(&FONT, Rgb565::new(252, 252, 252)),
            )
            .draw(&mut self.fbuf);
        }
    }

    pub fn blit(&mut self) {
        self.display.eat_framebuffer(self.fbuf.as_slice()).unwrap();
    }
}
