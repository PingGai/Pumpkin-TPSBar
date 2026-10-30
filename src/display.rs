use pumpkin_plugin_api::{
    boss_bar::BossBarColor,
    text::{NamedColor, TextComponent},
};

use crate::{
    config::{BarMetric, Config},
    i18n::Localizer,
    metrics::{Severity, bar_progress, displayed_tps, mspt_severity, ping_severity, tps_severity},
};

pub struct DisplaySnapshot {
    pub progress: f32,
    pub bar_severity: Severity,
    tps: f64,
    mspt: f64,
    ping: u32,
    tps_severity: Severity,
    mspt_severity: Severity,
    ping_severity: Severity,
}

impl DisplaySnapshot {
    pub fn new(raw_tps: f64, mspt: f64, ping: u32, config: &Config, metric: BarMetric) -> Self {
        let tps = displayed_tps(raw_tps, config.target_tps);
        let bar_severity = mspt_severity(mspt, config.thresholds.mspt);
        Self {
            tps,
            mspt,
            ping,
            progress: bar_progress(metric, tps, mspt, ping, config),
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

    pub fn title(&self, localizer: &Localizer, locale: &str, fallback: &str) -> TextComponent {
        let label = |key| localizer.message(locale, fallback, key, &[]);
        gray_text(&label("player.bar.tps"))
            .add_child(colored_text(&format!("{:.2}", self.tps), self.tps_severity))
            .add_child(gray_text(&label("player.bar.mspt")))
            .add_child(colored_text(
                &format!("{:.2}", self.mspt),
                self.mspt_severity,
            ))
            .add_child(gray_text(&label("player.bar.ms")))
            .add_child(gray_text(&label("player.bar.ping")))
            .add_child(colored_text(&self.ping.to_string(), self.ping_severity))
            .add_child(gray_text(&label("player.bar.ms")))
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
