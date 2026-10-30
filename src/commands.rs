use pumpkin_plugin_api::{
    Server,
    command::CommandNode,
    command_wit::{CommandError, CommandSender, ConsumedArgs},
    commands::{Command, CommandHandler},
    text::{NamedColor, TextComponent},
};

use crate::{
    config::BarMetric,
    i18n::diagnostic,
    runtime::{PERMISSION_NODE, SharedRuntime, lock, player_key, refresh_player, remove_bar},
};

pub fn build(runtime: SharedRuntime) -> Command {
    let by = CommandNode::literal("by")
        .then(CommandNode::literal("mspt").execute(SetMetricHandler {
            runtime: runtime.clone(),
            metric: BarMetric::Mspt,
        }))
        .then(CommandNode::literal("tps").execute(SetMetricHandler {
            runtime: runtime.clone(),
            metric: BarMetric::Tps,
        }))
        .then(CommandNode::literal("ping").execute(SetMetricHandler {
            runtime: runtime.clone(),
            metric: BarMetric::Ping,
        }));
    let description = {
        let state = lock(&runtime);
        state.localizer.message(
            &state.config.fallback_locale,
            &state.config.fallback_locale,
            "player.command.description",
            &[],
        )
    };
    Command::new(&["tpsbar".into()], &description)
        .execute(ToggleHandler {
            runtime: runtime.clone(),
            show_hint: false,
        })
        .then(CommandNode::literal("empty=toggle").execute(ToggleHandler {
            runtime,
            show_hint: true,
        }))
        .then(by)
}

struct ToggleHandler {
    runtime: SharedRuntime,
    show_hint: bool,
}

impl CommandHandler for ToggleHandler {
    fn handle(
        &self,
        sender: CommandSender,
        server: Server,
        _args: ConsumedArgs,
    ) -> Result<i32, CommandError> {
        if !sender.has_permission(&server, PERMISSION_NODE) {
            return Err(CommandError::CommandFailed(colored_message(
                &sender_message(&self.runtime, &sender, "player.error.permission_denied"),
                NamedColor::Red,
            )));
        }
        let Some(player) = sender.as_player() else {
            return Err(CommandError::CommandFailed(colored_message(
                &diagnostic(
                    &lock(&self.runtime).config.fallback_locale,
                    "command.player_only",
                    &[],
                ),
                NamedColor::Red,
            )));
        };
        let key = player_key(&player);
        let locale = player.get_locale();
        let enabled = {
            let mut state = lock(&self.runtime);
            let enabled = state.persistent_state.toggle(key);
            let store = state.state_store.clone();
            if let Err(error) = store.save(&state.persistent_state) {
                let _ = state.persistent_state.toggle(key);
                tracing::warn!(%error, "{}", diagnostic(&state.config.fallback_locale, "state.save_failed", &[]));
                return Err(CommandError::CommandFailed(colored_message(
                    &state.localizer.message(
                        &locale,
                        &state.config.fallback_locale,
                        "player.error.state_save",
                        &[],
                    ),
                    NamedColor::Red,
                )));
            }
            enabled
        };
        let message = {
            let state = lock(&self.runtime);
            let mut message = state.localizer.message(
                &locale,
                &state.config.fallback_locale,
                if enabled {
                    "player.command.enabled"
                } else {
                    "player.command.disabled"
                },
                &[],
            );
            if self.show_hint {
                message.push(' ');
                message.push_str(&state.localizer.message(
                    &locale,
                    &state.config.fallback_locale,
                    "player.command.empty_hint",
                    &[],
                ));
            }
            message
        };
        if enabled {
            refresh_player(&server, player, &self.runtime);
        } else {
            remove_bar(&self.runtime, key);
        }
        sender.send_message(colored_message(
            &message,
            if enabled {
                NamedColor::Green
            } else {
                NamedColor::Gray
            },
        ));
        Ok(i32::from(enabled))
    }
}

struct SetMetricHandler {
    runtime: SharedRuntime,
    metric: BarMetric,
}

impl CommandHandler for SetMetricHandler {
    fn handle(
        &self,
        sender: CommandSender,
        server: Server,
        _args: ConsumedArgs,
    ) -> Result<i32, CommandError> {
        if !sender.has_permission(&server, PERMISSION_NODE) {
            return Err(CommandError::CommandFailed(colored_message(
                &sender_message(&self.runtime, &sender, "player.error.permission_denied"),
                NamedColor::Red,
            )));
        }
        let Some(player) = sender.as_player() else {
            return Err(CommandError::CommandFailed(colored_message(
                &diagnostic(
                    &lock(&self.runtime).config.fallback_locale,
                    "command.player_only",
                    &[],
                ),
                NamedColor::Red,
            )));
        };
        let key = player_key(&player);
        let locale = player.get_locale();
        let message = {
            let mut state = lock(&self.runtime);
            let previous = state.persistent_state.set_metric(key, self.metric);
            let store = state.state_store.clone();
            if let Err(error) = store.save(&state.persistent_state) {
                state.persistent_state.restore_metric(key, previous);
                tracing::warn!(%error, "{}", diagnostic(&state.config.fallback_locale, "state.save_failed", &[]));
                return Err(CommandError::CommandFailed(colored_message(
                    &state.localizer.message(
                        &locale,
                        &state.config.fallback_locale,
                        "player.error.state_save",
                        &[],
                    ),
                    NamedColor::Red,
                )));
            }
            state.localizer.message(
                &locale,
                &state.config.fallback_locale,
                "player.command.metric_set",
                &[(
                    "metric",
                    &state.localizer.message(
                        &locale,
                        &state.config.fallback_locale,
                        self.metric.translation_key(),
                        &[],
                    ),
                )],
            )
        };
        refresh_player(&server, player, &self.runtime);
        sender.send_message(colored_message(&message, NamedColor::Green));
        Ok(1)
    }
}

impl BarMetric {
    const fn translation_key(self) -> &'static str {
        match self {
            Self::Mspt => "player.metric.mspt",
            Self::Tps => "player.metric.tps",
            Self::Ping => "player.metric.ping",
        }
    }
}

fn sender_message(runtime: &SharedRuntime, sender: &CommandSender, key: &str) -> String {
    let locale = sender.as_player().map(|player| player.get_locale());
    let state = lock(runtime);
    state.localizer.message(
        locale.as_deref().unwrap_or(&state.config.fallback_locale),
        &state.config.fallback_locale,
        key,
        &[],
    )
}

fn colored_message(message: &str, color: NamedColor) -> TextComponent {
    TextComponent::text(message).color_named(color)
}
