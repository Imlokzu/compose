//! Platform printing of local page snapshots; never submits without a native dialog.
use anyhow::Result;
use document::Document;
use std::path::Path;

#[cfg(target_os = "macos")]
pub struct PreparedPrint {
    directory: tempfile::TempDir,
    helper: std::path::PathBuf,
    manifest: std::path::PathBuf,
}
#[cfg(not(target_os = "macos"))]
pub struct PreparedPrint;

#[cfg(target_os = "macos")]
pub fn prepare(document: &Document) -> Result<PreparedPrint> {
    use std::os::unix::fs::PermissionsExt;
    let directory = tempfile::Builder::new()
        .prefix("compose-print-")
        .tempdir()?;
    let mut renderer = export::PageRenderer::default();
    let mut pages = Vec::new();
    for (index, page) in document.pages.iter().enumerate() {
        let path = directory.path().join(format!("page-{}.png", index + 1));
        let pixels = renderer.render(document, index, 300)?;
        export::save_png(&path, &pixels, 300)?;
        pages.push(serde_json::json!({"path": path, "width": page.width, "height": page.height}));
    }
    let manifest = directory.path().join("print.json");
    std::fs::write(
        &manifest,
        serde_json::to_vec(&serde_json::json!({"name":document.name,"pages":pages}))?,
    )?;
    let helper = directory.path().join("compose-print");
    std::fs::write(
        &helper,
        include_bytes!(concat!(env!("OUT_DIR"), "/compose-print")),
    )?;
    std::fs::set_permissions(&helper, std::fs::Permissions::from_mode(0o700))?;
    Ok(PreparedPrint {
        directory,
        helper,
        manifest,
    })
}
#[cfg(not(target_os = "macos"))]
pub fn prepare(_: &Document) -> Result<PreparedPrint> {
    anyhow::bail!("Native printing is currently available on macOS")
}

impl PreparedPrint {
    /// true means the native operation completed; false means it was cancelled.
    pub fn run(&self) -> Result<bool> {
        self.execute(None, false)
    }
    pub fn save_pdf_for_test(&self, path: &Path) -> Result<bool> {
        self.execute(Some(path), false)
    }
    pub fn check_dialog(&self) -> Result<bool> {
        self.execute(None, true)
    }
    #[cfg(target_os = "macos")]
    fn execute(&self, pdf: Option<&Path>, check: bool) -> Result<bool> {
        use anyhow::{Context, ensure};
        let mut command = std::process::Command::new(&self.helper);
        command
            .arg(&self.manifest)
            .current_dir(self.directory.path());
        if let Some(path) = pdf {
            command.arg("--save-pdf").arg(path);
        }
        if check {
            command.arg("--dialog-check");
        }
        let output = command
            .output()
            .context("Could not open the native print dialog")?;
        if output.status.code() == Some(2) {
            return Ok(false);
        }
        ensure!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr).trim()
        );
        if pdf.is_some() {
            ensure!(
                String::from_utf8_lossy(&output.stdout).contains("PRINT_PDF_PASS"),
                "Print output was not verified"
            );
        }
        if check {
            ensure!(
                String::from_utf8_lossy(&output.stdout).contains("PRINT_DIALOG_CHECK"),
                "Native print dialog did not return"
            );
        }
        Ok(true)
    }
    #[cfg(not(target_os = "macos"))]
    fn execute(&self, _: Option<&Path>, _: bool) -> Result<bool> {
        anyhow::bail!("Native printing is currently available on macOS")
    }
}
