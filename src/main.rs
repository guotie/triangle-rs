use std::env;
use triangle::triangle::TriAngleArb;

fn main() {
  let args: Vec<String> = env::args().collect();
  let mut ta = TriAngleArb::new(&args[1]);
  ta.start();
}
