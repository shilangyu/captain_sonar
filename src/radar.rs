use std::{collections::HashSet, ops::Add};

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

pub enum TraceMoveError {
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

    pub fn get_possible_starts(&self) -> HashSet<Coordinate> {
        let mut starts = HashSet::new();
        let path = self.trace.path().collect::<Vec<_>>();

        for x in 0..self.map.size {
            for y in 0..self.map.size {
                let origin = Coordinate::new(x, y);
                if self.map.obstacles.contains(&origin) {
                    continue;
                }
                let mut valid = true;

                // check if all path fits in the board and it is not on an obstacle
                for &p in &path {
                    let Ok(coord) = (p + origin.into()).try_into() else {
                        valid = false;
                        break;
                    };

                    if self.map.obstacles.contains(&coord) || !self.map.contains(coord) {
                        valid = false;
                        break;
                    }
                }

                if valid {
                    starts.insert(origin);
                }
            }
        }

        starts
    }

    pub const fn map(&self) -> &Map {
        &self.map
    }

    pub const fn trace(&self) -> &Trace {
        &self.trace
    }
}
