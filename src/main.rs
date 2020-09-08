use ssbh_lib;
use std::env;
use std::path::Path;
use std::time::Instant;

fn main() {
    let args: Vec<String> = env::args().collect();
    let input_path = Path::new(&args[1]);

    let parse_start_time = Instant::now();
    let ssbh = ssbh_lib::read_ssbh(&input_path).unwrap();
    let parse_time = parse_start_time.elapsed();
    eprintln!("Parse: {:?}", parse_time);

    let json = serde_json::to_string_pretty(&ssbh).unwrap();
    println!("{}", json);
}
