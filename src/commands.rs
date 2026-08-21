use pumpkin_plugin_api::{
    Server,
    command::CommandNode,
    command_wit::{CommandError, CommandSender, ConsumedArgs},
    commands::{Command, CommandHandler},
    text::{NamedColor, TextComponent},
};

use crate::config::BarMetric;
use crate::runtime::{
    PERMISSION_NODE, SharedRuntime, lock, player_key, refresh_all, refresh_player, remove_bar,
};

pub fn build(runtime: SharedRuntime) -> Command {
    let command = Command::new(
        &["tpsbar".into()],
        "切换个人 TPS BossBar，或设置血条进度指标 / Toggle your TPSBar or set its progress metric",
    )
    .execute(ToggleHandler {
        runtime: runtime.clone(),
    });

    let by = CommandNode::literal("by");
    by.then(CommandNode::literal("mspt").execute(SetMetricHandler {
        runtime: runtime.clone(),
        metric: BarMetric::Mspt,
    }));
    by.then(CommandNode::literal("tps").execute(SetMetricHandler {
        runtime: runtime.clone(),
        metric: BarMetric::Tps,
    }));
    by.then(CommandNode::literal("ping").execute(SetMetricHandler {
        runtime,
        metric: BarMetric::Ping,
    }));
    command.then(by);
    command
}

struct ToggleHandler {
    runtime: SharedRuntime,
}

impl CommandHandler for ToggleHandler {
    fn handle(
        &self,
        sender: CommandSender,
        server: Server,
        _args: ConsumedArgs,
    ) -> Result<i32, CommandError> {
        if !sender.has_permission(&server, PERMISSION_NODE) {
            return Err(CommandError::PermissionDenied);
        }

        let Some(player) = sender.as_player() else {
            let message = {
                let state = lock(&self.runtime);
                state.localizer.message(
                    &state.config.fallback_locale,
                    &state.config.fallback_locale,
                    "error.player_only",
                    &[],
                )
            };
            return Err(CommandError::CommandFailed(colored_message(
                &message,
                NamedColor::Red,
            )));
        };

        let key = player_key(&player);
        let locale = player.get_locale();
        let toggled = {
            let mut state = lock(&self.runtime);
            let enabled = state.persistent_state.toggle(key);
            let store = state.state_store.clone();
            match store.save(&state.persistent_state) {
                Ok(()) => Ok(enabled),
                Err(error) => {
                    let _ = state.persistent_state.toggle(key);
                    let message = state.localizer.message(
                        &locale,
                        &state.config.fallback_locale,
                        "error.state_save",
                        &[("error", &error)],
                    );
                    Err(message)
                }
            }
        };

        let enabled = match toggled {
            Ok(enabled) => enabled,
            Err(message) => {
                return Err(CommandError::CommandFailed(colored_message(
                    &message,
                    NamedColor::Red,
                )));
            }
        };

        let message = {
            let state = lock(&self.runtime);
            state.localizer.message(
                &locale,
                &state.config.fallback_locale,
                if enabled {
                    "command.enabled"
                } else {
                    "command.disabled"
                },
                &[],
            )
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
            return Err(CommandError::PermissionDenied);
        }

        let locale = sender
            .as_player()
            .map_or_else(|| "en_us".to_string(), |player| player.get_locale());
        let message = {
            let mut state = lock(&self.runtime);
            state.config.bar.metric = self.metric;
            state.localizer.message(
                &locale,
                &state.config.fallback_locale,
                "command.metric_set",
                &[("metric", self.metric.label())],
            )
        };

        refresh_all(&server, &self.runtime);
        sender.send_message(colored_message(&message, NamedColor::Green));
        Ok(1)
    }
}

fn colored_message(message: &str, color: NamedColor) -> TextComponent {
    let component = TextComponent::text(message);
    component.color_named(color);
    component
}
