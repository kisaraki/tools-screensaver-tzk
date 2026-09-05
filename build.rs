use std::env;
use std::error::Error;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::{self, Command};

const BINARY_NAME: &str = "my_datetime_screensaver";
const TARGET: &str = "x86_64-pc-windows-msvc";

fn main() {
    if let Err(error) = build_resources() {
        eprintln!("Windows resource build failed: {error}");
        process::exit(1);
    }
}

fn build_resources() -> Result<(), Box<dyn Error>> {
    for file in [
        "Cargo.toml",
        "resources/resources.rc",
        "resources/resource.h",
        "resources/app.manifest",
        "assets/app.ico",
    ] {
        println!("cargo:rerun-if-changed={file}");
    }
    for variable in ["RC", "PATH", "INCLUDE"] {
        println!("cargo:rerun-if-env-changed={variable}");
    }

    let host = env::var("HOST")?;
    let target = env::var("TARGET")?;
    if host != TARGET || target != TARGET {
        return Err(format!(
            "Phase 0 requires a Windows x64 MSVC host and target; host={host}, target={target}"
        )
        .into());
    }

    let root = PathBuf::from(required_path("CARGO_MANIFEST_DIR")?);
    let output = PathBuf::from(required_path("OUT_DIR")?);
    generate_version(&root, &output)?;
    generate_resource_ids(&root, &output)?;

    let resource_file = output.join("app.res");
    let compiler = env::var_os("RC").unwrap_or_else(|| "rc.exe".into());
    let result = Command::new(&compiler)
        .current_dir(root.join("resources"))
        .arg("/nologo")
        .arg("/i")
        .arg(&output)
        .arg("/fo")
        .arg(&resource_file)
        .arg("resources.rc")
        .output()
        .map_err(|error| {
            io::Error::new(
                error.kind(),
                format!(
                    "cannot execute {}: {error}. Open an x64 Developer Command Prompt with the Windows SDK, or set RC to the full path of rc.exe",
                    Path::new(&compiler).display()
                ),
            )
        })?;

    if !result.status.success() {
        return Err(format!(
            "{} exited with {}\n{}\n{}",
            Path::new(&compiler).display(),
            result.status,
            String::from_utf8_lossy(&result.stdout),
            String::from_utf8_lossy(&result.stderr)
        )
        .into());
    }
    if !resource_file.is_file() {
        return Err("rc.exe reported success without producing app.res".into());
    }

    // Only the GUI executable receives these resources, not the library test harness.
    println!(
        "cargo:rustc-link-arg-bin={BINARY_NAME}={}",
        resource_file.display()
    );
    // The RC file already embeds RT_MANIFEST #1. Do not synthesize another manifest.
    println!("cargo:rustc-link-arg-bin={BINARY_NAME}=/MANIFEST:NO");
    Ok(())
}

fn required_path(name: &str) -> Result<std::ffi::OsString, io::Error> {
    env::var_os(name)
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, format!("Cargo did not set {name}")))
}

fn generate_version(root: &Path, output: &Path) -> Result<(), Box<dyn Error>> {
    let major: u16 = env::var("CARGO_PKG_VERSION_MAJOR")?.parse()?;
    let minor: u16 = env::var("CARGO_PKG_VERSION_MINOR")?.parse()?;
    let patch: u16 = env::var("CARGO_PKG_VERSION_PATCH")?.parse()?;
    let version = env::var("CARGO_PKG_VERSION")?;
    let assembly_version = format!("{major}.{minor}.{patch}.0");
    let manifest = fs::read_to_string(root.join("resources/app.manifest"))?;
    if !manifest.contains(&format!("version=\"{assembly_version}\"")) {
        return Err(
            format!("app.manifest assembly version must match Cargo: {assembly_version}").into(),
        );
    }
    fs::write(
        output.join("version.generated.h"),
        format!(
            "#define APP_VERSION_NUM {major},{minor},{patch},0\n#define APP_VERSION_STR \"{version}\"\n"
        ),
    )?;
    Ok(())
}

fn generate_resource_ids(root: &Path, output: &Path) -> Result<(), Box<dyn Error>> {
    // resource.h is the single source for numeric IDs used by both RC and Rust.
    let header = fs::read_to_string(root.join("resources/resource.h"))?;
    let mut generated = String::from("// Generated from resources/resource.h.\n");
    for line in header.lines() {
        let fields: Vec<_> = line.split_whitespace().collect();
        if fields.first() == Some(&"#define") {
            match fields.as_slice() {
                [_, name, value] => {
                    let id: u16 = value.parse()?;
                    generated.push_str(&format!("pub const {name}: u16 = {id};\n"));
                }
                _ => return Err("resource.h IDs must be '#define NAME decimal_u16'".into()),
            }
        }
    }
    fs::write(output.join("resource_ids.rs"), generated)?;
    Ok(())
}
