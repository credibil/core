extern crate cargo_semver_checks;

use cargo_semver_checks::{ActualSemverUpdate, Check, GlobalConfig, ReleaseType, Rustdoc};
use std::io::{self, Write};
use std::process::Command;

fn main() {
    let output = Command::new("cargo")
        .arg("semver-checks")
        .arg("--baseline-rev")
        .arg("v0.1.0")
        // .arg("%(tag)")
        .output()
        .unwrap();

    println!("status: {}", output.status);
    println!("stdout: {}", String::from_utf8_lossy(&output.stdout));

    io::stdout().write_all(&output.stdout).unwrap();
    io::stderr().write_all(&output.stderr).unwrap();

    // assert!(output.status.success());

    // let current = Rustdoc::from_root("test_crates/trait_missing/old/");
    // let baseline = Rustdoc::from_root("test_crates/trait_missing/new/");
    // let mut config = GlobalConfig::new();
    // let mut check = Check::new(current);
    // let check = check.set_baseline(baseline);
    // let report = check.check_release(&mut config).unwrap();
    // assert!(!report.success());

    // let (_crate_name, crate_report) = report.crate_reports().iter().next().unwrap();
    // let required_bump = crate_report.required_bump().unwrap();
    // assert_eq!(required_bump, ReleaseType::Major);

    // // The "old" and "new" crates have the same version.
    // // The detected bump is the minimum-possible SemVer bump in a new release.
    // // Since the crates are v0.1.0, the minimum possible bump is minor.
    // assert_eq!(crate_report.detected_bump(), ActualSemverUpdate::Minor);
}
