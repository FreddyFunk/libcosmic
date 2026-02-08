//! Formatting utilities for file sizes and dates.

/// Format bytes using binary units (1024-based): B, KB, MB, GB
pub fn format_file_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = 1024 * 1024;
    const GB: u64 = 1024 * 1024 * 1024;

    if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.1} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}

/// Format timestamp for display (time only if today, full date otherwise)
pub fn format_modified(modified: std::time::SystemTime) -> String {
    let datetime = chrono::DateTime::<chrono::Local>::from(modified);
    let today = chrono::Local::now().date_naive();

    if datetime.date_naive() == today {
        datetime.format("%H:%M").to_string()
    } else {
        datetime.format("%b %d, %Y, %H:%M").to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_file_size_bytes() {
        assert_eq!(format_file_size(0), "0 B");
        assert_eq!(format_file_size(512), "512 B");
        assert_eq!(format_file_size(1023), "1023 B");
    }

    #[test]
    fn test_format_file_size_kb() {
        assert_eq!(format_file_size(1024), "1.0 KB");
        assert_eq!(format_file_size(1536), "1.5 KB");
        assert_eq!(format_file_size(1024 * 1023), "1023.0 KB");
    }

    #[test]
    fn test_format_file_size_mb() {
        assert_eq!(format_file_size(1024 * 1024), "1.00 MB");
        assert_eq!(format_file_size(1024 * 1024 * 100), "100.00 MB");
    }

    #[test]
    fn test_format_file_size_gb() {
        assert_eq!(format_file_size(1024 * 1024 * 1024), "1.00 GB");
        assert_eq!(format_file_size(1024 * 1024 * 1024 * 2), "2.00 GB");
    }
}
