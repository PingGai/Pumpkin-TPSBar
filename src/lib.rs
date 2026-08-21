mod commands;
mod config;
mod display;
mod events;
mod i18n;
mod metrics;
mod runtime;
mod state;

use std::path::PathBuf;

use pumpkin_plugin_api::{
    Context, Plugin, PluginMetadata,
    events::EventPriority,
    permission::{Permission, PermissionDefault, PermissionLevel},
    permissions, scheduler,
};

use crate::{
    config::load_or_create,
    events::LeaveHandler,
    i18n::Localizer,
    runtime::{
        PERMISSION_NODE, RuntimeState, SharedRuntime, lock, new_shared_runtime, refresh_all,
        shutdown,
    },
    state::StateStore,
};

struct TpsBarPlugin {
    runtime: SharedRuntime,
    refresh_task_id: Option<u32>,
}

impl Plugin for TpsBarPlugin {
    fn new() -> Self {
        Self {
            runtime: new_shared_runtime(),
            refresh_task_id: None,
        }
    }

    fn metadata(&self) -> PluginMetadata {
        PluginMetadata {
            name: env!("CARGO_PKG_NAME").to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            authors: vec!["PING".to_string()],
            description: env!("CARGO_PKG_DESCRIPTION").to_string(),
            dependencies: vec![],
            permissions: vec![
                permissions::FS_READ_DATA.to_string(),
                permissions::FS_WRITE_DATA.to_string(),
            ],
        }
    }

    fn on_load(&mut self, context: Context) -> pumpkin_plugin_api::Result<()> {
        let data_folder = PathBuf::from(context.get_data_folder());
        let loaded_config = load_or_create(&data_folder)?;
        if let Some(warning) = &loaded_config.warning {
            tracing::warn!(%warning);
        }
        tracing::info!(path = %loaded_config.path.display(), "TPSBar 配置已加载");

        let localizer = Localizer::load()?;
        let state_store = StateStore::new(&data_folder);
        let loaded_state = state_store.load();
        if let Some(warning) = &loaded_state.warning {
            tracing::warn!(%warning);
        }

        let refresh_interval_ticks = loaded_config.config.refresh_interval_ticks;
        let permission_level = permission_level(loaded_config.config.permission.default_op_level);
        *lock(&self.runtime) = RuntimeState::initialized(
            loaded_config.config,
            localizer,
            loaded_state.state,
            state_store,
        );

        context.register_permission(&Permission {
            node: PERMISSION_NODE.to_string(),
            description: "允许玩家切换自己的 TPSBar / Toggle personal TPSBar".to_string(),
            default: PermissionDefault::Op(permission_level),
            children: vec![],
        })?;
        context.register_command(commands::build(self.runtime.clone()), PERMISSION_NODE);
        context.register_event_handler(
            LeaveHandler {
                runtime: self.runtime.clone(),
            },
            EventPriority::Normal,
            false,
        )?;

        let runtime = self.runtime.clone();
        self.refresh_task_id = Some(scheduler::schedule_repeating_task(
            1,
            refresh_interval_ticks,
            move |server| refresh_all(&server, &runtime),
        ));

        tracing::info!(
            version = env!("CARGO_PKG_VERSION"),
            refresh_interval_ticks,
            permission = PERMISSION_NODE,
            "TPSBar 已加载"
        );
        Ok(())
    }

    fn on_unload(&mut self, _context: Context) -> pumpkin_plugin_api::Result<()> {
        if let Some(task_id) = self.refresh_task_id.take() {
            scheduler::cancel_task(task_id);
        }
        let result = shutdown(&self.runtime);
        tracing::info!("TPSBar 已卸载");
        result
    }
}

const fn permission_level(level: u8) -> PermissionLevel {
    match level {
        0 => PermissionLevel::Zero,
        1 => PermissionLevel::One,
        2 => PermissionLevel::Two,
        3 => PermissionLevel::Three,
        _ => PermissionLevel::Four,
    }
}

pumpkin_plugin_api::register_plugin!(TpsBarPlugin);
