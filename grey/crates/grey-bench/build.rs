fn main() {
    let javm_blob = build_javm::build("../../services/bench-ecrecover", "bench-ecrecover");
    let pvm_blob = build_pvm::build("../../services/bench-ecrecover");

    let out_dir = std::env::var("OUT_DIR").unwrap();
    std::fs::write(
        format!("{out_dir}/guest_blobs.rs"),
        format!(
            "const GREY_ECRECOVER_BLOB: &[u8] = include_bytes!(\"{}\");\n\
             const POLKAVM_ECRECOVER_BLOB: &[u8] = include_bytes!(\"{}\");\n",
            javm_blob.display(),
            pvm_blob.display(),
        ),
    )
    .unwrap();
}
