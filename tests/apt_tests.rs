// tests/apt_tests.rs
use meaupdater::apt::parse_apt_list_output;
use meaupdater::model::{PackageUpdate, UpdateType};

const SAMPLE: &str = r#"Listing...
bash/stable 5.1-2+deb11u1 amd64 [upgradable from: 5.1-2]
openssl/security 1.1.1d-0+deb10u6 amd64 [upgradable from: 1.1.1d-0+deb10u1]
"#;

#[test]
fn parse_empty() {
    let v = parse_apt_list_output("");
    assert!(v.is_empty());
}

#[test]
fn parse_sample() {
    let v = parse_apt_list_output(SAMPLE);
    assert_eq!(v.len(), 2);

    assert_eq!(
        v[0],
        PackageUpdate {
            name: "bash".into(),
            current_version: "5.1-2".into(),
            new_version: "5.1-2+deb11u1".into(),
            update_type: UpdateType::Software,
        }
    );
    assert_eq!(
        v[1],
        PackageUpdate {
            name: "openssl".into(),
            current_version: "1.1.1d-0+deb10u1".into(),
            new_version: "1.1.1d-0+deb10u6".into(),
            update_type: UpdateType::Security,
        }
    );
}
