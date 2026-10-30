use std::{
    collections::{BTreeMap, BTreeSet},
    fs::{self, OpenOptions},
    io::{ErrorKind, Write},
    path::{Path, PathBuf},
};

use crate::{config::BarMetric, i18n::diagnostic};
use serde::{Deserialize, Serialize};

const STATE_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct PlayerKey {
    pub high: u64,
    pub low: u64,
}

impl PlayerKey {
    fn storage_key(self) -> String {
        format!("{:016x}{:016x}", self.high, self.low)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PersistentState {
    schema_version: u32,
    enabled_players: BTreeSet<PlayerKey>,
    #[serde(default)]
    metric_overrides: BTreeMap<String, BarMetric>,
}

impl Default for PersistentState {
    fn default() -> Self {
        Self {
            schema_version: STATE_SCHEMA_VERSION,
            enabled_players: BTreeSet::new(),
            metric_overrides: BTreeMap::new(),
        }
    }
}

impl PersistentState {
    pub fn is_enabled(&self, player: PlayerKey) -> bool {
        self.enabled_players.contains(&player)
    }
    pub fn metric(&self, player: PlayerKey, default: BarMetric) -> BarMetric {
        self.metric_overrides
            .get(&player.storage_key())
            .copied()
            .unwrap_or(default)
    }
    pub fn set_metric(&mut self, player: PlayerKey, metric: BarMetric) -> Option<BarMetric> {
        self.metric_overrides.insert(player.storage_key(), metric)
    }
    pub fn restore_metric(&mut self, player: PlayerKey, previous: Option<BarMetric>) {
        if let Some(metric) = previous {
            self.set_metric(player, metric);
        } else {
            self.metric_overrides.remove(&player.storage_key());
        }
    }
    pub fn toggle(&mut self, player: PlayerKey) -> bool {
        if self.enabled_players.remove(&player) {
            false
        } else {
            self.enabled_players.insert(player);
            true
        }
    }
}

pub struct StateLoadResult {
    pub state: PersistentState,
    pub warning: Option<String>,
}

#[derive(Debug, Clone)]
pub struct StateStore {
    path: PathBuf,
    previous_path: PathBuf,
    backup_path: PathBuf,
    locale: String,
}

impl StateStore {
    pub fn new(data_folder: &Path) -> Self {
        Self {
            path: data_folder.join("state.json"),
            previous_path: data_folder.join("state.json.previous"),
            backup_path: data_folder.join("state.json.bak"),
            locale: "zh_cn".to_string(),
        }
    }
    pub fn with_locale(mut self, locale: &str) -> Self {
        self.locale = locale.to_string();
        self
    }
    pub fn load(&self) -> StateLoadResult {
        let mut errors = Vec::new();
        for path in [&self.path, &self.previous_path, &self.backup_path] {
            match read_state(path, &self.locale) {
                Ok(Some(state)) => {
                    let warning = (path != &self.path).then(|| {
                        diagnostic(
                            &self.locale,
                            "state.recovered",
                            &[("path", &path.display().to_string())],
                        )
                    });
                    return StateLoadResult { state, warning };
                }
                Ok(None) => {}
                Err(error) => errors.push(error),
            }
        }
        StateLoadResult {
            state: PersistentState::default(),
            warning: (!errors.is_empty()).then(|| {
                diagnostic(
                    &self.locale,
                    "state.unavailable",
                    &[("error", &errors.join("; "))],
                )
            }),
        }
    }
    pub fn save(&self, state: &PersistentState) -> Result<(), String> {
        let parent = self
            .path
            .parent()
            .ok_or_else(|| diagnostic(&self.locale, "state.parent", &[]))?;
        fs::create_dir_all(parent).map_err(|e| file_error(&self.locale, parent, &e))?;
        let bytes = serde_json::to_vec_pretty(state).map_err(|e| {
            diagnostic(
                &self.locale,
                "state.serialize",
                &[("error", &e.to_string())],
            )
        })?;
        let temporary_path = self.path.with_extension("json.new");
        write_synced(&temporary_path, &bytes, &self.locale)?;

        // 主文件损坏或丢失时，先保护上次事务中仍然有效的恢复文件。
        if read_state(&self.previous_path, &self.locale).is_ok_and(|value| value.is_some()) {
            self.update_backup()?;
        }
        remove_if_exists(&self.previous_path, &self.locale)?;
        if self.path.exists() {
            fs::rename(&self.path, &self.previous_path)
                .map_err(|e| file_error(&self.locale, &self.path, &e))?;
        }
        if let Err(error) = fs::rename(&temporary_path, &self.path) {
            if self.previous_path.exists() {
                let _ = fs::rename(&self.previous_path, &self.path);
            }
            return Err(file_error(&self.locale, &self.path, &error));
        }
        // 损坏的旧主文件不可覆盖最后一份有效备份。
        if read_state(&self.previous_path, &self.locale).is_ok_and(|value| value.is_some())
            && let Err(error) = self.update_backup()
        {
            tracing::warn!(%error, "{}", diagnostic(&self.locale, "state.backup", &[]));
            return Ok(());
        }
        if let Err(error) = remove_if_exists(&self.previous_path, &self.locale) {
            tracing::warn!(%error, "{}", diagnostic(&self.locale, "state.cleanup", &[]));
        }
        Ok(())
    }

    fn update_backup(&self) -> Result<(), String> {
        let next = self.backup_path.with_extension("bak.new");
        let previous = self.backup_path.with_extension("bak.previous");
        let bytes = fs::read(&self.previous_path)
            .map_err(|error| file_error(&self.locale, &self.previous_path, &error))?;
        write_synced(&next, &bytes, &self.locale)?;
        remove_if_exists(&previous, &self.locale)?;
        let had_backup = self.backup_path.exists();
        if had_backup {
            fs::rename(&self.backup_path, &previous)
                .map_err(|error| file_error(&self.locale, &self.backup_path, &error))?;
        }
        if let Err(error) = fs::rename(&next, &self.backup_path) {
            if had_backup {
                let _ = fs::rename(&previous, &self.backup_path);
            }
            return Err(file_error(&self.locale, &self.backup_path, &error));
        }
        if let Err(error) = remove_if_exists(&previous, &self.locale) {
            tracing::warn!(%error, "{}", diagnostic(&self.locale, "state.cleanup", &[]));
        }
        Ok(())
    }
}

fn read_state(path: &Path, locale: &str) -> Result<Option<PersistentState>, String> {
    let contents = match fs::read_to_string(path) {
        Ok(contents) => contents,
        Err(error) if error.kind() == ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(file_error(locale, path, &error)),
    };
    let state: PersistentState = serde_json::from_str(&contents).map_err(|e| {
        diagnostic(
            locale,
            "state.parse",
            &[
                ("path", &path.display().to_string()),
                ("error", &e.to_string()),
            ],
        )
    })?;
    if state.schema_version != STATE_SCHEMA_VERSION {
        return Err(diagnostic(
            locale,
            "state.schema",
            &[
                ("path", &path.display().to_string()),
                ("actual", &state.schema_version.to_string()),
                ("expected", &STATE_SCHEMA_VERSION.to_string()),
            ],
        ));
    }
    Ok(Some(state))
}

fn file_error(locale: &str, path: &Path, error: &std::io::Error) -> String {
    diagnostic(
        locale,
        "io",
        &[
            ("path", &path.display().to_string()),
            ("error", &error.to_string()),
        ],
    )
}
fn write_synced(path: &Path, bytes: &[u8], locale: &str) -> Result<(), String> {
    let mut file = OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(path)
        .map_err(|e| file_error(locale, path, &e))?;
    file.write_all(bytes)
        .and_then(|()| file.sync_all())
        .map_err(|e| file_error(locale, path, &e))
}
fn remove_if_exists(path: &Path, locale: &str) -> Result<(), String> {
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(()),
        Err(error) => Err(file_error(locale, path, &error)),
    }
}

#[cfg(test)]
mod tests {
    use super::{PersistentState, PlayerKey, StateStore};
    use crate::config::BarMetric;
    use std::{fs, path::PathBuf, process, time::SystemTime};

