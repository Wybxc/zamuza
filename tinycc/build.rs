fn main() {
    if std::env::var("DOCS_RS").is_ok() {
        return;
    }
    println!("cargo:rustc-link-lib=tcc")
}
