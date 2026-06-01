use crate::utils::now_text;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

static LOG_DIR: OnceLock<PathBuf> = OnceLock::new();

pub fn init(path: &Path) {
    let _ = LOG_DIR.set(path.to_path_buf());
}

pub fn info(module: &str, message: impl AsRef<str>) {
    write("INFO", module, message.as_ref());
}

pub fn warn(module: &str, message: impl AsRef<str>) {
    write("WARN", module, message.as_ref());
}

pub fn error(module: &str, message: impl AsRef<str>) {
    write("ERROR", module, message.as_ref());
}

fn write(level: &str, module: &str, message: &str) {
    let Some(dir) = LOG_DIR.get() else {
        return;
    };
    let date = chrono::Local::now().format("%Y%m%d").to_string();
    let path = dir.join(format!("easyinventory_{date}.log"));
    if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(path) {
        let sanitized = message.replace(['\r', '\n'], " ");
        let _ = writeln!(file, "{} [{level}] [{module}] {sanitized}", now_text());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn writes_sanitized_log_line_to_daily_file() {
        let dir = tempfile::tempdir().unwrap();
        init(dir.path());

        info("logger-test", "第一行\r\n第二行");

        let date = chrono::Local::now().format("%Y%m%d").to_string();
        let path = dir.path().join(format!("easyinventory_{date}.log"));
        let content = std::fs::read_to_string(path).unwrap();

        assert!(content.contains("[INFO] [logger-test] 第一行  第二行"));
        assert!(!content.contains('\r'));
    }
}
