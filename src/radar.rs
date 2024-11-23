use std::{collections::HashSet, ops::Add};

use thiserror::Error;

use crate::intel::{IntelQuestion, Quadrant};

#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash)]
pub struct Coordinate {
    x: u32,
    y: u32,
}

impl TryFrom<Offset> for Coordinate {
    type Error = ();

    fn try_from(value: Offset) -> Result<Self, Self::Error> {
        if value.x < 0 || value.y < 0 {
            Err(())
        } else {
            Ok(Self::new(value.x as u32, value.y as u32))
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash)]
pub struct Offset {
    x: i32,
    y: i32,
}

impl From<Coordinate> for Offset {
    fn from(value: Coordinate) -> Self {
        Self::new(value.x as i32, value.y as i32)
    }
}

impl Offset {
    const ZERO: Self = Self::new(0, 0);

    const fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }
}

impl Add<Self> for Offset {
    type Output = Self;

    fn add(self, rhs: Self) -> Self {
        Self::new(self.x + rhs.x, self.y + rhs.y)
    }
}

impl Coordinate {
    pub const fn new(x: u32, y: u32) -> Self {
        Self { x, y }
    }
}

#[derive(Debug)]
pub struct Map {
    size: u32,
    obstacles: HashSet<Coordinate>,
}

impl Map {
    pub fn new(size: u32, obstacles: HashSet<Coordinate>) -> Self {
        assert!(
            obstacles.iter().all(|&c| c.x < size && c.y < size),
            "Obstacle out of bounds"
        );

        Self { size, obstacles }
    }

    pub const fn contains(&self, coord: Coordinate) -> bool {
        coord.x < self.size && coord.y < self.size
    }

    pub const fn quadrant_of(&self, coord: Coordinate) -> Option<Quadrant> {
        if !self.contains(coord) {
            return None;
        }

        Some(match (coord.x < self.size / 2, coord.y < self.size / 2) {
            (true, true) => Quadrant::One,
            (false, true) => Quadrant::Two,
            (true, false) => Quadrant::Three,
            (false, false) => Quadrant::Four,
        })
    }

    pub const fn size(&self) -> u32 {
        self.size
    }

