use chrono::{Datelike, Local, NaiveDate};
use std::fs;
use std::path::{Path, PathBuf};

pub fn now_text() -> String {
    Local::now().format("%Y-%m-%d %H:%M:%S").to_string()
}

pub fn today_text() -> String {
    Local::now().format("%Y-%m-%d").to_string()
}

pub fn normalize_date(value: &str) -> String {
    if value.len() >= 10 {
        value[..10].replace('/', "-")
    } else {
        value.to_string()
    }
}

pub fn next_month(date: &str) -> String {
    let normalized = normalize_date(date);
    let parsed = NaiveDate::parse_from_str(&normalized, "%Y-%m-%d")
        .unwrap_or_else(|_| Local::now().date_naive());
    let mut year = parsed.year();
    let mut month = parsed.month() + 1;
    if month > 12 {
        month = 1;
        year += 1;
    }
    format!("{year:04}-{month:02}")
}

pub fn money(value: f64) -> f64 {
    (value * 100.0).round() / 100.0
}

pub fn safe_file_name(input: &str) -> String {
    let mut output = String::new();
    for ch in input.chars() {
        if matches!(ch, '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*') {
            output.push('_');
        } else {
            output.push(ch);
        }
    }
    let trimmed = output.trim();
    if trimmed.is_empty() {
        "未命名".to_string()
    } else {
        trimmed.to_string()
    }
}

pub fn ensure_dir(path: &Path) -> anyhow::Result<PathBuf> {
    fs::create_dir_all(path)?;
    Ok(path.to_path_buf())
}
