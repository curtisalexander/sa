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

/// Both install URLs pin the release tag, and GitHub 404s a tag or an asset name
/// that does not exist. A version bump has to carry them along.
#[test]
fn readme_install_commands_point_at_this_version() {
    let readme = include_str!("../README.md");

    let find_links = format!("releases/expanded_assets/v{CARGO_VERSION}");
    assert!(
        readme.contains(&find_links),
        "README.md --find-links URL should end in {find_links}"
    );

    let wheel =
        format!("releases/download/v{CARGO_VERSION}/sa-{CARGO_VERSION}-py3-none-win_amd64.whl");
    assert!(
        readme.contains(&wheel),
        "README.md wheel URL should be {wheel}"
    );
}
