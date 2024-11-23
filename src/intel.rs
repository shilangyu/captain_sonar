use std::fmt::Display;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Quadrant {
    One,
    Two,
    Three,
    Four,
}

impl Display for Quadrant {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::One => "1",
            Self::Two => "2",
            Self::Three => "3",
            Self::Four => "4",
        };
        write!(f, "{}", s)
    }
}

#[derive(Debug, Clone, Copy)]
pub enum InformationPiece {
    Quadrant(Quadrant),
    Column(u32),
    Row(u32),
}

#[derive(Debug, Clone, Copy)]
pub enum IntelQuestion {
    /// aka drone
    InQuadrant { quadrant: Quadrant, answer: bool },
    TruthLie {
        truth: InformationPiece,
        lie: InformationPiece,
    },
}
