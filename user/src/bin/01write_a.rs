#![no_std]
#![no_main]

#[macro_use]
extern crate user_lib;

use user_lib::yield_;

const WIDTH: usize = 10;
const HEIGHT: usize = 5;

#[no_mangle]
fn main() -> i32 {
    println!("write_a start!");
    for i in 0..HEIGHT {
        for _ in 0..WIDTH {
            print!("A");
        }
        println!("[{}/{}]", i + 1, HEIGHT);
        yield_();
    }
    println!("write_a done!");
    0
}
