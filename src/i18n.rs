use std::collections::BTreeMap;

const EN_US: &str = include_str!("../assets/lang/en_us.json");
const ZH_CN: &str = include_str!("../assets/lang/zh_cn.json");

#[derive(Default)]
pub struct Localizer {
    en_us: BTreeMap<String, String>,
    zh_cn: BTreeMap<String, String>,
}

impl Localizer {
    pub fn load() -> Result<Self, String> {
        let en_us = serde_json::from_str(EN_US)
            .map_err(|error| format!("内置 en_us 语言文件无效：{error}"))?;
        let zh_cn = serde_json::from_str(ZH_CN)
            .map_err(|error| format!("内置 zh_cn 语言文件无效：{error}"))?;
        Ok(Self { en_us, zh_cn })
    }

    pub fn message(
        &self,
        player_locale: &str,
        fallback_locale: &str,
        key: &str,
        replacements: &[(&str, &str)],
    ) -> String {
        let catalog = self.catalog_for(player_locale, fallback_locale);
        let fallback = self.en_us.get(key);
        let mut message = catalog
            .get(key)
            .or(fallback)
            .cloned()
            .unwrap_or_else(|| key.to_string());

        for (name, value) in replacements {
            message = message.replace(&format!("{{{name}}}"), value);
        }
        message
    }

    fn catalog_for(&self, player_locale: &str, fallback_locale: &str) -> &BTreeMap<String, String> {
        match language_family(player_locale).or_else(|| language_family(fallback_locale)) {
            Some(LanguageFamily::Chinese) => &self.zh_cn,
            Some(LanguageFamily::English) | None => &self.en_us,
        }
    }
}

#[derive(Clone, Copy)]
enum LanguageFamily {
    Chinese,
    English,
}

fn language_family(locale: &str) -> Option<LanguageFamily> {
    let normalized = locale.trim().to_ascii_lowercase().replace('-', "_");
    if normalized == "zh" || normalized.starts_with("zh_") {
        Some(LanguageFamily::Chinese)
    } else if normalized == "en" || normalized.starts_with("en_") {
        Some(LanguageFamily::English)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::Localizer;

    #[test]
    fn chooses_chinese_from_minecraft_locale() {
        let localizer = Localizer::load();
        assert!(localizer.is_ok());
        if let Ok(localizer) = localizer {
            assert_eq!(
                localizer.message("zh_cn", "en_us", "command.enabled", &[]),
                "TPSBar 已开启。"
            );
        }
    }

    #[test]
    fn falls_back_and_replaces_values() {
        let localizer = Localizer::load();
        assert!(localizer.is_ok());
        if let Ok(localizer) = localizer {
            assert_eq!(
                localizer.message(
                    "unknown",
                    "en_us",
                    "error.state_save",
                    &[("error", "disk full")],
                ),
                "Unable to save TPSBar state: disk full"
            );
        }
    }
}
