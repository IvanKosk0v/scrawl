//! M0.4: repo gates are configured with the measured MSRV and run deny.

const MSRV: &str = "1.85.1"; // measured with `cargo msrv find`

#[test]
fn cargo_toml_declares_measured_msrv() {
    let manifest = include_str!("../Cargo.toml");
    assert!(
        manifest.contains(&format!("rust-version = \"{MSRV}\"")),
        "rust-version must be {MSRV}"
    );
}

#[test]
fn check_script_runs_cargo_deny() {
    assert!(include_str!("../scripts/check.sh").contains("cargo deny check"));
}
