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
        let sanitized = redact_sensitive_text(&message.replace(['\r', '\n'], " "));
        let _ = writeln!(file, "{} [{level}] [{module}] {sanitized}", now_text());
    }
}

pub fn redact_sensitive_text(input: &str) -> String {
    let without_paths = redact_path_tokens(input);
    let without_phones = redact_phone_numbers(&without_paths);
    redact_labeled_values(&without_phones)
}

fn redact_path_tokens(input: &str) -> String {
    input
        .split_whitespace()
        .map(|token| redact_path_token(token).unwrap_or_else(|| token.to_string()))
        .collect::<Vec<_>>()
        .join(" ")
}

fn redact_path_token(token: &str) -> Option<String> {
    let path_start = token
        .find(":/")
        .or_else(|| token.find(":\\"))
        .map(|index| index.saturating_sub(1))
        .or_else(|| token.find("\\\\"));
    let path_start = path_start?;
    let (prefix, path_with_suffix) = token.split_at(path_start);
    let suffix_start = path_with_suffix
        .find([',', '，', ';', '；', '|', ')', ']', '}'])
        .unwrap_or(path_with_suffix.len());
    let (path_text, suffix) = path_with_suffix.split_at(suffix_start);
    Some(format!(
        "{prefix}{}{suffix}",
        final_path_component(path_text)
    ))
}

pub fn final_path_component(path_text: &str) -> String {
    let cleaned = path_text.trim_matches(|ch| matches!(ch, '"' | '\'' | '`'));
    cleaned
        .rsplit(['/', '\\'])
        .find(|part| !part.trim().is_empty())
        .unwrap_or(cleaned)
        .to_string()
}

fn redact_phone_numbers(input: &str) -> String {
    let mut output = String::with_capacity(input.len());
    let mut digits = String::new();
    let mut digit_prefix = None;

    for ch in input.chars() {
        if ch.is_ascii_digit() {
            if digits.is_empty() {
                digit_prefix = output.chars().last();
            }
            digits.push(ch);
            continue;
        }
        flush_digits(&mut output, &mut digits, digit_prefix, Some(ch));
        digit_prefix = None;
        output.push(ch);
    }
    flush_digits(&mut output, &mut digits, digit_prefix, None);
    output
}

fn flush_digits(
    output: &mut String,
    digits: &mut String,
    prefix: Option<char>,
    suffix: Option<char>,
) {
    if digits.is_empty() {
        return;
    }
    if should_redact_digits(digits, prefix, suffix) {
        let prefix = &digits[..3.min(digits.len())];
        let suffix_start = digits.len().saturating_sub(2);
        output.push_str(prefix);
        output.push_str("***");
        output.push_str(&digits[suffix_start..]);
    } else {
        output.push_str(digits);
    }
    digits.clear();
}

fn should_redact_digits(digits: &str, prefix: Option<char>, suffix: Option<char>) -> bool {
    digits.len() >= 7
        && !matches!(prefix, Some('_' | '-' | '.'))
        && !matches!(suffix, Some('_' | '-' | '.'))
}

fn redact_labeled_values(input: &str) -> String {
    input
        .split_whitespace()
        .map(redact_labeled_token)
        .collect::<Vec<_>>()
        .join(" ")
}

fn redact_labeled_token(token: &str) -> String {
    let Some((key, value)) = token.split_once('=') else {
        return token.to_string();
    };
    let normalized_key = key
        .trim_matches(|ch: char| !ch.is_ascii_alphanumeric() && ch != '_')
        .to_ascii_lowercase();
    let redacted = if is_phone_key(&normalized_key) {
        redact_phone_numbers(value)
    } else if is_address_key(&normalized_key) {
        redact_address(value)
    } else if is_path_key(&normalized_key) {
        final_path_component(value)
    } else if is_name_key(&normalized_key) {
        redact_name(value)
    } else {
        return token.to_string();
    };
    format!("{key}={redacted}")
}

fn is_phone_key(key: &str) -> bool {
    key.contains("phone") || key.contains("tel") || key.contains("mobile")
}

fn is_address_key(key: &str) -> bool {
    key.contains("address") || key.contains("addr")
}

fn is_name_key(key: &str) -> bool {
    key == "name"
        || key.contains("customer")
        || key.contains("supplier")
        || key.contains("merchant")
}

fn is_path_key(key: &str) -> bool {
    key.contains("path") || key.contains("file") || key.contains("dir")
}

fn redact_address(value: &str) -> String {
    let cleaned = value.trim_matches(|ch| matches!(ch, '"' | '\'' | '`' | ',' | '，'));
    if cleaned.chars().count() <= 4 {
        "***".to_string()
    } else {
        format!("{}***", cleaned.chars().take(4).collect::<String>())
    }
}

fn redact_name(value: &str) -> String {
    let cleaned = value.trim_matches(|ch| matches!(ch, '"' | '\'' | '`' | ',' | '，'));
    let chars = cleaned.chars().collect::<Vec<_>>();
    if chars.len() <= 2 {
        "***".to_string()
    } else {
        format!("{}*{}", chars[0], chars[chars.len() - 1])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn writes_sanitized_log_line_to_daily_file() {
        let dir = tempfile::tempdir().unwrap();
        init(dir.path());
        let active_dir = LOG_DIR.get().unwrap().clone();

        info("logger-test", "第一行\r\n第二行");

        let date = chrono::Local::now().format("%Y%m%d").to_string();
        let path = active_dir.join(format!("easyinventory_{date}.log"));
        let content = std::fs::read_to_string(path).unwrap();

        assert!(content.contains("[INFO] [logger-test] 第一行 第二行"));
        assert!(!content.contains('\r'));
    }

    #[test]
    fn redacts_common_sensitive_text() {
        let redacted = redact_sensitive_text(
            "phone=13800000000 address=广东省深圳市南山区 customer=测试客户 documentPath=C:/Users/ww/Desktop/work/orders/客户A/20260601001_客户A.xlsx",
        );

        assert!(redacted.contains("phone=138***00"));
        assert!(redacted.contains("address=广东省深***"));
        assert!(redacted.contains("customer=测*户"));
        assert!(redacted.contains("20260601001_客户A.xlsx"));
        assert!(!redacted.contains("13800000000"));
        assert!(!redacted.contains("C:/Users/ww/Desktop/work"));
    }

    #[test]
    fn keeps_short_names_private_and_paths_to_final_component() {
        let redacted =
            redact_sensitive_text("supplier=张三 backupPath=C:\\tmp\\backup\\inventory.db");

        assert!(redacted.contains("supplier=***"));
        assert!(redacted.contains("backupPath=inventory.db"));
        assert!(!redacted.contains("C:\\tmp\\backup"));
    }
}
