use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

/// Discord application ID for Rich Presence. Create one at
/// https://discord.com/developers/applications (name it e.g.
/// "Arknights: Endfield") and paste its Application ID here.
const DISCORD_APP_ID: &str = "";

/// Start Rich Presence for a play session. Returns a handle flag: store
/// `false` into it to clear the presence and disconnect. All failures
/// (Discord not running, no app id) are silent — presence is best-effort.
pub fn start_presence() -> Arc<AtomicBool> {
    let active = Arc::new(AtomicBool::new(true));

    if DISCORD_APP_ID.is_empty() {
        return active;
    }

    let active2 = active.clone();
    std::thread::spawn(move || {
        use discord_rich_presence::{
            activity::{Activity, Timestamps},
            DiscordIpc, DiscordIpcClient,
        };

        let mut client = DiscordIpcClient::new(DISCORD_APP_ID);
        if client.connect().is_err() {
            return;
        }

        let start = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);

        let _ = client.set_activity(
            Activity::new()
                .state("In game")
                .details("Arknights: Endfield")
                .timestamps(Timestamps::new().start(start)),
        );

        while active2.load(Ordering::SeqCst) {
            std::thread::sleep(std::time::Duration::from_secs(1));
        }

        let _ = client.clear_activity();
        let _ = client.close();
    });

    active
}
