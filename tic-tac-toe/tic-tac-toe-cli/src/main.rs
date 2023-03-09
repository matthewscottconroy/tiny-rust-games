use tic_tac_toe_lib::Board;
use tic_tac_toe_lib::Player;
use tic_tac_toe_lib::TicTacToeGame;

fn main() {
    println!("Program has started");
    let mut b = Board::new(5, 3);
    let mut g = TicTacToeGame::new(
        b,
        vec![
            Player::new(String::from("Matt"), 'X'),
            Player::new(String::from("John"), 'O'),
        ],
        4,
    );

    g.take_turn(1, 2);
    g.take_turn(4, 0);
    println!("{}", g.get_board_string());
    println!("{}", g.get_pretty_board());
    println!("Program has ended");
}
