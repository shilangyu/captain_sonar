use captain_sonar::{
    intel::{InformationPiece, IntelQuestion, Quadrant},
    radar::*,
};
use thiserror::Error;

use std::{collections::HashSet, fmt::Display, io};

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind};
use ratatui::{buffer::Buffer, layout::Rect, text::Text, widgets::Widget, DefaultTerminal, Frame};

fn radar_to_string(radar: &Radar, path: &[DecoratedCoordinate]) -> String {
    let mut result = String::new();
    let mines = path
        .iter()
        .filter_map(|c| if c.has_mine() { Some(c.coord()) } else { None })
        .flat_map(|c| Coordinate::neighbours(&c))
        .collect::<HashSet<_>>();
    let path = path.iter().map(|c| c.coord()).collect::<HashSet<_>>();

    for y in 0..radar.map().size() {
        for x in 0..radar.map().size() {
            let coordinate = Coordinate::new(x, y);
            if radar.map().obstacles().contains(&coordinate) {
                result.push('#');
            } else if path.contains(&coordinate) {
                result.push('*');
            } else if mines.contains(&coordinate) {
                result.push('x');
            } else {
                result.push('.');
            }

            if x != radar.map().size() - 1 {
                result.push_str("  ");
            }
        }
        result.push('\n');
        if y != radar.map().size() - 1 {
            for _ in 0..radar.map().size() * 2 - 1 {
                result.push(' ');
            }
            result.push('\n');
        }
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

#[derive(Debug, Error)]
enum AppError {
    #[error("Error registering move: {0}")]
    Move(TraceMoveError),
}

#[derive(Debug, Clone, Copy)]
enum PickTruthLieKind {
    Quadrant,
    Row,
    Column,
}

impl From<InformationPiece> for PickTruthLieKind {
    fn from(value: InformationPiece) -> Self {
        match value {
            InformationPiece::Quadrant(_) => Self::Quadrant,
            InformationPiece::Row(_) => Self::Row,
            InformationPiece::Column(_) => Self::Column,
        }
    }
}

impl Display for PickTruthLieKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Quadrant => write!(f, "quadrant"),
            Self::Row => write!(f, "row"),
            Self::Column => write!(f, "column"),
        }
    }
}

#[derive(Debug, Clone)]
enum PickTruthLieProgress {
    None,
    Info1Kind {
        kind: PickTruthLieKind,
        data: String,
    },
    Info1Information(InformationPiece),
    Info2Kind {
        info1: InformationPiece,
        kind: PickTruthLieKind,
        data: String,
    },
}

impl PickTruthLieProgress {
    fn previous(&self) -> Option<Self> {
        Some(match self {
            Self::None => return None,
            Self::Info1Kind { kind: _, data } if data.is_empty() => Self::None,
            Self::Info1Kind { kind, data } => Self::Info1Kind {
                kind: *kind,
                data: data[..data.len() - 1].to_string(),
            },
            Self::Info1Information(piece) => Self::Info1Kind {
                kind: match piece {
                    InformationPiece::Quadrant(_) => PickTruthLieKind::Quadrant,
                    InformationPiece::Row(_) => PickTruthLieKind::Row,
                    InformationPiece::Column(_) => PickTruthLieKind::Column,
                },
                data: String::new(),
            },
            Self::Info2Kind { info1, data, .. } if data.is_empty() => {
                Self::Info1Information(*info1)
            }
            Self::Info2Kind { info1, kind, data } => Self::Info2Kind {
                info1: *info1,
                kind: *kind,
                data: data[..data.len() - 1].to_string(),
            },
        })
    }
}

#[derive(Debug, Clone)]
enum Submenu {
    IntelPickQuadrant { quadrant: Option<Quadrant> },
    IntelPickTruthLie(PickTruthLieProgress),
}

impl Submenu {
    fn previous(&self) -> Option<Self> {
        Some(match self {
            Self::IntelPickQuadrant { quadrant: None } => return None,
            Self::IntelPickQuadrant { quadrant: Some(_) } => {
                Self::IntelPickQuadrant { quadrant: None }
            }
            Self::IntelPickTruthLie(progress) => Self::IntelPickTruthLie(progress.previous()?),
        })
    }
}

