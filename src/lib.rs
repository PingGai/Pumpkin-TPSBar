mod commands;
mod config;
mod display;
mod events;
mod i18n;
mod metrics;
mod runtime;
mod state;

use std::{path::PathBuf, sync::Mutex};

use pumpkin_plugin_api::{
    Context, Plugin, PluginMetadata,
    events::EventPriority,
    permission::{Permission, PermissionDefault, PermissionLevel},
    permissions, scheduler,
};

use crate::{
    config::load_or_create,
    events::LeaveHandler,
    i18n::{Localizer, diagnostic},
    runtime::{
        PERMISSION_NODE, RuntimeState, SharedRuntime, lock, new_shared_runtime, refresh_all,
        shutdown,
    },
    state::StateStore,
};

const BUILD_VERSION: &str = match option_env!("TPSBAR_BUILD_VERSION") {
    Some(version) => version,
    None => env!("CARGO_PKG_VERSION"),
};

struct TpsBarPlugin {
    runtime: SharedRuntime,
    refresh_task_id: Mutex<Option<u32>>,
}

impl Plugin for TpsBarPlugin {
    fn new() -> Self {
        Self {
            runtime: new_shared_runtime(),
            refresh_task_id: Mutex::new(None),
        }
    }

    fn metadata(&self) -> PluginMetadata {
        PluginMetadata {
            name: env!("CARGO_PKG_NAME").to_string(),
            version: BUILD_VERSION.to_string(),
            authors: vec!["PING".to_string()],
            description: diagnostic("zh_cn", "plugin.description", &[]),
            dependencies: vec![],
            permissions: vec![
                permissions::FS_READ_DATA.to_string(),
                permissions::FS_WRITE_DATA.to_string(),
            ],
        }
    }

    fn on_load(&self, context: Context) -> pumpkin_plugin_api::Result<()> {
        let data_folder = PathBuf::from(context.get_data_folder());
        let loaded_config = load_or_create(&data_folder)?;
        if let Some(warning) = &loaded_config.warning {
            tracing::warn!(%warning);
        }
        tracing::info!(path = %loaded_config.path.display(), "{}", diagnostic(&loaded_config.config.fallback_locale, "config.loaded", &[]));

        let localizer = Localizer::load()?;
        let state_store =
            StateStore::new(&data_folder).with_locale(&loaded_config.config.fallback_locale);
        let loaded_state = state_store.load()?;
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
            description: {
                let state = lock(&self.runtime);
                state.localizer.message(
                    &state.config.fallback_locale,
                    &state.config.fallback_locale,
                    "diagnostic.permission.description",
                    &[],
                )
            },
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
        let refresh_task_id =
            scheduler::schedule_repeating_task(1, refresh_interval_ticks, move |server| {
                refresh_all(&server, &runtime);
            });
        *self
            .refresh_task_id
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = Some(refresh_task_id);

        tracing::info!(
            version = BUILD_VERSION,
            refresh_interval_ticks,
            permission = PERMISSION_NODE,
            "{}",
            diagnostic(
                &lock(&self.runtime).config.fallback_locale,
                "plugin.loaded",
                &[]
            )
        );
        Ok(())
    }

    fn on_unload(&self, _context: Context) -> pumpkin_plugin_api::Result<()> {
        let task_id = self
            .refresh_task_id
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .take();
        if let Some(task_id) = task_id {
            scheduler::cancel_task(task_id);
        }
        let result = shutdown(&self.runtime);
        tracing::info!(
            "{}",
            diagnostic(
                &lock(&self.runtime).config.fallback_locale,
                "plugin.unloaded",
                &[]
            )
        );
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
