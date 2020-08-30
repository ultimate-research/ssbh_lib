use binread::BinReaderExt;
use ssbh_lib::formats;
use std::env;
use std::fs::File;
use std::path::Path;
use std::time::{Duration, Instant};

fn main() {
    let args: Vec<String> = env::args().collect();
    let input_path = Path::new(&args[1]);
    let mut file = File::open(input_path).expect("Error opening file.");

    let extension = input_path.extension().unwrap().to_str().unwrap();
    let start = Instant::now();
    match &extension[..] {
        // TODO: Move to function?
        "numatb" => {
            let parse_start = Instant::now();
            let matl = file.read_le::<formats::matl::Matl>().unwrap();
            eprintln!("Parse: {:?}", parse_start.elapsed());

            let json = serde_json::to_string_pretty(&matl).unwrap();
            println!("{}", json);
        }
        "numshb" => {
            let parse_start = Instant::now();
            let mesh = file.read_le::<formats::mesh::Mesh>().unwrap();
            eprintln!("Parse: {:?}", parse_start.elapsed());

            let json = serde_json::to_string_pretty(&mesh).unwrap();
            println!("{}", json);
        }
        _ => eprintln!("Unrecognized file extension {}", extension),
    }

    eprintln!("Total: {:?}", start.elapsed());
}