#[derive(Debug)]
pub struct App {
    exit: bool,
    radar: Radar,
    possible_paths: Vec<Vec<DecoratedCoordinate>>,
    show_path_index: Option<usize>,
    submenu: Option<Submenu>,
    error: Option<AppError>,
}

impl App {
    pub fn new(radar: Radar) -> Self {
        let mut this = Self {
            exit: false,
            radar,
            possible_paths: vec![],
            show_path_index: None,
            submenu: None,
            error: None,
        };

        this.update_possible_paths();

        this
    }

    fn update_possible_paths(&mut self) {
        self.possible_paths = self.radar.get_possible_paths().collect();
        if self.possible_paths.is_empty() {
            self.show_path_index = None;
        } else {
            self.show_path_index = Some(0);
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
        fn base_handling(app: &mut App, key_event: KeyEvent) -> bool {
            match key_event.code {
                KeyCode::Esc => {
                    app.exit();
                }
                KeyCode::Backspace => {
                    if app.error.is_some() {
                        app.error = None;
                    } else if let Some(submenu) = &app.submenu {
                        app.submenu = submenu.previous();
                    } else {
                        return false;
                    }
                }
                _ => return false,
            }

            true
        }

        const fn read_quadrant(key_event: KeyEvent) -> Option<Quadrant> {
            match key_event.code {
                KeyCode::Char('1') => Some(Quadrant::One),
                KeyCode::Char('2') => Some(Quadrant::Two),
                KeyCode::Char('3') => Some(Quadrant::Three),
                KeyCode::Char('4') => Some(Quadrant::Four),
                _ => None,
            }
        }

        const fn read_truth_lie_kind(key_event: KeyEvent) -> Option<PickTruthLieKind> {
            match key_event.code {
                KeyCode::Char('q') => Some(PickTruthLieKind::Quadrant),
                KeyCode::Char('r') => Some(PickTruthLieKind::Row),
                KeyCode::Char('c') => Some(PickTruthLieKind::Column),
                _ => None,
            }
        }

        fn read_information_piece(
            app: &App,
            key_event: KeyEvent,
            kind: PickTruthLieKind,
        ) -> Option<InformationPiece> {
            match kind {
                PickTruthLieKind::Quadrant => {
                    read_quadrant(key_event).map(InformationPiece::Quadrant)
                }
                PickTruthLieKind::Row => panic!("Row should not be handled here"),
                PickTruthLieKind::Column => match key_event.code {
                    KeyCode::Char(c) => {
                        // make sure we don't undeflow
                        if c >= 'a' {
                            let offset = c as u32 - 'a' as u32;
                            if offset < app.radar.map().size() {
                                return Some(InformationPiece::Column(offset));
                            }
                        }

                        None
                    }
                    _ => None,
                },
            }
        }

        const fn read_digit(key_event: KeyEvent) -> Option<char> {
            match key_event.code {
                KeyCode::Char(c @ '0'..='9') => Some(c),
                _ => None,
            }
        }

        if base_handling(self, key_event) {
            return;
        }

        match &self.submenu {
            None => match key_event.code {
                KeyCode::Backspace => {
                    self.radar.undo_trace();
                    self.update_possible_paths();
                }
                KeyCode::Up => {
                    self.error = self
                        .radar
                        .register_move(Move::Directed(Direction::North))
                        .err()
                        .map(AppError::Move);
                    self.update_possible_paths();
                }
                KeyCode::Down => {
                    self.error = self
                        .radar
                        .register_move(Move::Directed(Direction::South))
                        .err()
                        .map(AppError::Move);
                    self.update_possible_paths();
                }
                KeyCode::Left => {
                    self.error = self
                        .radar
                        .register_move(Move::Directed(Direction::West))
                        .err()
                        .map(AppError::Move);
                    self.update_possible_paths();
                }
                KeyCode::Right => {
                    self.error = self
                        .radar
                        .register_move(Move::Directed(Direction::East))
                        .err()
                        .map(AppError::Move);
                    self.update_possible_paths();
                }
                KeyCode::Char('d') => {
                    self.error = self
                        .radar
                        .register_move(Move::Dash)
                        .err()
                        .map(AppError::Move);
                    self.update_possible_paths();
                }
                KeyCode::Char('m') => {
                    self.radar.plant_mine();
                    self.update_possible_paths();
                }
                KeyCode::Char('q') => {
                    self.submenu = Some(Submenu::IntelPickQuadrant { quadrant: None });
                }
                KeyCode::Char('s') => {
                    self.submenu = Some(Submenu::IntelPickTruthLie(PickTruthLieProgress::None));
                }
                KeyCode::Tab => {
                    if let Some(index) = self.show_path_index {
                        self.show_path_index = Some((index + 1) % self.possible_paths.len());
                    }
                }
                _ => (),
            },
            Some(Submenu::IntelPickQuadrant { quadrant: None }) => {
                self.submenu = Some(Submenu::IntelPickQuadrant {
                    quadrant: read_quadrant(key_event),
                });
            }
            &Some(Submenu::IntelPickQuadrant {
                quadrant: Some(quadrant),
            }) => {
                let answer = match key_event.code {
                    KeyCode::Char('y') => true,
                    KeyCode::Char('n') => false,
                    _ => return,
                };

                self.radar
                    .add_intel(IntelQuestion::InQuadrant { quadrant, answer });
                self.submenu = None;
                self.update_possible_paths();
            }
            Some(Submenu::IntelPickTruthLie(progress)) => match progress {
                PickTruthLieProgress::None => {
                    if let Some(kind) = read_truth_lie_kind(key_event) {
                        self.submenu = Some(Submenu::IntelPickTruthLie(
                            PickTruthLieProgress::Info1Kind {
                                kind,
                                data: String::new(),
                            },
                        ));
                    }
                }
                PickTruthLieProgress::Info1Kind {
                    kind: PickTruthLieKind::Row,
                    data,
                } => {
                    if key_event.code == KeyCode::Enter {
                        if let Ok(row) = data.parse::<u32>() {
                            if row <= self.radar.map().size() {
                                let info1 = InformationPiece::Row(row - 1);
                                self.submenu = Some(Submenu::IntelPickTruthLie(
                                    PickTruthLieProgress::Info1Information(info1),
                                ));
                            }
                        }
                    } else if let Some(d) = read_digit(key_event) {
                        let data = data.to_owned() + &d.to_string();
                        self.submenu = Some(Submenu::IntelPickTruthLie(
                            PickTruthLieProgress::Info1Kind {
                                kind: PickTruthLieKind::Row,
                                data,
                            },
                        ));
                    }
                }
                &PickTruthLieProgress::Info1Kind { kind, data: _ } => {
                    if let Some(info) = read_information_piece(self, key_event, kind) {
                        self.submenu = Some(Submenu::IntelPickTruthLie(
                            PickTruthLieProgress::Info1Information(info),
                        ));
                    }
                }
                &PickTruthLieProgress::Info1Information(info1) => {
                    if let Some(kind) = read_truth_lie_kind(key_event) {
                        self.submenu = Some(Submenu::IntelPickTruthLie(
                            PickTruthLieProgress::Info2Kind {
                                info1,
                                kind,
                                data: String::new(),
                            },
                        ));
                    }
                }
                PickTruthLieProgress::Info2Kind {
                    info1,
                    kind: kind @ PickTruthLieKind::Row,
                    data,
                } => {
                    if key_event.code == KeyCode::Enter {
                        if let Ok(row) = data.parse::<u32>() {
                            if row <= self.radar.map().size() {
                                let info2 = InformationPiece::Row(row - 1);
                                self.radar.add_intel(IntelQuestion::TruthLie {
                                    info1: *info1,
                                    info2,
                                });
                                self.submenu = None;
                                self.update_possible_paths();
                            }
                        }
                    } else if let Some(d) = read_digit(key_event) {
                        let data = data.to_owned() + &d.to_string();
                        self.submenu = Some(Submenu::IntelPickTruthLie(
                            PickTruthLieProgress::Info2Kind {
                                info1: *info1,
                                kind: *kind,
                                data,
                            },
                        ));
                    }
                }
                &PickTruthLieProgress::Info2Kind {
                    info1,
                    kind,
                    data: _,
                } => {
                    if let Some(info2) = read_information_piece(self, key_event, kind) {
                        self.radar
                            .add_intel(IntelQuestion::TruthLie { info1, info2 });
                        self.submenu = None;
                        self.update_possible_paths();
                    }
                }
            },
        }
    }

    fn exit(&mut self) {
        self.exit = true;
    }
}

impl Widget for &mut App {
    fn render(self, area: Rect, buf: &mut Buffer) {
        const BASE_INSTRUCTIONS: &str = "
backspace - undo
ESC - quit";

        let instructions = format!(
            "

↑ - north, → - east, ↓ - south, ← - west
tab - next path
d - dash
m - plant mine
q - collect quadrant intel (drone)
s - collect truth/lie intel (sonar)
{}",
            BASE_INSTRUCTIONS
        );

        if let Some(error) = &self.error {
            let text = Text::from(error.to_string() + &instructions);
            text.render(area, buf);
        } else if let Some(submenu) = &self.submenu {
            match submenu {
                Submenu::IntelPickQuadrant { quadrant: None } => {
                    let text = Text::from(
                        "Pick a quadrant (1, 2, 3, 4)".to_string() + "\n" + BASE_INSTRUCTIONS,
                    );
                    text.render(area, buf);
                }
                Submenu::IntelPickQuadrant {
                    quadrant: Some(quadrant),
                } => {
                    let text = Text::from(format!(
                        "In quadrant {}? Pick answer (y / n)\n{}",
                        quadrant, BASE_INSTRUCTIONS
                    ));
                    text.render(area, buf);
                }
                Submenu::IntelPickTruthLie(progress) => {
                    let kind_instruction = "q - quadrant, r - row, c - column";
                    let info_instruction = |kind| match kind {
                        PickTruthLieKind::Quadrant => "1, 2, 3, 4".to_string(),
                        PickTruthLieKind::Row => (1..=self.radar.map().size())
                            .map(|n| n.to_string())
                            .collect::<Vec<_>>()
                            .join(", "),
                        PickTruthLieKind::Column => ('a'..='z')
                            .take(self.radar.map().size() as usize)
                            .map(String::from)
                            .collect::<Vec<_>>()
                            .join(", "),
                    };

                    let text = Text::from(
                        match progress {
                            PickTruthLieProgress::None => {
                                format!("Pick info 1 kind ({kind_instruction})")
                            }
                            PickTruthLieProgress::Info1Kind { kind, data } => {
                                format!(
                                    "Info 1 about {kind} ({}){}",
                                    info_instruction(*kind),
                                    if data.is_empty() {
                                        "".to_string()
                                    } else {
                                        format!(" + ENTER: {data}")
                                    }
                                )
                            }
                            &PickTruthLieProgress::Info1Information(piece) => {
                                format!(
                                    "Info 1 about {} is that it is {piece}.\nPick info 2 kind ({kind_instruction})",
                                    PickTruthLieKind::from(piece),
                                )
                            }
                            PickTruthLieProgress::Info2Kind { info1, kind, data } => {
                                format!(
                                    "Info 1 about {} is that it is {info1}.\nInfo 2 about {kind} ({}){}",
                                    PickTruthLieKind::from(*info1),
                                    info_instruction(*kind),
                                    if data.is_empty() {
                                        "".to_string()
                                    } else {
                                        format!(" + ENTER: {data}")
                                    }
                                )
                            }
                        } + "\n"
                            + BASE_INSTRUCTIONS,
                    );
                    text.render(area, buf);
                }
            }
        } else if let Some(index) = self.show_path_index {
            let path = &self.possible_paths[index];

            let mut s = radar_to_string(&self.radar, path);
            s.push('\n');

            s.push_str(&format!(
                "Possible path: {}/{}",
                index + 1,
                self.possible_paths.len()
            ));

            let text = Text::from(s + &instructions);
            text.render(area, buf);
        } else {
            let text = Text::from("No possible paths".to_string() + &instructions);
            text.render(area, buf);
        }
    }
}
