use serde::Serialize;
use ssbh_lib::formats::adj::Adj;
use ssbh_lib::formats::meshex::MeshEx;
use ssbh_lib::{Ssbh, SsbhFile};
use std::env;
use std::error::Error;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::Instant;

fn read_data_write_json<
    T: Serialize,
    P: AsRef<Path> + ToString,
    F: Fn(P) -> Result<T, Box<dyn Error>>,
>(
    input_path: P,
    output_path: Option<&String>,
    read_t: F,
) {
    // Modify the input if no output is specified to allow dragging a file onto the executable.
    let json_output_path = output_path
        .cloned()
        .unwrap_or(input_path.to_string() + ".json");

    let parse_start_time = Instant::now();
    match read_t(input_path) {
        Ok(adjb) => {
            eprintln!("Parse: {:?}", parse_start_time.elapsed());
            write_json(json_output_path, adjb);
        }
        Err(error) => eprintln!("{:?}", error),
    };
}

fn write_json<T: Sized + Serialize, P: AsRef<Path>>(output_path: P, object: T) {
    let json = serde_json::to_string_pretty(&object).unwrap();

    let mut output_file = std::fs::File::create(output_path).expect("unable to create file");
    output_file
        .write_all(json.as_bytes())
        .expect("unable to write");
}

fn read_json_write_data(input_path: &Path, output_path: Option<&String>) {
    // Modify the input if no output is specified to allow dragging a file onto the executable.
    let get_output_path = |ext| {
        output_path
            .map(PathBuf::from)
            .unwrap_or_else(|| input_path.with_extension(ext))
    };

    let json = std::fs::read_to_string(&input_path).expect("Failed to read file.");
    if let Ok(ssbh) = serde_json::from_str::<Ssbh>(&json) {
        // Determine the path based on the SSBH type if no output is specified.
        let output = get_output_path(match ssbh.data {
            SsbhFile::Hlpb(_) => "nuhlpb",
            SsbhFile::Matl(_) => "numatb",
            SsbhFile::Modl(_) => "numdlb",
            SsbhFile::Mesh(_) => "numshb",
            SsbhFile::Skel(_) => "nusktb",
            SsbhFile::Anim(_) => "nuanmb",
            SsbhFile::Nrpd(_) => "nurpdb",
            SsbhFile::Nufx(_) => "nuflxb",
            SsbhFile::Shdr(_) => "nushdb",
        });

        write_data(ssbh, output, Ssbh::write_to_file);
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
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage:");
        eprintln!("\tssbh_lib_json <file>");
        eprintln!("\tssbh_lib_json <file> <json output>");
        return;
    }

    let input = &args[1];
    let input_path = Path::new(input);

    // Try parsing one of the supported formats.
    match input_path.extension().unwrap().to_str().unwrap() {
        "adjb" => read_data_write_json(input, args.get(2), Adj::from_file),
        "numshexb" => read_data_write_json(input, args.get(2), MeshEx::from_file),
        "json" => read_json_write_data(input_path, args.get(2)),
        // Assume anything else is an SSBH file.
        _ => read_data_write_json(input, args.get(2), Ssbh::from_file),
    };
}
