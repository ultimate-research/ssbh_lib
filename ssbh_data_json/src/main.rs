use serde::{Deserialize, Serialize};
use ssbh_data::anim_data::AnimData;
use ssbh_data::mesh_data::MeshData;
use ssbh_data::modl_data::ModlData;
use ssbh_data::skel_data::SkelData;
use ssbh_data::SsbhData;
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
        Err(error) => eprintln!("{:?}", error),
    };
}

fn deserialize_and_save<'a, T: SsbhData + Deserialize<'a>>(
    json: &'a str,
    input: &Path,
    output: &Option<PathBuf>,
    extension: &str,
) -> serde_json::Result<()> {
    let data = serde_json::from_str::<T>(&json)?;

    let output_path = output
        .as_ref()
        .map(PathBuf::from)
        .unwrap_or(PathBuf::from(input).with_extension(extension));
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
    let output_path = args
        .get(2)
        .map(PathBuf::from)
        .unwrap_or(PathBuf::from(input.to_string() + ".json"));

    // Try parsing one of the supported formats.
    match input_path.extension().unwrap().to_str().unwrap() {
        "numshb" => parse_and_write_json::<MeshData, _>(input_path, &output_path),
        "nusktb" => parse_and_write_json::<SkelData, _>(input_path, &output_path),
        "nuanmb" => parse_and_write_json::<AnimData, _>(input_path, &output_path),
        "numdlb" => parse_and_write_json::<ModlData, _>(input_path, &output_path),
        "json" => {
            let json = std::fs::read_to_string(&input_path).expect("Failed to read file.");
            let output_path = args.get(2).map(PathBuf::from);

            // Try all available formats.
            // TODO: This could be cleaned up with an SsbhData trait?
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
                .unwrap();
        }
        _ => (),
    };
}
