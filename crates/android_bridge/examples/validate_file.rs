use anyhow::{Context, Result};
use compose_android_bridge::Session;
use serde_json::json;
fn main() -> Result<()> {
    let path = std::env::args()
        .nth(1)
        .context("Pass an exported .compose file")?;
    let mut session = Session::new();
    session.command(&json!({"op":"load","path":path}))?;
    session
        .editor
        .document
        .validate()
        .map_err(anyhow::Error::msg)?;
    let page = &session.editor.document.pages[0];
    println!(
        "DESKTOP_ANDROID_FILE_PASS: {} / {} pages / {} first-page layers / {:?}",
        session.editor.document.name,
        session.editor.document.pages.len(),
        page.nodes.len(),
        session.editor.document.mode
    );
    Ok(())
}
