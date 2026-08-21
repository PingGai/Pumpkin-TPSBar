use std::{
    collections::BTreeSet,
    fs::{self, OpenOptions},
    io::{ErrorKind, Write},
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};

const STATE_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct PlayerKey {
    pub high: u64,
    pub low: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PersistentState {
    schema_version: u32,
    enabled_players: BTreeSet<PlayerKey>,
}

impl Default for PersistentState {
    fn default() -> Self {
        Self {
            schema_version: STATE_SCHEMA_VERSION,
            enabled_players: BTreeSet::new(),
        }
    }
}

impl PersistentState {
    pub fn is_enabled(&self, player: PlayerKey) -> bool {
        self.enabled_players.contains(&player)
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
}

impl StateStore {
    pub fn new(data_folder: &Path) -> Self {
        Self {
            path: data_folder.join("state.json"),
            previous_path: data_folder.join("state.json.previous"),
            backup_path: data_folder.join("state.json.bak"),
        }
    }

    pub fn load(&self) -> StateLoadResult {
        let candidates = [
            (&self.path, "主状态文件"),
            (&self.previous_path, "事务恢复文件"),
            (&self.backup_path, "备份状态文件"),
        ];
        let mut errors = Vec::new();

        for (path, label) in candidates {
            match read_state(path) {
                Ok(Some(state)) => {
                    let warning = if path == &self.path {
                        None
                    } else {
                        Some(format!(
                            "{label} {} 已用于恢复玩家状态；下次切换时会写回主文件",
                            path.display()
                        ))
                    };
                    return StateLoadResult { state, warning };
                }
                Ok(None) => {}
                Err(error) => errors.push(error),
            }
        }

        let warning = if errors.is_empty() {
            None
        } else {
            Some(format!(
                "没有可用的玩家状态文件，将使用默认关闭状态：{}",
                errors.join("；")
            ))
        };
        StateLoadResult {
            state: PersistentState::default(),
            warning,
        }
    }

    pub fn save(&self, state: &PersistentState) -> Result<(), String> {
        let parent = self
            .path
            .parent()
            .ok_or_else(|| "状态文件缺少父目录".to_string())?;
        fs::create_dir_all(parent)
            .map_err(|error| format!("无法创建状态目录 {}：{error}", parent.display()))?;

        let mut serialized = serde_json::to_string_pretty(state)
            .map_err(|error| format!("无法序列化玩家状态：{error}"))?;
        serialized.push('\n');

        let temporary_path = self.path.with_extension("json.tmp");
        write_synced(&temporary_path, serialized.as_bytes())?;

        remove_if_exists(&self.previous_path)?;
        if self.path.exists() {
            fs::rename(&self.path, &self.previous_path).map_err(|error| {
                format!(
                    "无法把旧状态文件 {} 移入事务恢复位置：{error}",
                    self.path.display()
                )
            })?;
        }

        if let Err(error) = fs::rename(&temporary_path, &self.path) {
            if self.previous_path.exists() {
                let _ = fs::rename(&self.previous_path, &self.path);
            }
            return Err(format!(
                "无法提交新状态文件 {}：{error}",
                self.path.display()
            ));
        }

        if self.previous_path.exists() {
            if let Err(error) = fs::copy(&self.previous_path, &self.backup_path) {
                tracing::warn!(
                    path = %self.backup_path.display(),
                    %error,
                    "无法更新 TPSBar 状态备份，但主状态文件已经安全提交"
                );
            }
            if let Err(error) = fs::remove_file(&self.previous_path) {
                tracing::warn!(
                    path = %self.previous_path.display(),
                    %error,
                    "无法清理 TPSBar 事务恢复文件"
                );
            }
        }

        Ok(())
    }
}

fn read_state(path: &Path) -> Result<Option<PersistentState>, String> {
    let raw = match fs::read_to_string(path) {
        Ok(raw) => raw,
        Err(error) if error.kind() == ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(format!("无法读取 {}：{error}", path.display())),
    };
    let state = serde_json::from_str::<PersistentState>(&raw)
        .map_err(|error| format!("无法解析 {}：{error}", path.display()))?;
    if state.schema_version != STATE_SCHEMA_VERSION {
        return Err(format!(
            "{} 使用不支持的 schema_version={}（当前支持 {STATE_SCHEMA_VERSION}）",
            path.display(),
            state.schema_version
        ));
    }
    Ok(Some(state))
}

fn write_synced(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let mut file = OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(path)
        .map_err(|error| format!("无法创建临时状态文件 {}：{error}", path.display()))?;
    file.write_all(bytes)
        .and_then(|()| file.sync_all())
        .map_err(|error| format!("无法写入临时状态文件 {}：{error}", path.display()))
}

fn remove_if_exists(path: &Path) -> Result<(), String> {
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(()),
        Err(error) => Err(format!("无法清理旧事务文件 {}：{error}", path.display())),
    }
}

#[cfg(test)]
mod tests {
    use std::{fs, path::PathBuf, process, time::SystemTime};

    use super::{PersistentState, PlayerKey, StateStore};

    fn temporary_directory() -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .map_or(0, |duration| duration.as_nanos());
        std::env::temp_dir().join(format!("tpsbar-test-{}-{nanos}", process::id()))
    }

    #[test]
    fn state_round_trip_preserves_per_player_choice() {
        let directory = temporary_directory();
        let store = StateStore::new(&directory);
        let player = PlayerKey { high: 1, low: 2 };
        let mut state = PersistentState::default();
        assert!(state.toggle(player));
        assert!(store.save(&state).is_ok());

        let loaded = store.load();
        assert!(loaded.state.is_enabled(player));
        let _ = fs::remove_dir_all(directory);
    }

    #[test]
    fn a_new_player_is_disabled_by_default() {
        let state = PersistentState::default();
        assert!(!state.is_enabled(PlayerKey { high: 7, low: 9 }));
    }
}
