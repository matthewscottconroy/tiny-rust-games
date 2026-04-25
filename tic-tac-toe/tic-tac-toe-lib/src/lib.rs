use std::fmt;

const EMPTY_SYMBOL: char = ' ';

#[derive(Debug)]
pub struct Player {
    name: String,
    symbol: char,
}

#[derive(Debug)]
pub struct Placement {
    row: usize,
    column: usize,
}

#[derive(Debug)]
pub struct Turn {
    player: Player,
    placement: Placement,
}

#[derive(Debug)]
pub struct Board {
    board: Vec<Vec<char>>,
}

#[derive(Debug)]
pub struct TicTacToeGame {
    players: Vec<Player>,
    turn_history: Vec<Turn>,
    how_many_to_win: usize,
    board: Board,
}

impl Board {
    pub fn new(row: usize, column: usize) -> Self {
        Self {
            board: vec![vec![EMPTY_SYMBOL; column]; row],
        }
    }

    pub fn place(&mut self, val: char, row: usize, column: usize) {
        self.board[row][column] = val;
    }

    pub fn is_full(&self) -> bool {
        self.board.iter().all(|row| !row.contains(&EMPTY_SYMBOL))
    }

    pub fn is_entry_empty(&self, row: usize, column: usize) -> bool {
        self.board[row][column] == EMPTY_SYMBOL
    }

    pub fn get_vec(&self) -> &Vec<Vec<char>> {
        &self.board
    }

    pub fn get_width(&self) -> usize {
        self.board[0].len()
    }

    pub fn get_height(&self) -> usize {
        self.board.len()
    }

    fn to_string(&self) -> String {
        let mut result = String::new();
        for row in &self.board {
            result.push_str(&row.iter().collect::<String>());
            result.push('\n');
        }
        result
    }

    fn to_pretty_string(&self) -> String {
        let mut result = String::new();
        for i in 0..self.get_height() {
            for j in 0..self.get_width() {
                result.push(self.get_vec()[i][j]);
                if j != self.get_width() - 1 {
                    result.push('|');
                }
            }
            result.push('\n');
            if i != self.get_height() - 1 {
                for j in 0..self.get_width() {
                    result.push('-');
                    if j != self.get_width() - 1 {
                        result.push('+');
                    }
                }
                result.push('\n');
            }
        }
        result
    }
}

impl fmt::Display for Board {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_string())
    }
}

impl TicTacToeGame {
    pub fn new(board: Board, players: Vec<Player>, how_many: usize) -> Self {
        Self {
            board,
            players,
            how_many_to_win: how_many,
            turn_history: Vec::new(),
        }
    }

    pub fn reset(&mut self) {
        let height = self.board.get_height();
        let width = self.board.get_width();
        self.board = Board::new(height, width);
        self.turn_history.clear();
    }

    pub fn turn_count(&self) -> usize {
        self.turn_history.len()
    }

    pub fn get_number_of_players(&self) -> usize {
        self.players.len()
    }

    pub fn get_player_whose_turn(&self) -> &Player {
        &self.players[self.turn_count() % self.get_number_of_players()]
    }

    pub fn get_current_symbol(&self) -> char {
        self.get_player_whose_turn().symbol
    }

    pub fn get_current_player_name(&self) -> &String {
        &self.get_player_whose_turn().name
    }

    pub fn take_turn(&mut self, row: usize, column: usize) {
        let current_player = self.get_player_whose_turn();
        let turn = Turn {
            player: Player {
                name: current_player.name.clone(),
                symbol: current_player.symbol,
            },
            placement: Placement { row, column },
        };
        self.board.place(self.get_current_symbol(), row, column);
        self.turn_history.push(turn);
    }

    pub fn get_board(&self) -> &Board {
        &self.board
    }

    pub fn get_width(&self) -> usize {
        self.board.get_width()
    }

    pub fn get_height(&self) -> usize {
        self.board.get_height()
    }

    pub fn get_board_string(&self) -> String {
        self.board.to_string()
    }

    pub fn get_pretty_board(&self) -> String {
        self.board.to_pretty_string()
    }

    pub fn get_val_at(&self, row: usize, column: usize) -> char {
        self.get_board().get_vec()[row][column]
    }

    /// Check whether placing at (row, column) is part of a winning line.
    pub fn check_for_win(&self, row: usize, column: usize) -> bool {
        let symbol = self.get_val_at(row, column);
        if symbol == EMPTY_SYMBOL {
            return false;
        }
        let n = self.how_many_to_win;
        let height = self.board.get_height() as isize;
        let width = self.board.get_width() as isize;

        for (dr, dc) in [(0isize, 1isize), (1, 0), (1, 1), (1, -1)] {
            let mut count = 1;
            for sign in [-1isize, 1] {
                let mut r = row as isize + sign * dr;
                let mut c = column as isize + sign * dc;
                while r >= 0 && r < height && c >= 0 && c < width
                    && self.get_val_at(r as usize, c as usize) == symbol
                {
                    count += 1;
                    r += sign * dr;
                    c += sign * dc;
                }
            }
            if count >= n {
                return true;
            }
        }
        false
    }

    /// Scan every filled cell to see if any constitutes a win.
    pub fn do_full_win_check(&self) -> bool {
        for row in 0..self.board.get_height() {
            for col in 0..self.board.get_width() {
                if self.get_val_at(row, col) != EMPTY_SYMBOL && self.check_for_win(row, col) {
                    return true;
                }
            }
        }
        false
    }

    pub fn is_game_over(&self) -> bool {
        self.do_full_win_check() || self.board.is_full()
    }
}

impl Player {
    pub fn new(name: String, symbol: char) -> Self {
        Self { name, symbol }
    }
}
