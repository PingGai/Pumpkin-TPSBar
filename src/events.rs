use pumpkin_plugin_api::{
    Server,
    events::{EventData, EventHandler, PlayerLeaveEvent},
};

use crate::runtime::{SharedRuntime, player_key, remove_bar};

pub struct LeaveHandler {
    pub runtime: SharedRuntime,
}

impl EventHandler<PlayerLeaveEvent> for LeaveHandler {
    fn handle(
        &self,
        _server: Server,
        event: EventData<PlayerLeaveEvent>,
    ) -> EventData<PlayerLeaveEvent> {
        remove_bar(&self.runtime, player_key(&event.player));
        event
    }
}
