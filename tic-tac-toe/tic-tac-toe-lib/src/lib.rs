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
        let mut is_empty = false;

        for row in &self.board {
            is_empty = row.contains(&EMPTY_SYMBOL);

            if is_empty {
                break;
            }
        }

        !is_empty
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
        write!(f, "{}", self)
    }
}

impl TicTacToeGame {
    pub fn new(board: Board, players: Vec<Player>, how_many: usize) -> Self {
        Self {
            board: board,
            players: players,
            how_many_to_win: how_many,
            turn_history: Vec::new(),
        }
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
        let turn: Turn = Turn {
            player: Player {
                name: current_player.name.clone(),
                symbol: current_player.symbol,
            },
            placement: Placement {
                row: row,
                column: column,
            },
        };
        self.board.place(self.get_current_symbol(), row, column);
        self.turn_history.push(turn);
    }

    pub fn get_board(&self) -> &Board {
        &self.board
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

    pub fn check_for_win(&self, row: usize, column: usize) -> bool {
        false
    }

    pub fn do_full_win_check(&self) -> bool {
        false
    }

    pub fn is_game_over(&self) -> bool {
        self.do_full_win_check() || self.board.is_full()
    }
}

impl Player {
    pub fn new(name: String, symbol: char) -> Self {
        Self {
            name: name,
            symbol: symbol,
        }
    }
}
