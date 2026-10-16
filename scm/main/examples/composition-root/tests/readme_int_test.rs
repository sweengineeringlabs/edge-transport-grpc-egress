//! The README quotes the consumer code of `CompositionRoot`; this test fails when the quote and the
//! source differ, so the documentation cannot drift.

use std::error::Error;

fn between_markers(text: &str) -> Result<String, Box<dyn Error>> {
    let start = text
        .find("// BEGIN consumer code")
        .ok_or("no BEGIN marker")?;
    let end = text.find("// END consumer code").ok_or("no END marker")?;
    let body = &text[start..end];
    let first_line_end = body.find('\n').ok_or("empty quote")? + 1;
    Ok(body[first_line_end..].replace("\r\n", "\n"))
}

fn dedent(code: &str) -> String {
    let dedented = code
        .lines()
        .map(|line| line.strip_prefix("    ").unwrap_or(line))
        .collect::<Vec<_>>()
        .join("\n");
    format!("{}\n", dedented.trim_end())
}

#[test]
fn test_readme_quotes_the_consumer_code_of_the_composition_root_exactly(
) -> Result<(), Box<dyn Error>> {
    let source = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/composition_root.rs"
    ))?;
    let readme = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/README.md"))?
        .replace("\r\n", "\n");

    let quoted = dedent(&between_markers(&source)?);

    assert!(
        readme.contains(&quoted),
        "README.md must quote this code of CompositionRoot exactly:\n{quoted}"
    );
    Ok(())
}

#[test]
fn test_between_markers_text_without_markers_is_an_error() {
    assert!(between_markers("fn nothing() {}").is_err());
}
