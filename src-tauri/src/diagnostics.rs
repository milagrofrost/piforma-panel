use crate::config::PanelConfig;

pub fn verbose_from_env() -> bool {
    std::env::var("PIFORMA_PANEL_DEBUG")
        .map(|value| matches!(value.as_str(), "1" | "true" | "TRUE" | "yes" | "YES"))
        .unwrap_or(false)
}

pub fn verbose_for_config(config: &PanelConfig) -> bool {
    verbose_from_env() || config.diagnostics.verbose
}
