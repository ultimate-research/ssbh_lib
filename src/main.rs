use binread::BinReaderExt;
use ssbh_lib::Ssbh;
use std::env;
use std::fs::File;
use std::path::Path;
use std::time::Instant;

fn main() {
    let args: Vec<String> = env::args().collect();
    let input_path = Path::new(&args[1]);
    let mut file = File::open(input_path).expect("Error opening file.");

    let parse_start_time = Instant::now();
    let ssbh = file.read_le::<Ssbh>().unwrap();
    let parse_time = parse_start_time.elapsed();
    eprintln!("Parse: {:?}", parse_time);

    let json = serde_json::to_string_pretty(&ssbh).unwrap();
    println!("{}", json);
}
