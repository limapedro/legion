//! Minimal crossterm-linked binary to measure dependency weight and startup.
fn main() {
    // Do not require a console: just prove the API is linked.
    let _ = crossterm::terminal::size();
    println!("crossterm-linked");
}
