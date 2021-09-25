use serde::Serialize;
use ssbh_data::anim_data::AnimData;
use ssbh_data::mesh_data::MeshData;
use ssbh_data::modl_data::ModlData;
use ssbh_data::skel_data::SkelData;
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
        eprintln!("\tssbh_data_json <file>");
        eprintln!("\tssbh_data_json <file> <json output>");
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
        "numshb" => {
            match MeshData::from_file(&input_path) {
                Ok(mesh) => {
                    eprintln!("Parse: {:?}", parse_start_time.elapsed());
                    write_json(&output_path, mesh);
                }
                Err(error) => eprintln!("{:?}", error),
            };
        }
        "nusktb" => {
            match SkelData::from_file(&input_path) {
                Ok(skel) => {
                    eprintln!("Parse: {:?}", parse_start_time.elapsed());
                    write_json(&output_path, skel);
                }
                Err(error) => eprintln!("{:?}", error),
            };
        }
        "nuanmb" => {
            match AnimData::from_file(&input_path) {
                Ok(anim) => {
                    eprintln!("Parse: {:?}", parse_start_time.elapsed());
                    write_json(&output_path, anim);
                }
                Err(error) => eprintln!("{:?}", error),
            };
        }
        "numdlb" => {
            match ModlData::from_file(&input_path) {
                Ok(modl) => {
                    eprintln!("Parse: {:?}", parse_start_time.elapsed());
                    write_json(&output_path, modl);
                }
                Err(error) => eprintln!("{:?}", error),
            };
        }
        "json" => {
            let json = std::fs::read_to_string(&input_path).expect("Failed to read file.");

            // Try all available formats.
            // TODO: This could be cleaned up with an SsbhData trait?
            if let Ok(data) = serde_json::from_str::<MeshData>(&json) {
                let output_path = if args.len() == 3 {
                    PathBuf::from(&args[2])
                } else {
                    PathBuf::from(&input_path).with_extension("numshb")
                };
                data.write_to_file(output_path).unwrap();
            } else if let Ok(data) = serde_json::from_str::<SkelData>(&json) {
                let output_path = if args.len() == 3 {
                    PathBuf::from(&args[2])
                } else {
                    PathBuf::from(&input_path).with_extension("nusktb")
                };
                data.write_to_file(output_path).unwrap();
            } else if let Ok(data) = serde_json::from_str::<ModlData>(&json) {
                let output_path = if args.len() == 3 {
                    PathBuf::from(&args[2])
                } else {
                    PathBuf::from(&input_path).with_extension("numdlb")
                };
                data.write_to_file(output_path).unwrap();
            } else if let Ok(data) = serde_json::from_str::<AnimData>(&json) {
                let output_path = if args.len() == 3 {
                    PathBuf::from(&args[2])
                } else {
                    PathBuf::from(&input_path).with_extension("nuanmb")
                };
                data.write_to_file(output_path).unwrap();
            } 
        }
        _ => (),
    };
}
