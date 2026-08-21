use crate::config::{BarMetric, Config, MsptThresholds, PingThresholds, TpsThresholds};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Green,
    Yellow,
    Gold,
    Red,
}

pub fn displayed_tps(raw_tps: f64, target_tps: f64) -> f64 {
    if !target_tps.is_finite() || target_tps <= 0.0 || raw_tps.is_nan() || raw_tps <= 0.0 {
        return 0.0;
    }
    if raw_tps.is_infinite() {
        return target_tps;
    }
    raw_tps.clamp(0.0, target_tps)
}

pub fn bar_progress(metric: BarMetric, tps: f64, mspt: f64, ping: u32, config: &Config) -> f32 {
    let (value, full) = match metric {
        BarMetric::Mspt => (mspt, config.bar.mspt_full),
        BarMetric::Tps => (tps, config.target_tps),
        BarMetric::Ping => (f64::from(ping), config.bar.ping_full),
    };
    if !value.is_finite() || !full.is_finite() || full <= 0.0 {
        return 0.0;
    }
    let clamped = (value / full).clamp(0.0, 1.0);
    // BossBar 协议只接受 f32；值已限制在 [0, 1]，转换不会产生溢出。
    #[allow(clippy::cast_possible_truncation)]
    let progress = clamped as f32;
    progress
}

pub fn mspt_severity(mspt: f64, thresholds: MsptThresholds) -> Severity {
    if !mspt.is_finite() || mspt < 0.0 {
        return Severity::Red;
    }
    if mspt < thresholds.green {
        Severity::Green
    } else if mspt < thresholds.yellow {
        Severity::Yellow
    } else if mspt < thresholds.gold {
        Severity::Gold
    } else {
        Severity::Red
    }
}

pub fn tps_severity(tps: f64, thresholds: TpsThresholds) -> Severity {
    if !tps.is_finite() || tps < 0.0 {
        return Severity::Red;
    }
    if tps < thresholds.red {
        Severity::Red
    } else if tps < thresholds.gold {
        Severity::Gold
    } else if tps < thresholds.yellow {
        Severity::Yellow
    } else {
        Severity::Green
    }
}

pub fn ping_severity(ping: u32, thresholds: PingThresholds) -> Severity {
    if ping <= thresholds.green {
        Severity::Green
    } else if ping <= thresholds.yellow {
        Severity::Yellow
    } else if ping <= thresholds.gold {
        Severity::Gold
    } else {
        Severity::Red
    }
}

#[cfg(test)]
mod tests {
    use super::{
        Severity, bar_progress, displayed_tps, mspt_severity, ping_severity, tps_severity,
    };
    use crate::config::{BarMetric, Config, MsptThresholds, PingThresholds, TpsThresholds};

    #[test]
    fn tps_is_capped_at_configured_target() {
        assert!((displayed_tps(52.0, 20.0) - 20.0).abs() < f64::EPSILON);
        assert!((displayed_tps(12.5, 20.0) - 12.5).abs() < f64::EPSILON);
        assert!((displayed_tps(f64::INFINITY, 20.0) - 20.0).abs() < f64::EPSILON);
        assert!(displayed_tps(f64::NAN, 20.0).abs() < f64::EPSILON);
    }

    #[test]
    fn progress_uses_selected_metric_and_is_bounded() {
        let config = Config::default();
        assert!((bar_progress(BarMetric::Mspt, 20.0, 25.0, 0, &config) - 0.5).abs() < f32::EPSILON);
        assert!((bar_progress(BarMetric::Mspt, 20.0, 50.0, 0, &config) - 1.0).abs() < f32::EPSILON);
        assert!((bar_progress(BarMetric::Mspt, 20.0, 80.0, 0, &config) - 1.0).abs() < f32::EPSILON);
        assert!((bar_progress(BarMetric::Tps, 20.0, 0.0, 0, &config) - 1.0).abs() < f32::EPSILON);
        assert!((bar_progress(BarMetric::Tps, 10.0, 0.0, 0, &config) - 0.5).abs() < f32::EPSILON);
        assert!(
            (bar_progress(BarMetric::Ping, 20.0, 0.0, 100, &config) - 0.5).abs() < f32::EPSILON
        );
    }

    #[test]
    fn mspt_boundaries_match_configuration() {
        let thresholds = MsptThresholds::default();
        assert_eq!(mspt_severity(34.999, thresholds), Severity::Green);
        assert_eq!(mspt_severity(35.0, thresholds), Severity::Yellow);
        assert_eq!(mspt_severity(50.0, thresholds), Severity::Gold);
        assert_eq!(mspt_severity(80.0, thresholds), Severity::Red);
    }

    #[test]
    fn tps_boundaries_match_configuration() {
        let thresholds = TpsThresholds::default();
        assert_eq!(tps_severity(11.999, thresholds), Severity::Red);
        assert_eq!(tps_severity(12.0, thresholds), Severity::Gold);
        assert_eq!(tps_severity(17.0, thresholds), Severity::Yellow);
        assert_eq!(tps_severity(19.0, thresholds), Severity::Green);
    }

    #[test]
    fn ping_boundaries_match_configuration() {
        let thresholds = PingThresholds::default();
        assert_eq!(ping_severity(60, thresholds), Severity::Green);
        assert_eq!(ping_severity(61, thresholds), Severity::Yellow);
        assert_eq!(ping_severity(121, thresholds), Severity::Gold);
        assert_eq!(ping_severity(201, thresholds), Severity::Red);
    }
}
