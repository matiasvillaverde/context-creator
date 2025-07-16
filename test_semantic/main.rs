mod utils;
use utils::helper;

fn main() {
    let result = helper::process("data");
    println!("Result: {}", result);
}