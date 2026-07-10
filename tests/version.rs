//! The version lives in three files. Keep them from drifting.

const CARGO_VERSION: &str = env!("CARGO_PKG_VERSION");

#[test]
fn pyproject_matches_cargo_toml() {
    let pyproject = include_str!("../pyproject.toml");
    let expected = format!("version = \"{CARGO_VERSION}\"");
    assert!(
        pyproject.contains(&expected),
        "pyproject.toml should declare {expected}"
    );
}

#[test]
fn python_package_matches_cargo_toml() {
    let init = include_str!("../python/sa/__init__.py");
    let expected = format!("__version__ = \"{CARGO_VERSION}\"");
    assert!(
        init.contains(&expected),
        "python/sa/__init__.py should declare {expected}"
    );
}

#[test]
fn readme_install_command_points_at_this_version() {
    let readme = include_str!("../README.md");
    let expected = format!("sa-{CARGO_VERSION}-py3-none-win_amd64.whl");
    assert!(
        readme.contains(&expected),
        "README.md install command should name {expected}"
    );
}
