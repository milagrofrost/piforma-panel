use crate::config::{config_dimension, PanelConfig};
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct PanelGeometry {
    pub monitor_id: Option<String>,
    pub monitor_origin_x: i32,
    pub monitor_origin_y: i32,
    pub monitor_width: Option<u32>,
    pub monitor_height: Option<u32>,
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub scale_factor: f64,
    pub coordinate_space: &'static str,
}

#[derive(Debug, Clone)]
pub struct MonitorGeometry {
    pub id: Option<String>,
    pub origin_x: i32,
    pub origin_y: i32,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub scale_factor: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PopupAnchor {
    pub x: i32,
    pub y: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TopStrutSpan {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EwmhTopStrut {
    pub top: u32,
    pub start_x: u32,
    pub end_x: u32,
}

impl EwmhTopStrut {
    pub fn basic_values(self) -> [u32; 4] {
        [0, 0, self.top, 0]
    }

    pub fn partial_values(self) -> [u32; 12] {
        [
            0,
            0,
            self.top,
            0,
            0,
            0,
            0,
            0,
            self.start_x,
            self.end_x,
            0,
            0,
        ]
    }
}

impl Default for MonitorGeometry {
    fn default() -> Self {
        Self {
            id: None,
            origin_x: 0,
            origin_y: 0,
            width: None,
            height: None,
            scale_factor: 1.0,
        }
    }
}

impl PanelGeometry {
    pub fn from_config(
        config: &PanelConfig,
        monitor: Option<MonitorGeometry>,
    ) -> Result<Self, String> {
        let monitor = monitor.unwrap_or_default();
        Ok(Self {
            monitor_id: monitor.id,
            monitor_origin_x: monitor.origin_x,
            monitor_origin_y: monitor.origin_y,
            monitor_width: monitor.width,
            monitor_height: monitor.height,
            x: config.bar.x,
            y: config.bar.y,
            width: config_dimension(config.bar.width, "bar.width")?,
            height: config_dimension(config.bar.height, "bar.height")?,
            scale_factor: monitor.scale_factor,
            coordinate_space: "physical",
        })
    }

    pub fn popup_anchor(&self, button_left: f64) -> PopupAnchor {
        PopupAnchor {
            x: self.x + button_left.floor() as i32,
            y: self.y + i32::try_from(self.height).unwrap_or(i32::MAX),
        }
    }

    pub fn top_strut_span(&self) -> TopStrutSpan {
        TopStrutSpan {
            x: self.x,
            y: self.y,
            width: self.width,
            height: self.height,
        }
    }

    pub fn ewmh_top_strut(&self) -> EwmhTopStrut {
        let top = i64::from(self.y)
            .max(0)
            .saturating_add(i64::from(self.height))
            .min(i64::from(u32::MAX)) as u32;
        let start_x = i64::from(self.x).max(0).min(i64::from(u32::MAX)) as u32;
        let raw_end_x = i64::from(self.x)
            .saturating_add(i64::from(self.width))
            .saturating_sub(1);
        let end_x = raw_end_x
            .max(i64::from(start_x))
            .min(i64::from(u32::MAX)) as u32;

        EwmhTopStrut {
            top,
            start_x,
            end_x,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn geometry_uses_normalized_config_dimensions() {
        let mut config = PanelConfig::default();
        config.bar.x = 76;
        config.bar.y = 0;
        config.bar.width = 658;
        config.bar.height = 20;

        let geometry = PanelGeometry::from_config(&config, None).expect("geometry should build");

        assert_eq!(geometry.x, 76);
        assert_eq!(geometry.y, 0);
        assert_eq!(geometry.width, 658);
        assert_eq!(geometry.height, 20);
        assert_eq!(geometry.monitor_origin_x, 0);
        assert_eq!(geometry.monitor_origin_y, 0);
        assert_eq!(geometry.scale_factor, 1.0);
        assert_eq!(geometry.coordinate_space, "physical");
    }

    #[test]
    fn geometry_carries_monitor_context_when_available() {
        let config = PanelConfig::default();
        let geometry = PanelGeometry::from_config(
            &config,
            Some(MonitorGeometry {
                id: Some("HDMI-1".to_string()),
                origin_x: 10,
                origin_y: 20,
                width: Some(1024),
                height: Some(600),
                scale_factor: 1.25,
            }),
        )
        .expect("geometry should build");

        assert_eq!(geometry.monitor_id.as_deref(), Some("HDMI-1"));
        assert_eq!(geometry.monitor_origin_x, 10);
        assert_eq!(geometry.monitor_origin_y, 20);
        assert_eq!(geometry.monitor_width, Some(1024));
        assert_eq!(geometry.monitor_height, Some(600));
        assert_eq!(geometry.scale_factor, 1.25);
    }

    #[test]
    fn popup_anchor_uses_effective_panel_origin_and_height() {
        let geometry = test_geometry();

        assert_eq!(geometry.popup_anchor(35.8), PopupAnchor { x: 111, y: 20 });
    }

    #[test]
    fn top_strut_span_matches_effective_panel_span() {
        let geometry = test_geometry();

        assert_eq!(
            geometry.top_strut_span(),
            TopStrutSpan {
                x: 76,
                y: 0,
                width: 658,
                height: 20
            }
        );
    }

    #[test]
    fn ewmh_top_strut_reserves_panel_depth_and_horizontal_span() {
        let geometry = test_geometry();
        let strut = geometry.ewmh_top_strut();

        assert_eq!(
            strut,
            EwmhTopStrut {
                top: 20,
                start_x: 76,
                end_x: 733
            }
        );
        assert_eq!(strut.basic_values(), [0, 0, 20, 0]);
        assert_eq!(
            strut.partial_values(),
            [0, 0, 20, 0, 0, 0, 0, 0, 76, 733, 0, 0]
        );
    }

    #[test]
    fn ewmh_top_strut_includes_positive_top_offset() {
        let mut geometry = test_geometry();
        geometry.y = 4;

        assert_eq!(geometry.ewmh_top_strut().top, 24);
    }

    fn test_geometry() -> PanelGeometry {
        PanelGeometry {
            monitor_id: Some("panel".to_string()),
            monitor_origin_x: 0,
            monitor_origin_y: 0,
            monitor_width: Some(1024),
            monitor_height: Some(600),
            x: 76,
            y: 0,
            width: 658,
            height: 20,
            scale_factor: 1.0,
            coordinate_space: "physical",
        }
    }
}
