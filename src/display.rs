use pumpkin_plugin_api::{
    boss_bar::BossBarColor,
    text::{NamedColor, TextComponent},
};

use crate::{
    config::Config,
    metrics::{Severity, bar_progress, displayed_tps, mspt_severity, ping_severity, tps_severity},
};

pub struct DisplaySnapshot {
    pub tps: f64,
    pub mspt: f64,
    pub ping: u32,
    pub progress: f32,
    pub bar_severity: Severity,
    tps_severity: Severity,
    mspt_severity: Severity,
    ping_severity: Severity,
}

impl DisplaySnapshot {
    pub fn new(raw_tps: f64, mspt: f64, ping: u32, config: &Config) -> Self {
        let tps = displayed_tps(raw_tps, config.target_tps);
        let bar_severity = mspt_severity(mspt, config.thresholds.mspt);
        Self {
            tps,
            mspt,
            ping,
            progress: bar_progress(config.bar.metric, tps, mspt, ping, config),
            bar_severity,
            tps_severity: tps_severity(tps, config.thresholds.tps),
            mspt_severity: bar_severity,
            ping_severity: ping_severity(ping, config.thresholds.ping),
        }
    }

    pub fn bossbar_color(&self) -> BossBarColor {
        match self.bar_severity {
            Severity::Green => BossBarColor::Green,
            Severity::Yellow | Severity::Gold => BossBarColor::Yellow,
            Severity::Red => BossBarColor::Red,
        }
    }

    pub fn title(&self) -> TextComponent {
        gray_text("TPS: ")
            .add_child(colored_text(&format!("{:.2}", self.tps), self.tps_severity))
            .add_child(gray_text("  MSPT: "))
            .add_child(colored_text(
                &format!("{:.2}", self.mspt),
                self.mspt_severity,
            ))
            .add_child(gray_text(" ms"))
            .add_child(gray_text("  PING: "))
            .add_child(colored_text(&self.ping.to_string(), self.ping_severity))
            .add_child(gray_text(" ms"))
    }
}

fn gray_text(text: &str) -> TextComponent {
    TextComponent::text(text).color_named(NamedColor::Gray)
}

fn colored_text(text: &str, severity: Severity) -> TextComponent {
    TextComponent::text(text).color_named(match severity {
        Severity::Green => NamedColor::Green,
        Severity::Yellow => NamedColor::Yellow,
        Severity::Gold => NamedColor::Gold,
        Severity::Red => NamedColor::Red,
    })
}
