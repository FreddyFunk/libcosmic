//! Global syntax highlighting system
//!
//! Provides a singleton `SyntaxSystem` containing syntax definitions and themes
//! for syntax highlighting, following the cosmic-edit pattern.

use cosmic_text::SyntaxSystem;
use std::io;
use std::sync::OnceLock;

/// Global syntax system singleton
static SYNTAX_SYSTEM: OnceLock<SyntaxSystem> = OnceLock::new();

/// Get the global syntax system, initializing it on first access.
pub fn syntax_system() -> &'static SyntaxSystem {
    SYNTAX_SYSTEM.get_or_init(|| {
        // Load extended themes from two-face
        let lazy_theme_set = two_face::theme::LazyThemeSet::from(two_face::theme::extra());
        let mut theme_set = syntect::highlighting::ThemeSet::from(&lazy_theme_set);

        // Add COSMIC themes (dark and light)
        for (theme_name, theme_data) in &[
            ("COSMIC Dark", cosmic_syntax_theme::COSMIC_DARK_TM_THEME),
            ("COSMIC Light", cosmic_syntax_theme::COSMIC_LIGHT_TM_THEME),
        ] {
            let mut cursor = io::Cursor::new(theme_data);
            match syntect::highlighting::ThemeSet::load_from_reader(&mut cursor) {
                Ok(mut theme) => {
                    // Use transparent background so libcosmic handles container styling
                    theme.settings.background = Some(syntect::highlighting::Color {
                        r: 0,
                        g: 0,
                        b: 0,
                        a: 0,
                    });
                    theme.settings.gutter = Some(syntect::highlighting::Color {
                        r: 0,
                        g: 0,
                        b: 0,
                        a: 0,
                    });
                    theme_set.themes.insert(theme_name.to_string(), theme);
                }
                Err(err) => {
                    tracing::error!("Failed to load {:?} syntax theme: {}", theme_name, err);
                }
            }
        }

        SyntaxSystem {
            // Use extended syntax definitions from two-face
            syntax_set: two_face::syntax::extra_no_newlines(),
            theme_set,
        }
    })
}

/// Get the theme name for the current mode (dark/light)
pub fn theme_name_for_mode(is_dark: bool) -> &'static str {
    if is_dark {
        "COSMIC Dark"
    } else {
        "COSMIC Light"
    }
}

/// Get the syntect theme for the current mode
pub fn theme_for_mode(is_dark: bool) -> Option<&'static syntect::highlighting::Theme> {
    let system = syntax_system();
    let name = theme_name_for_mode(is_dark);
    system.theme_set.themes.get(name)
}
