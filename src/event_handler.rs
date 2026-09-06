use crate::storage::Storage;
use eros::Context;
use std::sync::Arc;
use tracing::{debug, info};
use twilight_gateway::Event;
use twilight_http::Client as HttpClient;

pub async fn handle(
    _http: Arc<HttpClient>,
    storage: Arc<Storage>,
    event: Event,
) -> eros::Result<()> {
    match event {
        Event::Ready(ready) => {
            info!(user = %ready.user.name, "bot logged in and ready");
        }
        Event::GuildCreate(guild) => {
            debug!(guild_id = %guild.id(), "saw guild");
        }
        Event::MemberUpdate(member) => {
            debug!(user_id = %member.user.id, "member update");
            if storage
                .is_linked(member.user.id.get())
                .await
                .context("checking if member is linked")?
            {
                storage
                    .queue_resync(member.user.id.get())
                    .await
                    .context("queueing resync after member update")?;
            }
        }
        _ => {}
    }
    Ok(())
}
