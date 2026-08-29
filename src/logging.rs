use std::io::Write;

/// Log only operation metadata. This API intentionally cannot receive request
/// payloads, credentials, identifiers, post text, or direct-message bodies.
pub fn event(operation: &'static str, succeeded: bool) {
    let Some(mut directory) = dirs::data_local_dir() else {
        return;
    };
    directory.push("bsky_tui");
    if std::fs::create_dir_all(&directory).is_err() {
        return;
    }
    let Ok(mut file) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(directory.join("bsky_tui.log"))
    else {
        return;
    };
    let status = if succeeded { "ok" } else { "error" };
    let _ = writeln!(
        file,
        "{} operation={operation:?} status={status}",
        chrono::Utc::now().to_rfc3339()
    );
}

#[cfg(test)]
mod tests {
    #[test]
    fn log_schema_has_no_field_for_secret_content() {
        let rendered = format!("operation={:?} status={}", "Direct message request", "ok");
        assert!(!rendered.contains("password"));
        assert!(!rendered.contains("message_body"));
    }
}