    pub const fn obstacles(&self) -> &HashSet<Coordinate> {
        &self.obstacles
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum Direction {
    North,
    East,
    South,
    West,
}

impl Direction {
    const fn delta(&self) -> Offset {
        match self {
            Self::North => Offset::new(0, -1),
            Self::East => Offset::new(1, 0),
            Self::South => Offset::new(0, 1),
            Self::West => Offset::new(-1, 0),
        }
    }
}

#[derive(Debug)]
pub enum Move {
    Directed(Direction),
    Dash,
}

#[derive(Debug)]
pub enum TraceElement {
    Move(Move),
    Intel(IntelQuestion),
}

#[derive(Debug)]
pub struct Trace {
    trace: Vec<TraceElement>,
}

#[derive(Debug, Error)]
pub enum TraceMoveError {
    #[error("The move would intersect the path")]
    SelfIntersect,
}

#[derive(Debug, Clone)]
pub struct OffsetWithIntel {
    offset: Offset,
    intel: Vec<IntelQuestion>,
}

impl Trace {
    const fn new() -> Self {
        Self { trace: Vec::new() }
    }

    fn make_move(&mut self, r#move: Move) -> Result<(), TraceMoveError> {
        match r#move {
            Move::Directed(direction) => {
                let all_self_intersects = self.paths().iter().all(|path| {
                    if let Some(last) = path.last() {
                        if path
                            .iter()
                            .any(|p| p.offset == (last.offset + direction.delta()))
                        {
                            return true;
                        }
                    }

                    false
                });

                if all_self_intersects {
                    return Err(TraceMoveError::SelfIntersect);
                }

                self.trace
                    .push(TraceElement::Move(Move::Directed(direction)));
                Ok(())
            }
            Move::Dash => {
                self.trace.push(TraceElement::Move(Move::Dash));
                Ok(())
            }
        }
    }

    fn undo_trace(&mut self) -> bool {
        self.trace.pop().is_some()
    }

    fn add_intel(&mut self, intel: IntelQuestion) {
        self.trace.push(TraceElement::Intel(intel));
    }

    pub fn paths(&self) -> Vec<Vec<OffsetWithIntel>> {
        let mut paths = vec![vec![OffsetWithIntel {
            offset: Offset::ZERO,
            intel: vec![],
        }]];

        for m in &self.trace {
            match m {
                TraceElement::Move(Move::Directed(direction)) => {
                    for path in &mut paths {
                        let last = path.last().unwrap();
                        let next = OffsetWithIntel {
                            offset: last.offset + direction.delta(),
                            intel: vec![],
                        };
                        path.push(next);
                    }
                }
                TraceElement::Move(Move::Dash) => {
                    let mut new_paths = vec![];

                    for path in &paths {
                        for direction in &[
                            Direction::North,
                            Direction::East,
                            Direction::South,
                            Direction::West,
                        ] {
                            let mut new_path = path.clone();

                            for _ in 0..4 {
                                let last = new_path.last().unwrap();
                                let next = OffsetWithIntel {
                                    offset: last.offset + direction.delta(),
                                    intel: vec![],
                                };

                                if new_path.iter().any(|p| p.offset == next.offset) {
                                    break;
                                }
                                new_path.push(next);
                                new_paths.push(new_path.clone());
                            }
                        }
                    }

                    paths.extend(new_paths);
                }
                TraceElement::Intel(intel) => {
                    for path in &mut paths {
                        let last = path.last_mut().unwrap();
                        last.intel.push(*intel);
                    }
                }
            }
        }

        paths
    }
}

#[derive(Debug)]
pub struct Radar {
    map: Map,
    trace: Trace,
}

impl Radar {
    pub const fn new(map: Map) -> Self {
        Self {
            map,
            trace: Trace::new(),
        }
    }

    pub fn register_move(&mut self, r#move: Move) -> Result<(), TraceMoveError> {
        self.trace.make_move(r#move)
    }

    /// Undo the last trace (move/intel). Returns `true` if there was a trace to undo.
    pub fn undo_trace(&mut self) -> bool {
        self.trace.undo_trace()
    }

    pub fn get_possible_paths(&self) -> impl Iterator<Item = Vec<Coordinate>> + use<'_> {
        let paths = self.trace.paths();

        (0..self.map.size)
            .flat_map(|x| (0..self.map.size).map(move |y| Coordinate::new(x, y)))
            .flat_map(move |origin| {
                if self.map.obstacles.contains(&origin) {
                    return vec![];
                }

                paths
                    .iter()
                    .filter_map(|path| {
                        path.iter()
                            .map(|p| {
                                // check if we are a coordinate
                                let Ok(coord) = (p.offset + origin.into()).try_into() else {
                                    return None;
                                };

                                // check if we are in some quadrant
                                let quadrant = self.map.quadrant_of(coord)?;

                                // check if we are on an obstacle
                                if self.map.obstacles.contains(&coord) {
                                    return None;
                                }

                                // check if intel excludes this coordinate
                                for intel in &p.intel {
                                    match intel {
                                        IntelQuestion::InQuadrant {
                                            quadrant: question_quadrant,
                                            answer,
                                        } => {
                                            let valid = match answer {
                                                true => quadrant == *question_quadrant,
                                                false => quadrant != *question_quadrant,
                                            };

                                            if !valid {
                                                return None;
                                            }
                                        }
                                        IntelQuestion::TruthLie { truth, lie } => {
                                            todo!()
                                        }
                                    }
                                }

                                Some(coord)
                            })
                            .collect()
                    })
                    .collect()
            })
    }

    pub fn add_intel(&mut self, intel: IntelQuestion) {
        self.trace.add_intel(intel);
    }

    pub const fn map(&self) -> &Map {
        &self.map
    }

    pub const fn trace(&self) -> &Trace {
        &self.trace
    }
}
