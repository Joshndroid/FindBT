fn main() {
    println!("cargo:rerun-if-changed=../../assets/icons/FindBT.ico");

    #[cfg(windows)]
    embed_windows_icon();
}

// `winresource` locates rc.exe via the Windows SDK registry entries instead of
// relying on it being on PATH, which is why the previous hand-rolled rc.exe
// invocation kept warning that the icon couldn't be found: rc.exe is only on
// PATH inside a "Developer Command Prompt for VS", not for a plain
// `cargo build` / VS Code task shell.
#[cfg(windows)]
fn embed_windows_icon() {
    let mut res = winresource::WindowsResource::new();
    res.set_icon("../../assets/icons/FindBT.ico");
    if let Err(err) = res.compile() {
        println!(
            "cargo:warning=failed to embed FindBT icon into FindBT.exe ({err}); \
             the executable will use the default icon instead."
        );
    }
}
