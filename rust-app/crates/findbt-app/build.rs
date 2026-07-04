use std::env;
use std::path::PathBuf;
use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=../../assets/icons/FindBT.ico");

    let target = env::var("TARGET").unwrap_or_default();
    let host = env::var("HOST").unwrap_or_default();
    if !target.contains("windows") || !host.contains("windows") {
        return;
    }

    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR"));
    let icon_path = manifest_dir.join("../../assets/icons/FindBT.ico");
    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));
    let rc_path = out_dir.join("findbt.rc");
    let res_path = out_dir.join("findbt.res");

    std::fs::write(
        &rc_path,
        format!(
            "1 ICON \"{}\"\n",
            icon_path.display().to_string().replace('\\', "\\\\")
        ),
    )
    .expect("write Windows icon resource script");

    let status = Command::new("rc.exe")
        .arg("/nologo")
        .arg("/fo")
        .arg(&res_path)
        .arg(&rc_path)
        .status();

    match status {
        Ok(status) if status.success() => {
            println!("cargo:rustc-link-arg={}", res_path.display());
        }
        Ok(status) => {
            println!("cargo:warning=rc.exe failed to compile FindBT icon resource: {status}");
        }
        Err(err) => {
            println!("cargo:warning=rc.exe was not available; FindBT.exe will use the default executable icon: {err}");
        }
    }
}
