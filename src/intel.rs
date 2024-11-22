#[derive(Debug, Clone, Copy)]
pub enum Quadrant {
    One,
    Two,
    Three,
    Four,
}

#[derive(Debug, Clone, Copy)]
pub enum InformationPiece {
    Quadrant(Quadrant),
    Column(u32),
    Row(u32),
}

#[derive(Debug, Clone, Copy)]
pub enum IntelQuestion {
    InQuadrant {
        quadrant: Quadrant,
        answer: bool,
    },
    TruthLie {
        truth: InformationPiece,
        lie: InformationPiece,
    },
}
