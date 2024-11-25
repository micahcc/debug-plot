use std::env;
use std::error::Error;
use std::fs::File;
use std::io::Write;
use std::path::Path;

const SOURCE_DIR: &str = "src/static";

fn main() -> Result<(), Box<dyn Error>> {
    let out_dir = env::var("OUT_DIR")?;
    let dest_path = Path::new(&out_dir).join("static_files.rs");
    let mut all_the_files = File::create(&dest_path)?;

    writeln!(&mut all_the_files, r##"["##,)?;

    for f in std::fs::read_dir(SOURCE_DIR)? {
        let f = f?;

        if !f.file_type()?.is_file() {
            continue;
        }

        let short = f.file_name().into_string().expect("should be string");
        let mut out_path = std::path::PathBuf::new();
        out_path.push(&out_dir);
        out_path.push(&short);
        std::fs::copy(f.path(), out_path)?;
        writeln!(
            &mut all_the_files,
            r##"("/{short}", include_bytes!(r#"{short}"#)),"##,
        )?;
    }

    writeln!(&mut all_the_files, r##"]"##,)?;

    Ok(())
}
