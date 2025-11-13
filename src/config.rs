use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Config {
    pub display_width: u32,
    pub display_height: u32,
    pub panel_height: u32,
    pub eve_width: u32,
    pub eve_height: u32,
    pub overlay_x: f32,
    pub overlay_y: f32,
}

impl Config {
    fn config_path() -> PathBuf {
        let mut path = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
        path.push("nicotine");
        path.push("config.toml");
        path
    }

    fn detect_display_size() -> (u32, u32) {
        // Try to detect display size using xrandr
        if let Ok(output) = std::process::Command::new("xrandr")
            .args(["--current"])
            .output()
        {
            if let Ok(stdout) = String::from_utf8(output.stdout) {
                for line in stdout.lines() {
                    if line.contains("*") && line.contains("x") {
                        // Parse line like: "7680x2160     60.00*+"
                        if let Some(resolution) = line.split_whitespace().next() {
                            if let Some((w, h)) = resolution.split_once('x') {
                                if let (Ok(width), Ok(height)) = (w.parse(), h.parse()) {
                                    return (width, height);
                                }
                            }
                        }
                    }
                }
            }
        }

        // Fallback to common resolution
        (1920, 1080)
    }

    pub fn load() -> Result<Self> {
        let config_path = Self::config_path();

        // Try to load existing config
        if let Ok(contents) = fs::read_to_string(&config_path) {
            return toml::from_str(&contents).context("Failed to parse config.toml");
        }

        // Auto-generate config based on detected display
        println!("Generating config based on your display...");
        let (display_width, display_height) = Self::detect_display_size();
        println!("Detected display: {}x{}", display_width, display_height);

        let config = Self {
            display_width,
            display_height,
            panel_height: 0, // Assume no panel by default
            eve_width: (display_width as f32 * 0.54) as u32, // ~54% of width
            eve_height: display_height,
            overlay_x: 10.0,
            overlay_y: 10.0,
        };

        // Save the generated config
        if let Some(parent) = config_path.parent() {
            fs::create_dir_all(parent)?;
        }
        let contents = toml::to_string_pretty(&config)?;
        fs::write(&config_path, contents)?;
        println!("Created config: {}", config_path.display());
        println!("Edit it to customize window sizes and positions");

        Ok(config)
    }

    pub fn save_default() -> Result<()> {
        let config_path = Self::config_path();
        let (display_width, display_height) = Self::detect_display_size();

        let config = Self {
            display_width,
            display_height,
            panel_height: 0,
            eve_width: (display_width as f32 * 0.54) as u32,
            eve_height: display_height,
            overlay_x: 10.0,
            overlay_y: 10.0,
        };

        if let Some(parent) = config_path.parent() {
            fs::create_dir_all(parent)?;
        }
        let contents = toml::to_string_pretty(&config)?;
        fs::write(&config_path, contents)?;
        println!("Created config: {}", config_path.display());
        Ok(())
    }

    pub fn eve_x(&self) -> i32 {
        ((self.display_width - self.eve_width) / 2) as i32
    }

    pub fn eve_y(&self) -> i32 {
        0
    }

    pub fn eve_height_adjusted(&self) -> u32 {
        self.display_height - self.panel_height
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_eve_x_centers_window() {
        let config = Config {
            display_width: 1920,
            display_height: 1080,
            panel_height: 0,
            eve_width: 1000,
            eve_height: 1080,
            overlay_x: 10.0,
            overlay_y: 10.0,
        };

        // Window should be centered: (1920 - 1000) / 2 = 460
        assert_eq!(config.eve_x(), 460);
    }

    #[test]
    fn test_eve_y_is_zero() {
        let config = Config {
            display_width: 1920,
            display_height: 1080,
            panel_height: 0,
            eve_width: 1000,
            eve_height: 1080,
            overlay_x: 10.0,
            overlay_y: 10.0,
        };

        assert_eq!(config.eve_y(), 0);
    }

    #[test]
    fn test_eve_height_adjusted_with_panel() {
        let config = Config {
            display_width: 1920,
            display_height: 1080,
            panel_height: 40,
            eve_width: 1000,
            eve_height: 1080,
            overlay_x: 10.0,
            overlay_y: 10.0,
        };

        // Height should be: 1080 - 40 = 1040
        assert_eq!(config.eve_height_adjusted(), 1040);
    }

    #[test]
    fn test_eve_height_adjusted_without_panel() {
        let config = Config {
            display_width: 1920,
            display_height: 1080,
            panel_height: 0,
            eve_width: 1000,
            eve_height: 1080,
            overlay_x: 10.0,
            overlay_y: 10.0,
        };

        assert_eq!(config.eve_height_adjusted(), 1080);
    }

    #[test]
    fn test_config_serialization() {
        let config = Config {
            display_width: 7680,
            display_height: 2160,
            panel_height: 0,
            eve_width: 4147,
            eve_height: 2160,
            overlay_x: 10.0,
            overlay_y: 10.0,
        };

        let toml_str = toml::to_string(&config).unwrap();
        let deserialized: Config = toml::from_str(&toml_str).unwrap();

        assert_eq!(deserialized.display_width, 7680);
        assert_eq!(deserialized.display_height, 2160);
        assert_eq!(deserialized.eve_width, 4147);
    }
}
