use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

use twilight_gateway::{CloseFrame, Config, Event, EventTypeFlags, Shard, StreamExt as _};

use crate::bot::{Bot, clock::MeetClock};

static SHUTTING_DOWN: AtomicBool = AtomicBool::new(false);

pub async fn connect(bot: Arc<Bot>, token: String) -> anyhow::Result<()> {
    let config = Config::new(token, Bot::intents());
    let shards: Vec<Shard> = twilight_gateway::create_recommended(&bot.http, config, |_, builder| builder.build())
        .await?
        .collect();
    let senders: Vec<_> = shards.iter().map(Shard::sender).collect();
    let tasks: Vec<_> = shards.into_iter().map(|shard| tokio::spawn(run_shard(shard, Arc::clone(&bot)))).collect();
    tokio::signal::ctrl_c().await?;
    tracing::info!("Shutting down");
    SHUTTING_DOWN.store(true, Ordering::Relaxed);
    for sender in senders {
        let _ = sender.close(CloseFrame::NORMAL);
    }
    for task in tasks {
        task.await?;
    }
    Ok(())
}

async fn run_shard(mut shard: Shard, bot: Arc<Bot>) {
    while let Some(item) = shard.next_event(EventTypeFlags::all()).await {
        let event = match item {
            Ok(Event::GatewayClose(_)) if SHUTTING_DOWN.load(Ordering::Relaxed) => break,
            Ok(event) => event,
            Err(error) => {
                tracing::warn!(?error, "Receiving a gateway event failed");
                continue;
            }
        };
        if let Event::Ready(ready) = &event {
            tracing::info!(user = %ready.user.name, "Ready");
            MeetClock::check_now(&bot);
        }
        let bot = Arc::clone(&bot);
        tokio::spawn(async move {
            if let Err(error) = bot.handle_event(event).await {
                tracing::error!(?error, "Event handler failed");
            }
        });
    }
}
