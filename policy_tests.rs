// tests/policy_tests.rs
use meaupdater::policy::{build_install_command, install_packages};

#[test]
fn build_command_empty() {
    let cmd = build_install_command(&[]);
    assert!(cmd.starts_with("apt update"));
    assert!(cmd.ends_with(' '));
}

#[test]
fn build_command_nonempty() {
    let pkgs = vec!["foo".to_string(), "bar".to_string()];
    let cmd = build_install_command(&pkgs);
    assert_eq!(
        cmd,
        "apt update && apt install --only-upgrade -y foo bar"
    );
}

#[test]
fn install_empty_should_error() {
    let res = install_packages(&[]);
    assert!(res.is_err());
}

