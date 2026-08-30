use std::{io::Write, time::Duration};

/// Log only operation metadata. This API intentionally cannot receive request
/// payloads, credentials, identifiers, post text, or direct-message bodies.
pub fn effect_event(
    operation: &'static str,
    succeeded: bool,
    queue_wait: Duration,
    execution: Duration,
    acknowledgement_wait: Duration,
) {
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
    let _ = writeln!(
        file,
        "{}",
        format_effect_event(
            chrono::Utc::now().to_rfc3339(),
            operation,
            succeeded,
            queue_wait,
            execution,
            acknowledgement_wait,
        )
    );
}

fn format_effect_event(
    timestamp: String,
    operation: &'static str,
    succeeded: bool,
    queue_wait: Duration,
    execution: Duration,
    acknowledgement_wait: Duration,
) -> String {
    let status = if succeeded { "ok" } else { "error" };
    format!(
        "{timestamp} operation={operation:?} status={status} queue_wait_ms={} execution_ms={} ack_wait_ms={}",
        queue_wait.as_millis(),
        execution.as_millis(),
        acknowledgement_wait.as_millis(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn effect_log_exposes_serial_waits_without_secret_content() {
        let rendered = format_effect_event(
            "2026-08-30T00:00:00Z".into(),
            "Direct message request",
            true,
            Duration::from_millis(12),
            Duration::from_millis(34),
            Duration::from_millis(5),
        );
        assert!(rendered.contains("queue_wait_ms=12"));
        assert!(rendered.contains("execution_ms=34"));
        assert!(rendered.contains("ack_wait_ms=5"));
        assert!(!rendered.contains("password"));
        assert!(!rendered.contains("message_body"));
    }
}
