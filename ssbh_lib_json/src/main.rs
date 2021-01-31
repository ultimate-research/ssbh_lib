use binread::Error;
use serde::Serialize;
use ssbh_lib;
use std::env;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::Instant;

fn print_errors(error: Error) {
    match error {
        binread::Error::EnumErrors {
            pos,
            variant_errors,
            ..
        } => {
            eprintln!("EnumErrors at pos {:?}", pos);
            for (_, sub_error) in variant_errors {
                print_errors(sub_error);
            }
        }
        binread::Error::BadMagic { pos, found, .. } => {
            eprintln!("BadMagic at pos {:?}, {:?}", pos, found);
        }
        _ => eprintln!("{:?}", error),
    }
}

fn write_json<T: Sized + Serialize>(output_path: &Path, object: T) {
    let json = serde_json::to_string_pretty(&object).unwrap();

    let mut output_file = std::fs::File::create(output_path).expect("unable to create file");
    output_file
        .write_all(json.as_bytes())
        .expect("unable to write");
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage:");
        eprintln!("\tssbh_lib_json <file>");
        eprintln!("\tssbh_lib_json <file> <json output>");
        return;
    }

    let input_path = Path::new(&args[1]);

    // Modify the input if no output is specified to allow dragging a file onto the executable.
    let output_path = if args.len() == 3 {
        PathBuf::from(&args[2])
    } else {
        PathBuf::from(args[1].to_string() + ".json")
    };

    // Try parsing one of the supported formats.
    let parse_start_time = Instant::now();
    match input_path.extension().unwrap().to_str().unwrap() {
        "adjb" => {
            match ssbh_lib::read_adjb(&input_path) {
                Ok(adjb) => {
                    eprintln!("Parse: {:?}", parse_start_time.elapsed());
                    write_json(&output_path, adjb);
                }
                Err(error) => print_errors(error),
            };
        }
        "numshexb" => {
            match ssbh_lib::read_meshex(&input_path) {
                Ok(meshex) => {
                    eprintln!("Parse: {:?}", parse_start_time.elapsed());
                    write_json(&output_path, meshex);
                }
                Err(error) => print_errors(error),
            };
        }
        _ => {
            match ssbh_lib::read_ssbh(&input_path) {
                Ok(ssbh) => {
                    eprintln!("Parse: {:?}", parse_start_time.elapsed());
                    write_json(&output_path, ssbh);
                }
                Err(error) => print_errors(error),
            };
        }
    };
}
