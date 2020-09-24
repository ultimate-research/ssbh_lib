use ssbh_lib;
use std::env;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::Instant;

fn main() {
    let args: Vec<String> = env::args().collect();

    let input_path = Path::new(&args[1]);
    let output_path = if args.len() == 3 {
        PathBuf::from(&args[2])
    } else {
        // Modify the input if no output path to allow dragging a file onto the executable.
        PathBuf::from(args[1].to_string() + ".json")
    };

    let parse_start_time = Instant::now();
    let ssbh = ssbh_lib::read_ssbh(&input_path).unwrap();
    let parse_time = parse_start_time.elapsed();
    eprintln!("Parse: {:?}", parse_time);

    let json = serde_json::to_string_pretty(&ssbh).unwrap();

    let mut output_file = std::fs::File::create(output_path).expect("unable to create file");
    output_file
        .write_all(json.as_bytes())
        .expect("unable to write");
}
