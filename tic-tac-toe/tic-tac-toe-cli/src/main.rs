//! An interactive terminal frontend for [`tic_tac_toe_lib`].
//!
//! Two players share the keyboard and take turns entering `row column`
//! coordinates until somebody wins or the board fills up.
//!
//! The frontend owns only input parsing and drawing — every rule (which moves
//! are legal, who has won, whether it is a draw) comes from the library, which
//! is what keeps the same logic usable by the bracket-lib frontend.
//!
//! ```text
//! $ cargo run
//! Xavier's turn (X). Enter "row column", or "q" to quit.
//! > 1 1
//! ```

use std::io::{self, BufRead, Write};

use tic_tac_toe_lib::{Board, GameStatus, Player, TicTacToeGame, ai};

/// What a line typed at the prompt asked for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Command {
    /// Play at this zero-indexed coordinate.
    Play {
        /// Requested row.
        row: usize,
        /// Requested column.
        column: usize,
    },
    /// Leave the game.
    Quit,
}

/// Parses one line of input into a [`Command`].
///
/// Accepts two non-negative integers separated by whitespace or a comma, or
/// `q`/`quit`/`exit` to stop. Kept a pure function so the whole input grammar
/// is unit-testable without a terminal.
///
/// # Errors
/// Returns a message suitable for showing to the player.
fn parse_command(line: &str) -> Result<Command, String> {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return Err("Enter two numbers, e.g. `1 2`.".to_string());
    }
    if matches!(trimmed.to_ascii_lowercase().as_str(), "q" | "quit" | "exit") {
        return Ok(Command::Quit);
    }

    let parts: Vec<&str> = trimmed
        .split(|c: char| c.is_whitespace() || c == ',')
        .filter(|s| !s.is_empty())
        .collect();
    let [row, column] = parts.as_slice() else {
        return Err(format!(
            "Expected two numbers separated by a space, got `{trimmed}`."
        ));
    };

    let parse = |what: &str, value: &str| {
        value
            .parse::<usize>()
            .map_err(|_| format!("`{value}` is not a valid {what}."))
    };
    Ok(Command::Play {
        row: parse("row", row)?,
        column: parse("column", column)?,
    })
}

/// Renders the board with row and column rulers so players can read off
/// the coordinates they need to type.
fn render(game: &TicTacToeGame) -> String {
    let mut out = String::new();
    out.push_str("   ");
    for column in 0..game.width() {
        out.push_str(&format!(" {column}"));
    }
    out.push('\n');
    for (i, line) in game.pretty_board().lines().enumerate() {
        // `pretty_board` interleaves cell rows with `-+-` separators, so the
        // board row index advances every other line.
        if i % 2 == 0 {
            out.push_str(&format!("{:>3} ", i / 2));
        } else {
            out.push_str("    ");
        }
        out.push_str(line);
        out.push('\n');
    }
    out
}

/// Describes how the game ended.
fn outcome_message(game: &TicTacToeGame) -> String {
    match game.status() {
        GameStatus::Won(player) => {
            format!("{} ({}) wins!", player.name(), player.symbol())
        }
        GameStatus::Draw => "It's a draw!".to_string(),
        GameStatus::InProgress => "Game abandoned.".to_string(),
    }
}

