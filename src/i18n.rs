use std::{collections::BTreeMap, sync::LazyLock};

use serde::Deserialize;

const EN_US: &str = include_str!("../assets/lang/en_us.json");
const ZH_CN: &str = include_str!("../assets/lang/zh_cn.json");
static CATALOGS: LazyLock<Result<Localizer, String>> = LazyLock::new(Localizer::parse);

#[derive(Clone, Default, Deserialize)]
struct Catalog {
    player: BTreeMap<String, String>,
    diagnostic: BTreeMap<String, String>,
}

#[derive(Clone, Default)]
pub struct Localizer {
    en_us: Catalog,
    zh_cn: Catalog,
}

impl Localizer {
    pub fn load() -> Result<Self, String> {
        CATALOGS.as_ref().cloned().map_err(Clone::clone)
    }

    fn parse() -> Result<Self, String> {
        // 语言目录本身无效时只能返回稳定的诊断标识；正常启动由测试保证目录完整。
        let en_us = serde_json::from_str(EN_US)
            .map_err(|error| format!("diagnostic.locale_invalid: en_us: {error}"))?;
        let zh_cn = serde_json::from_str(ZH_CN)
            .map_err(|error| format!("diagnostic.locale_invalid: zh_cn: {error}"))?;
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
        let mut message = lookup(catalog, key)
            .or_else(|| lookup(&self.en_us, key))
            .cloned()
            .unwrap_or_else(|| key.to_string());

        // 只处理原模板里的占位符，替换内容中的花括号不会再次被解释。
        let template = message.clone();
        message.clear();
        let mut remaining = template.as_str();
        while let Some(start) = remaining.find('{') {
            message.push_str(&remaining[..start]);
            let Some(end) = remaining[start..].find('}').map(|end| start + end) else {
                message.push_str(&remaining[start..]);
                return message;
            };
            let name = &remaining[start + 1..end];
            if let Some((_, value)) = replacements
                .iter()
                .find(|(candidate, _)| *candidate == name)
            {
                message.push_str(value);
            } else {
                message.push_str(&remaining[start..=end]);
            }
            remaining = &remaining[end + 1..];
        }
        message.push_str(remaining);
        message
    }

    fn catalog_for(&self, player_locale: &str, fallback_locale: &str) -> &Catalog {
        match language_family(player_locale).or_else(|| language_family(fallback_locale)) {
            Some(LanguageFamily::Chinese) => &self.zh_cn,
            Some(LanguageFamily::English) | None => &self.en_us,
        }
    }
}

pub fn diagnostic(locale: &str, key: &str, replacements: &[(&str, &str)]) -> String {
    match CATALOGS.as_ref() {
        Ok(localizer) => {
            localizer.message(locale, "en_us", &format!("diagnostic.{key}"), replacements)
        }
        Err(error) => error.clone(),
    }
}

fn lookup<'a>(catalog: &'a Catalog, key: &str) -> Option<&'a String> {
    let (namespace, key) = key.split_once('.')?;
    match namespace {
        "player" => catalog.player.get(key),
        "diagnostic" => catalog.diagnostic.get(key),
        _ => None,
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
    use std::collections::BTreeSet;

    use super::{EN_US, Localizer, ZH_CN};

    #[test]
    fn locale_selection_and_fallback_work() -> Result<(), String> {
        let localizer = Localizer::load()?;
        assert_eq!(
            localizer.message("zh-CN", "en_us", "player.command.enabled", &[]),
            "TPSBar 已开启。"
        );
        assert_eq!(
            localizer.message("unknown", "en_us", "player.command.enabled", &[]),
            "TPSBar enabled."
        );
        assert_eq!(
            localizer.message("unknown", "zh_cn", "player.command.enabled", &[]),
            "TPSBar 已开启。"
        );
        assert_eq!(
            localizer.message("en_us", "zh_cn", "player.command.enabled", &[]),
            "TPSBar enabled."
        );
        Ok(())
    }

    #[test]
    fn catalogs_have_matching_keys_and_placeholders() -> Result<(), String> {
        let en: serde_json::Value = serde_json::from_str(EN_US).map_err(|e| e.to_string())?;
        let zh: serde_json::Value = serde_json::from_str(ZH_CN).map_err(|e| e.to_string())?;
        for namespace in ["player", "diagnostic"] {
            let en = en[namespace].as_object().ok_or("missing en namespace")?;
            let zh = zh[namespace].as_object().ok_or("missing zh namespace")?;
            assert_eq!(en.keys().collect::<Vec<_>>(), zh.keys().collect::<Vec<_>>());
            for key in en.keys() {
                let placeholders = |text: &str| -> BTreeSet<String> {
                    text.split('{')
                        .skip(1)
                        .filter_map(|part| part.split_once('}').map(|(name, _)| name.to_string()))
                        .collect()
                };
                assert_eq!(
                    placeholders(en[key].as_str().ok_or("not text")?),
                    placeholders(zh[key].as_str().ok_or("not text")?),
                    "{namespace}.{key}"
                );
            }
        }
        Ok(())
    }

    #[test]
    fn replacement_values_are_not_interpreted_again() -> Result<(), String> {
        let localizer = Localizer::load()?;
        let message = localizer.message(
            "en_us",
            "en_us",
            "diagnostic.io",
            &[("path", "{error}"), ("error", "disk full")],
        );
        assert_eq!(message, "Unable to access {error}: disk full");
        Ok(())
    }
}
