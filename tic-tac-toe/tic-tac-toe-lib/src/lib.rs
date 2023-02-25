use std::fmt;

#[derive(Debug)]
pub struct Board {
    board: Vec<Vec<char>>,
}

impl Board {
    pub fn new(row: usize, column: usize) -> Self {
        Self {
            board: vec![vec![' '; column]; row],
        }
    }

    pub fn place(&mut self, val: char, row: usize, column: usize) {
        self.board[row][column] = val;
    }

    /*
    pub fn to_string(&self) -> String {
        format!("{:?}", self)
    }
    */
}

impl fmt::Display for Board {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self)
    }
}