/// Runs the game to completion, reading commands from `input` and writing all
/// output to `output`.
///
/// Generic over the streams so the loop can be driven by a test with scripted
/// input instead of a live terminal.
///
/// # Errors
/// Returns any I/O error from reading or writing the streams.
fn run(
    game: &mut TicTacToeGame,
    input: &mut impl BufRead,
    output: &mut impl Write,
    computer: Option<char>,
) -> io::Result<()> {
    writeln!(
        output,
        "Tic-Tac-Toe — {} in a row wins.",
        game.how_many_to_win()
    )?;

    let mut line = String::new();
    while !game.is_game_over() {
        write!(output, "\n{}", render(game))?;

        // The computer's whole turn. Every rule it plays by — which squares are
        // legal, who has won, and now which move is best — comes from the
        // library; this frontend only prints the result. That is the same split
        // the Bevy and Godot frontends have, and the reason the opponent lives
        // in `tic-tac-toe-lib::ai` rather than here.
        if computer == Some(game.current_symbol()) {
            match ai::best_move(game) {
                Some(mv) => {
                    let (row, column) = (mv.row(), mv.column());
                    writeln!(
                        output,
                        "\n{} ({}) plays {row} {column}.",
                        game.current_player().name(),
                        game.current_symbol()
                    )?;
                    game.take_turn(row, column)
                        .expect("the library only suggests legal moves");
                }
                None => writeln!(output, "\nThe computer has no move.")?,
            }
            continue;
        }

        let player = game.current_player();
        writeln!(
            output,
            "\n{}'s turn ({}). Enter \"row column\", or \"q\" to quit.",
            player.name(),
            player.symbol()
        )?;
        write!(output, "> ")?;
        output.flush()?;

        line.clear();
        if input.read_line(&mut line)? == 0 {
            // End of input (Ctrl-D, or a test script running out of moves).
            writeln!(output, "\nInput ended.")?;
            break;
        }

        match parse_command(&line) {
            Ok(Command::Quit) => break,
            Ok(Command::Play { row, column }) => {
                if let Err(e) = game.take_turn(row, column) {
                    writeln!(output, "Invalid move: {e}")?;
                }
            }
            Err(message) => writeln!(output, "{message}")?,
        }
    }

    write!(output, "\n{}", render(game))?;
    writeln!(output, "\n{}", outcome_message(game))?;
    Ok(())
}

