use std::{
    collections::{HashMap, HashSet},
    mem,
    sync::{Arc, Mutex, MutexGuard},
};

use pumpkin_plugin_api::{
    Server,
    boss_bar::{BossBar, BossBarDivision},
    player::Player,
};

use crate::{
    config::Config,
    display::DisplaySnapshot,
    i18n::Localizer,
    metrics::Severity,
    state::{PersistentState, PlayerKey, StateStore},
};

pub const PERMISSION_NODE: &str = "tpsbar:command.toggle";

pub type SharedRuntime = Arc<Mutex<RuntimeState>>;

pub struct RuntimeState {
    pub config: Config,
    pub localizer: Localizer,
    pub persistent_state: PersistentState,
    pub state_store: StateStore,
    bars: HashMap<PlayerKey, PlayerBar>,
}

struct PlayerBar {
    bossbar: BossBar,
    severity: Severity,
}

impl Default for RuntimeState {
    fn default() -> Self {
        Self {
            config: Config::default(),
            localizer: Localizer::default(),
            persistent_state: PersistentState::default(),
            state_store: StateStore::new(std::path::Path::new("data")),
            bars: HashMap::new(),
        }
    }
}

impl RuntimeState {
    pub fn initialized(
        config: Config,
        localizer: Localizer,
        persistent_state: PersistentState,
        state_store: StateStore,
    ) -> Self {
        Self {
            config,
            localizer,
            persistent_state,
            state_store,
            bars: HashMap::new(),
        }
    }
}

pub fn new_shared_runtime() -> SharedRuntime {
    Arc::new(Mutex::new(RuntimeState::default()))
}

pub fn lock(runtime: &SharedRuntime) -> MutexGuard<'_, RuntimeState> {
    runtime
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

pub fn player_key(player: &Player) -> PlayerKey {
    let id = player.get_id();
    PlayerKey {
        high: id.high,
        low: id.low,
    }
}

pub fn refresh_all(server: &Server, runtime: &SharedRuntime) {
    let config = lock(runtime).config.clone();
    let raw_tps = server.get_tps();
    let mspt = server.get_mspt();
    let mut active_players = HashSet::new();

    for player in server.get_all_players() {
        if let Some(key) = refresh_player_with_sample(player, runtime, &config, raw_tps, mspt) {
            active_players.insert(key);
        }
    }

    let removed = {
        let mut state = lock(runtime);
        let stale_keys = state
            .bars
            .keys()
            .filter(|key| !active_players.contains(key))
            .copied()
            .collect::<Vec<_>>();
        stale_keys
            .into_iter()
            .filter_map(|key| state.bars.remove(&key))
            .collect::<Vec<_>>()
    };
    drop(removed);
}

pub fn refresh_player(server: &Server, player: Player, runtime: &SharedRuntime) {
    let config = lock(runtime).config.clone();
    let raw_tps = server.get_tps();
    let mspt = server.get_mspt();
    let _ = refresh_player_with_sample(player, runtime, &config, raw_tps, mspt);
}

fn refresh_player_with_sample(
    player: Player,
    runtime: &SharedRuntime,
    config: &Config,
    raw_tps: f64,
    mspt: f64,
) -> Option<PlayerKey> {
    let key = player_key(&player);
    let should_show = {
        let state = lock(runtime);
        state.persistent_state.is_enabled(key)
    } && player.has_permission(PERMISSION_NODE);

    if !should_show {
        remove_bar(runtime, key);
        return None;
    }

    let snapshot = DisplaySnapshot::new(raw_tps, mspt, player.get_ping(), config);
    let mut state = lock(runtime);
    if let Some(entry) = state.bars.get_mut(&key) {
        entry.bossbar.set_title(snapshot.title());
        entry.bossbar.set_health(snapshot.progress);
        if entry.severity != snapshot.bar_severity {
            entry.bossbar.set_color(snapshot.bossbar_color());
            entry.severity = snapshot.bar_severity;
        }
    } else {
        let bossbar = BossBar::new(
            snapshot.title(),
            snapshot.bossbar_color(),
            BossBarDivision::NoDivision,
        );
        bossbar.set_health(snapshot.progress);
        bossbar.add_player(player);
        state.bars.insert(
            key,
            PlayerBar {
                bossbar,
                severity: snapshot.bar_severity,
            },
        );
    }
    Some(key)
}

pub fn remove_bar(runtime: &SharedRuntime, key: PlayerKey) {
    let removed = lock(runtime).bars.remove(&key);
    drop(removed);
}

pub fn shutdown(runtime: &SharedRuntime) -> Result<(), String> {
    let (save_result, bars) = {
        let mut state = lock(runtime);
        let save_result = state.state_store.save(&state.persistent_state);
        let bars = mem::take(&mut state.bars);
        (save_result, bars)
    };
    drop(bars);
    save_result
}
