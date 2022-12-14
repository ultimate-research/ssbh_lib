use serde::{Deserialize, Serialize};
use ssbh_data::prelude::*;
use std::env;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::Instant;

fn parse_and_write_json<T: SsbhData + Serialize, P: AsRef<Path>>(input: P, output: P) {
    let parse_start_time = Instant::now();
    match T::from_file(&input) {
        Ok(data) => {
            eprintln!("Parse: {:?}", parse_start_time.elapsed());

            let json = serde_json::to_string_pretty(&data).unwrap();

            let mut output_file = std::fs::File::create(output).expect("unable to create file");
            output_file
                .write_all(json.as_bytes())
                .expect("unable to write");
        }
        Err(error) => eprintln!("{error:?}"),
    };
}

fn deserialize_and_save<'a, T: SsbhData + Deserialize<'a>>(
    json: &'a str,
    input: &Path,
    output: &Option<PathBuf>,
    extension: &str,
) -> serde_json::Result<()> {
    let data = serde_json::from_str::<T>(json)?;

    let output_path = output
        .as_ref()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(input).with_extension(extension));
    data.write_to_file(output_path).unwrap();
    Ok(())
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage:");
        eprintln!("\tssbh_data_json <file>");
        eprintln!("\tssbh_data_json <file> <json output>");
        return;
    }

    let input = args.get(1).unwrap();
    let input_path = Path::new(&input);
    // Modify the input if no output is specified to allow dragging a file onto the executable.
    let output_path: PathBuf = args.get(2).unwrap_or(&(input.to_string() + ".json")).into();

    // Try parsing one of the supported formats.
    match input_path.extension().unwrap().to_str().unwrap() {
        "numshb" => parse_and_write_json::<MeshData, _>(input_path, &output_path),
        "nusktb" => parse_and_write_json::<SkelData, _>(input_path, &output_path),
        "nuanmb" => parse_and_write_json::<AnimData, _>(input_path, &output_path),
        "numdlb" => parse_and_write_json::<ModlData, _>(input_path, &output_path),
        "numatb" => parse_and_write_json::<MatlData, _>(input_path, &output_path),
        "adjb" => parse_and_write_json::<AdjData, _>(input_path, &output_path),
        "numshexb" => parse_and_write_json::<MeshExData, _>(input_path, &output_path),
        "json" => {
            let json = std::fs::read_to_string(input_path).expect("Failed to read file.");
            let output_path = args.get(2).map(PathBuf::from);

            // Try all available formats.
            deserialize_and_save::<MeshData>(&json, input_path, &output_path, "numshb")
                .or_else(|_| {
                    deserialize_and_save::<SkelData>(&json, input_path, &output_path, "nusktb")
                })
                .or_else(|_| {
                    deserialize_and_save::<AnimData>(&json, input_path, &output_path, "nuanmb")
                })
                .or_else(|_| {
                    deserialize_and_save::<ModlData>(&json, input_path, &output_path, "numdlb")
                })
                .or_else(|_| {
                    deserialize_and_save::<MatlData>(&json, input_path, &output_path, "numatb")
                })
                .or_else(|_| {
                    deserialize_and_save::<MeshExData>(&json, input_path, &output_path, "numshexb")
                })
                .or_else(|_| {
                    deserialize_and_save::<AdjData>(&json, input_path, &output_path, "adjb")
                })
                .unwrap();
        }
        _ => (),
    };
}
