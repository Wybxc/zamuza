fn main() {
    println!("cargo:rerun-if-env-changed=DOCS_RS");
    if std::env::var("DOCS_RS").is_ok() {
        return;
    }
    println!("cargo:rustc-link-lib=tcc")
}
