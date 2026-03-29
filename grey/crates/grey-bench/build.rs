fn main() {
    let javm_ecrecover = build_javm::build("../../services/bench-ecrecover", "bench-ecrecover");
    let pvm_ecrecover = build_pvm::build("../../services/bench-ecrecover");
    let javm_sieve = build_javm::build("../../services/bench-prime-sieve", "bench-prime-sieve");
    let pvm_sieve = build_pvm::build("../../services/bench-prime-sieve");
    let service_blob = build_javm::build_service("../../services/sample-service", "sample-service");

    let out_dir = std::env::var("OUT_DIR").unwrap();
    std::fs::write(
        format!("{out_dir}/guest_blobs.rs"),
        format!(
            "const GREY_ECRECOVER_BLOB: &[u8] = include_bytes!(\"{}\");\n\
             const POLKAVM_ECRECOVER_BLOB: &[u8] = include_bytes!(\"{}\");\n\
             const GREY_SIEVE_BLOB: &[u8] = include_bytes!(\"{}\");\n\
             const POLKAVM_SIEVE_BLOB: &[u8] = include_bytes!(\"{}\");\n\
             const SAMPLE_SERVICE_BLOB: &[u8] = include_bytes!(\"{}\");\n",
            javm_ecrecover.display(),
            pvm_ecrecover.display(),
            javm_sieve.display(),
            pvm_sieve.display(),
            service_blob.display(),
        ),
    )
    .unwrap();
}
