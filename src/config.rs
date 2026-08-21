use std::{
    fs::{self, OpenOptions},
    io::{ErrorKind, Write},
    path::{Path, PathBuf},
};

use serde::Deserialize;

pub const DEFAULT_CONFIG_TEMPLATE: &str = include_str!("../assets/config.default.toml");
const CONFIG_FILE_NAME: &str = "config.toml";
const CONFIG_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BarMetric {
    #[default]
    Mspt,
    Tps,
    Ping,
}

impl BarMetric {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Mspt => "MSPT",
            Self::Tps => "TPS",
            Self::Ping => "Ping",
        }
    }
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
    fs::create_dir_all(data_folder)
        .map_err(|error| format!("无法创建插件数据目录 {}：{error}", data_folder.display()))?;

    let path = data_folder.join(CONFIG_FILE_NAME);
    create_default_file_if_missing(&path)?;

    let raw = fs::read_to_string(&path)
        .map_err(|error| format!("无法读取配置文件 {}：{error}", path.display()))?;

    match toml::from_str::<Config>(&raw) {
        Ok(config) => match config.validate() {
            Ok(()) => Ok(ConfigLoadResult {
                config,
                warning: None,
                path,
            }),
            Err(error) => Ok(ConfigLoadResult {
                config: Config::default(),
                warning: Some(format!(
                    "配置文件 {} 未通过校验，保留原文件并使用内置默认值：{error}",
                    path.display()
                )),
                path,
            }),
        },
        Err(error) => Ok(ConfigLoadResult {
            config: Config::default(),
            warning: Some(format!(
                "配置文件 {} 无法解析，保留原文件并使用内置默认值：{error}",
                path.display()
            )),
            path,
        }),
    }
}

fn create_default_file_if_missing(path: &Path) -> Result<(), String> {
    let file = OpenOptions::new().write(true).create_new(true).open(path);
    let mut file = match file {
        Ok(file) => file,
        Err(error) if error.kind() == ErrorKind::AlreadyExists => return Ok(()),
        Err(error) => {
            return Err(format!("无法创建默认配置文件 {}：{error}", path.display()));
        }
    };

    file.write_all(DEFAULT_CONFIG_TEMPLATE.as_bytes())
        .and_then(|()| file.sync_all())
        .map_err(|error| format!("无法写入默认配置文件 {}：{error}", path.display()))
}

impl Config {
    fn validate(&self) -> Result<(), String> {
        if self.schema_version != CONFIG_SCHEMA_VERSION {
            return Err(format!(
                "不支持 schema_version={}，当前仅支持 {CONFIG_SCHEMA_VERSION}",
                self.schema_version
            ));
        }
        if !(5..=1_200).contains(&self.refresh_interval_ticks) {
            return Err("refresh_interval_ticks 必须在 5 到 1200 之间".to_string());
        }
        if !self.target_tps.is_finite() || !(1.0..=100.0).contains(&self.target_tps) {
            return Err("target_tps 必须是 1.0 到 100.0 之间的有限数值".to_string());
        }
        if !self.bar.mspt_full.is_finite() || self.bar.mspt_full <= 0.0 {
            return Err("bar.mspt_full 必须是正的有限数值".to_string());
        }
        if !self.bar.ping_full.is_finite() || self.bar.ping_full <= 0.0 {
            return Err("bar.ping_full 必须是正的有限数值".to_string());
        }
        if self.permission.default_op_level > 4 {
            return Err("permission.default_op_level 必须在 0 到 4 之间".to_string());
        }
        validate_f64_ascending(
            "thresholds.mspt",
            [
                self.thresholds.mspt.green,
                self.thresholds.mspt.yellow,
                self.thresholds.mspt.gold,
            ],
        )?;
        validate_f64_ascending(
            "thresholds.tps",
            [
                self.thresholds.tps.red,
                self.thresholds.tps.gold,
                self.thresholds.tps.yellow,
            ],
        )?;

        let ping = self.thresholds.ping;
        if !(ping.green < ping.yellow && ping.yellow < ping.gold) {
            return Err("thresholds.ping 的三个阈值必须严格递增".to_string());
        }

        Ok(())
    }
}

fn validate_f64_ascending(name: &str, values: [f64; 3]) -> Result<(), String> {
    if values
        .iter()
        .any(|value| !value.is_finite() || *value < 0.0)
    {
        return Err(format!("{name} 的阈值必须是非负有限数值"));
    }
    if !(values[0] < values[1] && values[1] < values[2]) {
        return Err(format!("{name} 的三个阈值必须严格递增"));
    }
    Ok(())
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
