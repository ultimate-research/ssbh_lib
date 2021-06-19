use serde::Serialize;
use ssbh_lib::formats::adj::Adj;
use ssbh_lib::formats::meshex::MeshEx;
use ssbh_lib::{Ssbh, SsbhFile};
use std::env;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::Instant;

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
            match ssbh_lib::formats::adj::Adj::from_file(&input_path) {
                Ok(adjb) => {
                    eprintln!("Parse: {:?}", parse_start_time.elapsed());
                    write_json(&output_path, adjb);
                }
                Err(error) => eprintln!("{:?}", error),
            };
        }
        "numshexb" => {
            match ssbh_lib::formats::meshex::MeshEx::from_file(&input_path) {
                Ok(meshex) => {
                    eprintln!("Parse: {:?}", parse_start_time.elapsed());
                    write_json(&output_path, meshex);
                }
                Err(error) => eprintln!("{:?}", error),
            };
        }
        "json" => {
            let contents = std::fs::read_to_string(&input_path).expect("Failed to read file.");

            // TODO: Clean up repetitive code.

            // Try all available formats.
            if let Ok(ssbh) = serde_json::from_str::<Ssbh>(&contents) {
                // Determine the path based on the SSBH type if no output is specified.
                let output_path = if args.len() == 3 {
                    PathBuf::from(&args[2])
                } else {
                    match ssbh.data {
                        SsbhFile::Hlpb(_) => PathBuf::from(&input_path).with_extension("nuhlpb"),
                        SsbhFile::Matl(_) => PathBuf::from(&input_path).with_extension("numatb"),
                        SsbhFile::Modl(_) => PathBuf::from(&input_path).with_extension("numdlb"),
                        SsbhFile::Mesh(_) => PathBuf::from(&input_path).with_extension("numshb"),
                        SsbhFile::Skel(_) => PathBuf::from(&input_path).with_extension("nusktb"),
                        SsbhFile::Anim(_) => PathBuf::from(&input_path).with_extension("nuanmb"),
                        SsbhFile::Nrpd(_) => PathBuf::from(&input_path).with_extension("nurpdb"),
                        SsbhFile::Nufx(_) => PathBuf::from(&input_path).with_extension("nuflxb"),
                        SsbhFile::Shdr(_) => PathBuf::from(&input_path).with_extension("nushdb"),
                    }
                };

                let export_time = Instant::now();
                ssbh.write_to_file(&output_path)
                    .expect("Failed to write SSBH file.");
                eprintln!("Export: {:?}", export_time.elapsed());
            } else if let Ok(mesh_ex) = serde_json::from_str::<MeshEx>(&contents) {
                let output_path = if args.len() == 3 {
                    PathBuf::from(&args[2])
                } else {
                    PathBuf::from(&input_path).with_extension("numshexb")
                };

                let export_time = Instant::now();
                mesh_ex
                    .write_to_file(&output_path)
                    .expect("Failed to write MESHEX file.");
                eprintln!("Export: {:?}", export_time.elapsed());
            } else if let Ok(adj) = serde_json::from_str::<Adj>(&contents) {
                let output_path = if args.len() == 3 {
                    PathBuf::from(&args[2])
                } else {
                    PathBuf::from(&input_path).with_extension("adjb")
                };

                let export_time = Instant::now();
                adj.write_to_file(&output_path)
                    .expect("Failed to write ADJ file.");
                eprintln!("Export: {:?}", export_time.elapsed());
            }
        }
        _ => {
            match ssbh_lib::Ssbh::from_file(&input_path) {
                Ok(ssbh) => {
                    eprintln!("Parse: {:?}", parse_start_time.elapsed());
                    write_json(&output_path, ssbh);
                }
                Err(error) => eprintln!("{:?}", error),
            };
        }
    };
}
