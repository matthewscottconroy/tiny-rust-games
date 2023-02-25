use tic_tac_toe_lib::Board;

fn main() {
    println!("Program has started");
    let mut b = Board::new(3, 3);
    println!("printing board...");
    println!("{:?}", b);
    b.place('X', 2, 1);
    println!("printing board...");
    println!("{:?}", b);
    println!("Program has ended");
}
