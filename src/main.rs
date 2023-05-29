use std::{
    io::Write,
    path::{Path, PathBuf},
};

use clap::Parser;

#[derive(Parser)]
#[clap(
    name = "finch",
    about = "A small CLI program to compile an asset directory into a C header file."
)]
struct Opt {
    directory: PathBuf,
    output: Option<String>,
    #[clap(short, long)]
    c_file: bool,
}

fn main() {
    let opt = Opt::parse();

    let directory = match opt.directory.canonicalize() {
        Ok(path) => path,
        Err(_) => {
            eprintln!("Error: Invalid directory path.");
            std::process::exit(1);
        }
    };

    if !directory.is_dir() {
        eprintln!("Error: Path is not a directory.");
        std::process::exit(1);
    }

    let output_name = match opt.output {
        Some(path) => path,
        None => directory.file_stem().unwrap().to_str().unwrap().to_string(),
    };

    let output_header = format!("{}.h", output_name);

    let mut output = match std::fs::File::create(output_header) {
        Ok(file) => file,
        Err(_) => {
            eprintln!("Error: Invalid output path.");
            std::process::exit(1);
        }
    };

    generate_header(&directory, &output_name, &mut output);

    if opt.c_file {
        let output_impl = format!("{}.c", output_name);

        output = match std::fs::File::create(output_impl) {
            Ok(file) => file,
            Err(_) => {
                eprintln!("Error: Invalid output path.");
                std::process::exit(1);
            }
        };
    }

    generate_impl(&directory, &output_name, &mut output, !opt.c_file);
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
enum AssetOutputType {
    String,
    Bytes,
}

impl AssetOutputType {
    pub fn guess_from_filepath(path: &Path) -> Self {
        let extension = path.extension();

        if let Some(extension) = extension {
            if let Some(extension) = extension.to_str() {
                match extension {
                    "txt" | "json" | "xml" | "csv" | "html" | "htm" | "css" | "js" | "md"
                    | "toml" | "rs" | "glsl" | "frag" | "vert" => Self::String,
                    _ => Self::Bytes,
                }
            } else {
                Self::Bytes
            }
        } else {
            Self::Bytes
        }
    }
}

fn generate_header(directory: &Path, output_name: &str, output: &mut impl Write) {
    writeln!(output, "#ifndef {}_H", output_name.to_uppercase()).unwrap();
    writeln!(output, "#define {}_H", output_name.to_uppercase()).unwrap();

    writeln!(output, "#include <stdint.h>").unwrap();
    writeln!(output, "#include <stddef.h>").unwrap();

    writeln!(output, "#ifdef __cplusplus").unwrap();
    writeln!(output, "extern \"C\" {{").unwrap();
    writeln!(output, "#endif").unwrap();

    writeln!(output, "typedef struct {{").unwrap();

    for entry in directory.read_dir().unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();

        struct_fieldify(&path, output);
    }

    writeln!(output, "}} __{}_t;", output_name).unwrap();

    writeln!(
        output,
        "extern const __{}_t {};",
        output_name, output_name
    )
    .unwrap();

    writeln!(output, "#ifdef __cplusplus").unwrap();
    writeln!(output, "}}").unwrap();
    writeln!(output, "#endif").unwrap();

    writeln!(output, "#endif").unwrap();
}

fn struct_fieldify(path: &Path, output: &mut impl Write) {
    let name = path.file_stem().unwrap().to_str().unwrap();
    let name = name.replace('-', "_");

    if path.is_dir() {
        writeln!(output, "struct {{").unwrap();

        for entry in path.read_dir().unwrap() {
            let entry = entry.unwrap();
            let path = entry.path();

            struct_fieldify(&path, output);
        }

        writeln!(output, "}} {};", name).unwrap();
    } else {
        let output_type = AssetOutputType::guess_from_filepath(path);
        let filesize = path.metadata().unwrap().len();

        match output_type {
            AssetOutputType::String => {
                writeln!(output, "const char {}[{} + 1];", name, filesize).unwrap();
                writeln!(output, "const size_t {}_len;", name).unwrap();
            }
            AssetOutputType::Bytes => {
                writeln!(output, "const uint8_t {}[{}];", name, filesize).unwrap();
                writeln!(output, "const size_t {}_len;", name).unwrap();
            }
        }
    }
}

fn generate_impl(directory: &Path, output_name: &str, output: &mut impl Write, single_file: bool) {
    if single_file {
        writeln!(
            output,
            "#ifdef {}_IMPLEMENTATION",
            output_name.to_uppercase()
        )
        .unwrap();
    } else {
        writeln!(output, "#include \"{}.h\"", output_name).unwrap();
    }

    writeln!(output, "#include <stddef.h>").unwrap();
    writeln!(output, "#include <stdint.h>").unwrap();

    writeln!(output, "#ifdef __cplusplus").unwrap();
    writeln!(output, "extern \"C\" {{").unwrap();
    writeln!(output, "#endif").unwrap();

    writeln!(
        output,
        "const __{}_t {} = {{",
        output_name, output_name
    )
    .unwrap();

    for entry in directory.read_dir().unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();

        struct_fieldify_impl(&path, output);
    }

    writeln!(output, "}};").unwrap();

    writeln!(output, "#ifdef __cplusplus").unwrap();
    writeln!(output, "}}").unwrap();
    writeln!(output, "#endif").unwrap();

    if single_file {
        writeln!(
            output,
            "#undef {}_IMPLEMENTATION",
            output_name.to_uppercase()
        )
        .unwrap();

        writeln!(output, "#endif").unwrap();
    }
}

fn struct_fieldify_impl(path: &Path, output: &mut impl Write) {
    if path.is_dir() {
        writeln!(output, "{{").unwrap();

        for entry in path.read_dir().unwrap() {
            let entry = entry.unwrap();
            let path = entry.path();

            struct_fieldify_impl(&path, output);
        }

        writeln!(output, "}},").unwrap();
    } else {
        let output_type = AssetOutputType::guess_from_filepath(path);

        match output_type {
            AssetOutputType::String => {
                let contents = std::fs::read_to_string(path).unwrap();
                let contents = contents.replace('\n', "\\n");
                let contents = contents.replace('\r', "\\r");
                let contents = contents.replace('\t', "\\t");
                let contents = contents.replace('\"', "\\\"");

                let contents_len = contents.len();

                writeln!(output, "\"{}\",", contents).unwrap();
                writeln!(output, "{},", contents_len).unwrap();
            }
            AssetOutputType::Bytes => {
                let contents = std::fs::read(path).unwrap();
                let contents_len = contents.len();

                writeln!(output, "{{").unwrap();

                const BYTES_PER_LINE: usize = 16;

                let mut bytes_in_line = 0;

                for byte in contents {
                    write!(output, "0x{:02x}, ", byte).unwrap();

                    bytes_in_line += 1;

                    if bytes_in_line == BYTES_PER_LINE {
                        writeln!(output).unwrap();
                        bytes_in_line = 0;
                    }
                }

                if bytes_in_line != 0 {
                    writeln!(output).unwrap();
                }

                writeln!(output, "}},").unwrap();
                writeln!(output, "{},", contents_len).unwrap();
            }
        }
    }
}