fn main() -> io::Result<()> {
    // `--vs-computer` puts the searching opponent in O's seat.
    let computer = std::env::args()
        .any(|arg| arg == "--vs-computer")
        .then_some('O');

    let mut game = TicTacToeGame::new(
        Board::new(3, 3),
        vec![
            Player::new(String::from("Xavier"), 'X'),
            Player::new(String::from("Olive"), 'O'),
        ],
        3,
    );

    let stdin = io::stdin();
    let mut input = stdin.lock();
    let mut output = io::stdout();
    run(&mut game, &mut input, &mut output, computer)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn game() -> TicTacToeGame {
        TicTacToeGame::new(
            Board::new(3, 3),
            vec![
                Player::new("Xavier".to_string(), 'X'),
                Player::new("Olive".to_string(), 'O'),
            ],
            3,
        )
    }

    /// Drives `run` with scripted input and returns everything it printed.
    fn play_script(game: &mut TicTacToeGame, script: &str) -> String {
        let mut input = script.as_bytes();
        let mut output = Vec::new();
        run(game, &mut input, &mut output, None).expect("in-memory streams never fail");
        String::from_utf8(output).expect("output is valid UTF-8")
    }

    #[test]
    fn parses_a_space_separated_move() {
        assert_eq!(
            parse_command("1 2"),
            Ok(Command::Play { row: 1, column: 2 })
        );
    }

    #[test]
    fn parses_surrounding_whitespace_and_commas() {
        let expected = Ok(Command::Play { row: 0, column: 2 });
        assert_eq!(parse_command("  0   2  \n"), expected);
        assert_eq!(parse_command("0,2"), expected);
        assert_eq!(parse_command("0, 2"), expected);
    }

    #[test]
    fn parses_quit_in_any_case() {
        for word in ["q", "Q", "quit", "QUIT", "exit", " Exit \n"] {
            assert_eq!(parse_command(word), Ok(Command::Quit), "for input {word:?}");
        }
    }

    #[test]
    fn rejects_malformed_input() {
        for bad in ["", "   ", "1", "1 2 3", "a b", "1 x", "-1 0", "1.5 2"] {
            assert!(parse_command(bad).is_err(), "{bad:?} should be rejected");
        }
    }

    #[test]
    fn render_labels_rows_and_columns() {
        let rendered = render(&game());
        let first = rendered.lines().next().unwrap();
        assert!(first.contains('0') && first.contains('1') && first.contains('2'));
        // Board rows are labelled 0..height.
        assert!(rendered.lines().any(|l| l.starts_with("  0 ")));
        assert!(rendered.lines().any(|l| l.starts_with("  2 ")));
    }

    #[test]
    fn a_scripted_game_can_be_won() {
        let mut game = game();
        // X takes the top row while O answers in the middle row.
        let output = play_script(&mut game, "0 0\n1 0\n0 1\n1 1\n0 2\n");
        assert_eq!(
            game.winner().map(|p| p.name().to_string()),
            Some("Xavier".into())
        );
        assert!(output.contains("Xavier (X) wins!"), "{output}");
    }

    #[test]
    fn a_scripted_game_can_be_drawn() {
        let mut game = game();
        let output = play_script(&mut game, "0 0\n0 1\n0 2\n1 1\n1 0\n1 2\n2 1\n2 0\n2 2\n");
        assert!(game.is_draw());
        assert!(output.contains("It's a draw!"), "{output}");
    }

    #[test]
    fn invalid_moves_are_reported_without_consuming_a_turn() {
        let mut game = game();
        // Occupied cell, then off the board, then a legal move.
        let output = play_script(&mut game, "1 1\n1 1\n9 9\nq\n");
        assert_eq!(game.turn_count(), 1);
        assert!(output.contains("already taken"), "{output}");
        assert!(output.contains("outside the board"), "{output}");
    }

    #[test]
    fn unparseable_input_is_reported_and_the_game_continues() {
        let mut game = game();
        let output = play_script(&mut game, "hello\n1 1\nq\n");
        assert_eq!(game.turn_count(), 1);
        assert!(output.contains("Expected two numbers"), "{output}");
    }

    #[test]
    fn quitting_ends_the_loop_early() {
        let mut game = game();
        let output = play_script(&mut game, "q\n");
        assert_eq!(game.turn_count(), 0);
        assert!(output.contains("Game abandoned."), "{output}");
    }

    #[test]
    fn exhausted_input_ends_the_loop() {
        let mut game = game();
        let output = play_script(&mut game, "0 0\n");
        assert_eq!(game.turn_count(), 1);
        assert!(output.contains("Input ended."), "{output}");
    }

    /// Plays a whole game with the computer in O's seat.
    ///
    /// The human plays a losing line on purpose: the point is that the
    /// computer's moves come from the library and are always legal, and that
    /// the game ends without the frontend ever deciding anything.
    fn play_against_computer(script: &str) -> String {
        let mut game = TicTacToeGame::new(
            Board::new(3, 3),
            vec![
                Player::new(String::from("Human"), 'X'),
                Player::new(String::from("Computer"), 'O'),
            ],
            3,
        );
        let mut input = script.as_bytes();
        let mut output = Vec::new();
        run(&mut game, &mut input, &mut output, Some('O')).expect("in-memory streams never fail");
        String::from_utf8(output).expect("output is utf-8")
    }

    #[test]
    fn the_computer_takes_its_own_turns() {
        let output = play_against_computer("0 0\n1 0\n2 0\n");
        assert!(
            output.contains("plays"),
            "the computer should announce a move"
        );
        // The human is running down column 0, and its third move is
        // refused — because the computer already took that square to block.
        // A fixed script colliding with the reply is not a flaw in the
        // test; it is the opponent working.
        assert!(
            output.contains("Invalid move"),
            "the computer should have blocked column 0 first:\n{output}"
        );
    }

    #[test]
    fn the_computer_never_loses_to_a_greedy_human() {
        // X grabs the top row, which is exactly the threat a perfect O must
        // block. If the frontend ever stopped consulting the library, or the
        // search regressed, X would win here.
        let output = play_against_computer("0 0\n0 1\n0 2\n1 0\n1 1\n2 2\n");
        assert!(
            !output.contains("Human (X) wins"),
            "a perfect opponent lost:\n{output}"
        );
    }
}