    fn temporary_directory() -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .map_or(0, |duration| duration.as_nanos());
        std::env::temp_dir().join(format!("tpsbar-test-{}-{nanos}", process::id()))
    }
    #[test]
    fn preferences_are_personal_and_survive_restart() -> Result<(), String> {
        let directory = temporary_directory();
        let store = StateStore::new(&directory);
        let a = PlayerKey { high: 1, low: 2 };
        let b = PlayerKey { high: 1, low: 3 };
        let mut state = PersistentState::default();
        assert!(!state.is_enabled(a));
        assert!(state.toggle(a));
        state.set_metric(a, BarMetric::Ping);
        state.set_metric(b, BarMetric::Tps);
        store.save(&state)?;
        let loaded = store.load();
        assert_eq!(loaded.state, state);
        assert!(!loaded.state.is_enabled(b));
        assert_eq!(loaded.state.metric(a, BarMetric::Mspt), BarMetric::Ping);
        assert_eq!(loaded.state.metric(b, BarMetric::Mspt), BarMetric::Tps);
        let _ = fs::remove_dir_all(directory);
        Ok(())
    }
    #[test]
    fn version_02_state_keeps_toggles_and_uses_default_metric() -> Result<(), String> {
        let state: PersistentState =
            serde_json::from_str(r#"{"schema_version":1,"enabled_players":[{"high":1,"low":2}]}"#)
                .map_err(|e| e.to_string())?;
        let player = PlayerKey { high: 1, low: 2 };
        assert!(state.is_enabled(player));
        assert_eq!(state.metric(player, BarMetric::Tps), BarMetric::Tps);
        Ok(())
    }
    #[test]
    fn rolling_back_an_unset_metric_restores_default_behavior() {
        let mut state = PersistentState::default();
        let player = PlayerKey { high: 1, low: 2 };
        let previous = state.set_metric(player, BarMetric::Ping);
        state.restore_metric(player, previous);
        assert_eq!(state.metric(player, BarMetric::Tps), BarMetric::Tps);
    }
    #[test]
    fn corrupt_primary_does_not_replace_good_backup() -> Result<(), String> {
        let directory = temporary_directory();
        let store = StateStore::new(&directory);
        let mut state = PersistentState::default();
        state.set_metric(PlayerKey { high: 1, low: 2 }, BarMetric::Ping);
        store.save(&state)?;
        fs::copy(&store.path, &store.backup_path).map_err(|e| e.to_string())?;
        fs::write(&store.path, "broken").map_err(|e| e.to_string())?;
        assert_eq!(store.load().state, state);
        store.save(&state)?;
        assert_eq!(
            fs::read(&store.path).ok(),
            fs::read(&store.backup_path).ok()
        );
        let _ = fs::remove_dir_all(directory);
        Ok(())
    }
    #[test]
    fn failed_backup_write_preserves_existing_backup_and_recovery() -> Result<(), String> {
        let directory = temporary_directory();
        let store = StateStore::new(&directory);
        let state = PersistentState::default();
        store.save(&state)?;
        fs::copy(&store.path, &store.backup_path).map_err(|error| error.to_string())?;
        fs::rename(&store.path, &store.previous_path).map_err(|error| error.to_string())?;
        let old_backup = fs::read(&store.backup_path).map_err(|error| error.to_string())?;
        fs::create_dir(store.backup_path.with_extension("bak.new"))
            .map_err(|error| error.to_string())?;
        assert!(store.save(&state).is_err());
        assert_eq!(fs::read(&store.backup_path).ok(), Some(old_backup));
        assert_eq!(store.load().state, state);
        let _ = fs::remove_dir_all(directory);
        Ok(())
    }

    #[test]
    fn recovery_file_is_protected_before_a_new_save() -> Result<(), String> {
        let directory = temporary_directory();
        let store = StateStore::new(&directory);
        let state = PersistentState::default();
        store.save(&state)?;
        fs::rename(&store.path, &store.previous_path).map_err(|e| e.to_string())?;
        assert!(store.load().warning.is_some());
        store.save(&state)?;
        assert!(store.backup_path.exists());
        let _ = fs::remove_dir_all(directory);
        Ok(())
    }
}
