use std::sync::Arc;
use tracing::{debug, info};
use twilight_gateway::Event;
use twilight_http::Client as HttpClient;

pub async fn handle(_http: Arc<HttpClient>, event: Event) -> eros::Result<()> {
    match event {
        Event::Ready(ready) => {
            info!(user = %ready.user.name, "bot logged in and ready");
        }
        Event::GuildCreate(guild) => {
            debug!(guild_id = %guild.id(), "saw guild");
        }
        Event::MemberUpdate(member) => {
            debug!(user_id = %member.user.id, "member update");
        }
        _ => {}
    }
    Ok(())
}
