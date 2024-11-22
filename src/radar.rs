use std::{collections::HashSet, iter, ops::Add};

use thiserror::Error;

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
pub struct Trace {
    moves: Vec<Direction>,
}

#[derive(Debug, Error)]
pub enum TraceMoveError {
    #[error("The move would intersect the path")]
    SelfIntersect,
}

impl Trace {
    const fn new() -> Self {
        Self { moves: Vec::new() }
    }

    fn make_move(&mut self, direction: Direction) -> Result<(), TraceMoveError> {
        let positions = self.path().collect::<Vec<_>>();

        if let Some(last) = positions.last() {
            if positions.contains(&(*last + direction.delta())) {
                return Err(TraceMoveError::SelfIntersect);
            }
        }
        self.moves.push(direction);

        Ok(())
    }

    pub fn path(&self) -> impl Iterator<Item = Offset> + '_ {
        iter::once(Offset::ZERO).chain(self.moves.iter().scan(Offset::ZERO, |s, e| {
            *s = *s + e.delta();
            Some(*s)
        }))
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

    pub fn register_move(&mut self, direction: Direction) -> Result<(), TraceMoveError> {
        self.trace.make_move(direction)
    }

    /// Undo the last move. Returns `true` if there was a move to undo.
    pub fn undo_move(&mut self) -> bool {
        self.trace.moves.pop().is_some()
    }

    pub fn get_possible_paths(&self) -> impl Iterator<Item = Vec<Coordinate>> + use<'_> {
        let path = self.trace.path().collect::<Vec<_>>();

        (0..self.map.size)
            .flat_map(|x| (0..self.map.size).map(move |y| Coordinate::new(x, y)))
            .filter_map(move |origin| {
                if self.map.obstacles.contains(&origin) {
                    return None;
                }

                // check if all path fits in the board and it is not on an obstacle
                path.iter()
                    .map(|&p| {
                        let Ok(coord) = (p + origin.into()).try_into() else {
                            return None;
                        };

                        if self.map.obstacles.contains(&coord) || !self.map.contains(coord) {
                            return None;
                        }

                        Some(coord)
                    })
                    .collect()
            })
    }

    pub const fn map(&self) -> &Map {
        &self.map
    }

    pub const fn trace(&self) -> &Trace {
        &self.trace
    }
}
