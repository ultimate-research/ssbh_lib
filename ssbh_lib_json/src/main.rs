use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::Instant;

use clap::Parser;
use serde::Serialize;
use ssbh_lib::prelude::*;

/// Convert SSBH, Meshex, and Adjb files to and from JSON.
#[derive(Parser)]
#[command(author, version, about)]
struct Cli {
    /// The input JSON or binary file path
    input: String,
    /// The output JSON or binary file path.
    /// Set as <input>.json or inferred from the JSON data if not specified.
    output: Option<String>,
}

fn read_data_write_json<T, E, P, F>(input_path: P, output_path: Option<String>, read_t: F)
where
    T: Serialize,
    P: AsRef<Path> + ToString,
    F: Fn(P) -> Result<T, E>,
    E: std::fmt::Debug,
{
    // Modify the input to allow dragging a file onto the executable.
    let output_path = output_path
        .map(|o| PathBuf::from(&o))
        .unwrap_or_else(|| PathBuf::from(&(input_path.to_string() + ".json")));

    let parse_start_time = Instant::now();
    match read_t(input_path) {
        Ok(adjb) => {
            eprintln!("Parse: {:?}", parse_start_time.elapsed());
            write_json(output_path, adjb);
        }
        Err(error) => eprintln!("{error:?}"),
    };
}

fn write_json<T: Sized + Serialize, P: AsRef<Path>>(output_path: P, object: T) {
    let json = serde_json::to_string_pretty(&object).unwrap();

    let mut output_file = std::fs::File::create(output_path).expect("unable to create file");
    output_file
        .write_all(json.as_bytes())
        .expect("unable to write");
}

fn read_json_write_data<P: AsRef<Path>>(input_path: P, output_path: Option<String>) {
    // Modify the input if no output is specified to allow dragging a file onto the executable.
    let get_output_path = |ext| {
        output_path
            .map(PathBuf::from)
            .unwrap_or_else(|| input_path.as_ref().with_extension(ext))
    };

    let json = std::fs::read_to_string(input_path.as_ref()).expect("Failed to read file.");
    if let Ok(ssbh) = serde_json::from_str::<SsbhFile>(&json) {
        // Determine the path based on the SSBH type if no output is specified.
        let output = get_output_path(match ssbh.data {
            Ssbh::Hlpb(_) => "nuhlpb",
            Ssbh::Matl(_) => "numatb",
            Ssbh::Modl(_) => "numdlb",
            Ssbh::Mesh(_) => "numshb",
            Ssbh::Skel(_) => "nusktb",
            Ssbh::Anim(_) => "nuanmb",
            Ssbh::Nrpd(_) => "nurpdb",
            Ssbh::Nufx(_) => "nuflxb",
            Ssbh::Shdr(_) => "nushdb",
        });

        write_data(ssbh, output, SsbhFile::write_to_file);
    } else if let Ok(mesh_ex) = serde_json::from_str::<MeshEx>(&json) {
        write_data(mesh_ex, get_output_path("numshexb"), MeshEx::write_to_file);
    } else if let Ok(adj) = serde_json::from_str::<Adj>(&json) {
        write_data(adj, get_output_path("adjb"), Adj::write_to_file);
    }
}

fn write_data<T, P: AsRef<Path>, F: Fn(&T, P) -> std::io::Result<()>>(
    data: T,
    output_path: P,
    write_t: F,
) {
    let export_time = Instant::now();
    write_t(&data, output_path).expect("Failed to write file.");
    eprintln!("Export: {:?}", export_time.elapsed());
}

fn main() {
    let cli = Cli::parse();

    // Try parsing one of the supported formats.
    match Path::new(&cli.input).extension().unwrap().to_str().unwrap() {
        "adjb" => read_data_write_json(cli.input, cli.output, Adj::from_file),
        "numshexb" => read_data_write_json(cli.input, cli.output, MeshEx::from_file),
        "json" => read_json_write_data(cli.input, cli.output),
        // Assume anything else is an SSBH file.
        _ => read_data_write_json(cli.input, cli.output, SsbhFile::from_file),
    };
}
