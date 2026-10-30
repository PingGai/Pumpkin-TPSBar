use std::{
    fs::{self, OpenOptions},
    io::{ErrorKind, Write},
    path::{Path, PathBuf},
};

use crate::i18n::diagnostic;
use serde::Deserialize;

pub const DEFAULT_CONFIG_TEMPLATE: &str = include_str!("../assets/config.default.toml");
const CONFIG_FILE_NAME: &str = "config.toml";
const CONFIG_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum BarMetric {
    #[default]
    Mspt,
    Tps,
    Ping,
}

#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(default)]
pub struct BarConfig {
    pub metric: BarMetric,
    pub mspt_full: f64,
    pub ping_full: f64,
}

impl Default for BarConfig {
    fn default() -> Self {
        Self {
            metric: BarMetric::Mspt,
            mspt_full: 50.0,
            ping_full: 200.0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(default)]
pub struct Config {
    pub schema_version: u32,
    pub refresh_interval_ticks: u64,
    pub target_tps: f64,
    pub bar: BarConfig,
    pub fallback_locale: String,
    pub permission: PermissionConfig,
    pub thresholds: ThresholdConfig,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            schema_version: CONFIG_SCHEMA_VERSION,
            refresh_interval_ticks: 20,
            target_tps: 20.0,
            bar: BarConfig::default(),
            fallback_locale: "zh_cn".to_string(),
            permission: PermissionConfig::default(),
            thresholds: ThresholdConfig::default(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(default)]
pub struct PermissionConfig {
    pub default_op_level: u8,
}

impl Default for PermissionConfig {
    fn default() -> Self {
        Self {
            default_op_level: 3,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Deserialize, Default)]
#[serde(default)]
pub struct ThresholdConfig {
    pub mspt: MsptThresholds,
    pub tps: TpsThresholds,
    pub ping: PingThresholds,
}

#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(default)]
pub struct MsptThresholds {
    #[serde(rename = "green_below")]
    pub green: f64,
    #[serde(rename = "yellow_below")]
    pub yellow: f64,
    #[serde(rename = "gold_below")]
    pub gold: f64,
}

impl Default for MsptThresholds {
    fn default() -> Self {
        Self {
            green: 35.0,
            yellow: 50.0,
            gold: 80.0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(default)]
pub struct TpsThresholds {
    #[serde(rename = "red_below")]
    pub red: f64,
    #[serde(rename = "gold_below")]
    pub gold: f64,
    #[serde(rename = "yellow_below")]
    pub yellow: f64,
}

impl Default for TpsThresholds {
    fn default() -> Self {
        Self {
            red: 12.0,
            gold: 17.0,
            yellow: 19.0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(default)]
pub struct PingThresholds {
    #[serde(rename = "green_at_most")]
    pub green: u32,
    #[serde(rename = "yellow_at_most")]
    pub yellow: u32,
    #[serde(rename = "gold_at_most")]
    pub gold: u32,
}

impl Default for PingThresholds {
    fn default() -> Self {
        Self {
            green: 60,
            yellow: 120,
            gold: 200,
        }
    }
}

pub struct ConfigLoadResult {
    pub config: Config,
    pub warning: Option<String>,
    pub path: PathBuf,
}

pub fn load_or_create(data_folder: &Path) -> Result<ConfigLoadResult, String> {
    let fallback = Config::default().fallback_locale;
    fs::create_dir_all(data_folder)
        .map_err(|error| config_io_error(data_folder, &error, &fallback))?;
    let path = data_folder.join(CONFIG_FILE_NAME);
    create_default_file_if_missing(&path, &fallback)?;
    let raw =
        fs::read_to_string(&path).map_err(|error| config_io_error(&path, &error, &fallback))?;
    let result = toml::from_str::<Config>(&raw)
        .map_err(|error| error.to_string())
        .and_then(|config| {
            config.validate()?;
            Ok(config)
        });
    match result {
        Ok(config) => Ok(ConfigLoadResult {
            config,
            warning: None,
            path,
        }),
        Err(error) => Ok(ConfigLoadResult {
            config: Config::default(),
            warning: Some(diagnostic(
                &fallback,
                "config.fallback",
                &[("path", &path.display().to_string()), ("error", &error)],
            )),
            path,
        }),
    }
}

fn config_io_error(path: &Path, error: &std::io::Error, locale: &str) -> String {
    diagnostic(
        locale,
        "io",
        &[
            ("path", &path.display().to_string()),
            ("error", &error.to_string()),
        ],
    )
}

fn create_default_file_if_missing(path: &Path, locale: &str) -> Result<(), String> {
    let file = OpenOptions::new().write(true).create_new(true).open(path);
    let mut file = match file {
        Ok(file) => file,
        Err(error) if error.kind() == ErrorKind::AlreadyExists => return Ok(()),
        Err(error) => return Err(config_io_error(path, &error, locale)),
    };
    file.write_all(DEFAULT_CONFIG_TEMPLATE.as_bytes())
        .and_then(|()| file.sync_all())
        .map_err(|error| config_io_error(path, &error, locale))
}

impl Config {
    fn validate(&self) -> Result<(), String> {
        let error = |key| diagnostic(&self.fallback_locale, key, &[]);
        if self.schema_version != CONFIG_SCHEMA_VERSION {
            return Err(diagnostic(
                &self.fallback_locale,
                "config.schema",
                &[
                    ("actual", &self.schema_version.to_string()),
                    ("expected", &CONFIG_SCHEMA_VERSION.to_string()),
                ],
            ));
        }
        if !(5..=1_200).contains(&self.refresh_interval_ticks) {
            return Err(error("config.refresh"));
        }
        if !self.target_tps.is_finite() || !(1.0..=100.0).contains(&self.target_tps) {
            return Err(error("config.tps"));
        }
        for (name, value) in [
            ("bar.mspt_full", self.bar.mspt_full),
            ("bar.ping_full", self.bar.ping_full),
        ] {
            if !value.is_finite() || value <= 0.0 {
                return Err(diagnostic(
                    &self.fallback_locale,
                    "config.positive",
                    &[("name", name)],
                ));
            }
        }
        if self.permission.default_op_level > 4 {
            return Err(error("config.permission"));
        }
        let mspt = self.thresholds.mspt;
        let tps = self.thresholds.tps;
        let ping = self.thresholds.ping;
        for (name, values) in [
            ("thresholds.mspt", [mspt.green, mspt.yellow, mspt.gold]),
            ("thresholds.tps", [tps.red, tps.gold, tps.yellow]),
            (
                "thresholds.ping",
                [
                    f64::from(ping.green),
                    f64::from(ping.yellow),
                    f64::from(ping.gold),
                ],
            ),
        ] {
            if values
                .iter()
                .any(|value| !value.is_finite() || *value < 0.0)
                || !(values[0] < values[1] && values[1] < values[2])
            {
                return Err(diagnostic(
                    &self.fallback_locale,
                    "config.thresholds",
                    &[("name", name)],
                ));
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::{Config, DEFAULT_CONFIG_TEMPLATE};

    #[test]
    fn bundled_template_matches_defaults() {
        let parsed = toml::from_str::<Config>(DEFAULT_CONFIG_TEMPLATE);
        assert_eq!(parsed, Ok(Config::default()));
    }

    #[test]
    fn invalid_threshold_order_is_rejected() {
        let mut config = Config::default();
        config.thresholds.mspt.yellow = config.thresholds.mspt.green;
        assert!(config.validate().is_err());
    }
}
