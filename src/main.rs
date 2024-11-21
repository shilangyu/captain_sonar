use captain_sonar::radar::*;

use std::{collections::HashSet, io};

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind};
use ratatui::{buffer::Buffer, layout::Rect, text::Text, widgets::Widget, DefaultTerminal, Frame};

fn radar_to_string(radar: &Radar, origin: Coordinate) -> String {
    let mut result = String::new();
    let path = radar
        .trace()
        .path()
        .map(|e| e + origin.into())
        .collect::<HashSet<_>>();

    for y in 0..radar.map().size() {
        for x in 0..radar.map().size() {
            let coordinate = Coordinate::new(x, y);
            if radar.map().obstacles().contains(&coordinate) {
                result.push('#');
            } else if path.contains(&coordinate.into()) {
                result.push('*');
            } else {
                result.push('~');
            }
        }
        result.push('\n');
    }

    result
}

fn main() -> io::Result<()> {
    let map = Map::new(
        10,
        HashSet::from([
            Coordinate::new(1, 2),
            Coordinate::new(5, 1),
            Coordinate::new(8, 3),
            Coordinate::new(3, 4),
            Coordinate::new(1, 5),
            Coordinate::new(8, 6),
            Coordinate::new(3, 7),
            Coordinate::new(5, 7),
        ]),
    );

    let radar = Radar::new(map);

    let mut terminal = ratatui::init();
    let app_result = App::new(radar).run(&mut terminal);
    ratatui::restore();
    app_result
}

#[derive(Debug)]
pub struct App {
    exit: bool,
    radar: Radar,
    possible_starts: Vec<Coordinate>,
    show_start_index: Option<usize>,
}

impl App {
    pub fn new(radar: Radar) -> Self {
        let mut this = Self {
            exit: false,
            radar,
            possible_starts: vec![],
            show_start_index: None,
        };

        this.update_possible_starts();

        this
    }

    fn update_possible_starts(&mut self) {
        self.possible_starts = self.radar.get_possible_starts().into_iter().collect();
        if self.possible_starts.is_empty() {
            self.show_start_index = None;
        } else {
            self.show_start_index = Some(0);
        }
    }

    /// runs the application's main loop until the user quits
    pub fn run(&mut self, terminal: &mut DefaultTerminal) -> io::Result<()> {
        while !self.exit {
            terminal.draw(|frame| self.draw(frame))?;
            self.handle_events()?;
        }
        Ok(())
    }

    fn draw(&mut self, frame: &mut Frame) {
        frame.render_widget(self, frame.area());
    }

    /// updates the application's state based on user input
    fn handle_events(&mut self) -> io::Result<()> {
        if !event::poll(std::time::Duration::from_millis(100))? {
            return Ok(());
        }
        match event::read()? {
            // it's important to check that the event is a key press event as
            // crossterm also emits key release and repeat events on Windows.
            Event::Key(key_event) if key_event.kind == KeyEventKind::Press => {
                self.handle_key_event(key_event);
            }
            _ => {}
        };
        Ok(())
    }

    fn handle_key_event(&mut self, key_event: KeyEvent) {
        match key_event.code {
            KeyCode::Char('q') => {
                self.exit();
            }
            // TODO: propagate error
            KeyCode::Up => {
                self.radar.register_move(Direction::North);
                self.update_possible_starts();
            }
            KeyCode::Down => {
                self.radar.register_move(Direction::South);
                self.update_possible_starts();
            }
            KeyCode::Left => {
                self.radar.register_move(Direction::West);
                self.update_possible_starts();
            }
            KeyCode::Right => {
                self.radar.register_move(Direction::East);
                self.update_possible_starts();
            }
            KeyCode::Tab => {
                if let Some(index) = self.show_start_index {
                    self.show_start_index = Some((index + 1) % self.possible_starts.len());
                }
            }
            _ => (),
        }
    }

    fn exit(&mut self) {
        self.exit = true;
    }
}

impl Widget for &mut App {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if let Some(index) = self.show_start_index {
            let origin = self.possible_starts[index];

            let mut s = radar_to_string(&self.radar, origin);
            s.push('\n');
            s.push('\n');

            s.push_str(&format!(
                "Possible trace: {}/{}",
                index + 1,
                self.possible_starts.len()
            ));

            let text = Text::from(s);
            text.render(area, buf);
        } else {
            let text = Text::from("No possible traces");
            text.render(area, buf);
        }
    }
}
