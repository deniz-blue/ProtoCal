mod bot;
mod components;
mod config;
mod db;
mod discord;
mod gateway;
mod grants;
mod meet;
mod open_evnt;
mod records;
mod text;
mod time;
mod web;

use std::{env, sync::Arc};

use anyhow::Context as _;
use tracing_subscriber::EnvFilter;
use twilight_http::Client;
use twilight_model::id::{
    Id,
    marker::{ApplicationMarker, GuildMarker},
};

use crate::{bot::Bot, config::Config, db::Database};

fn token_of_env() -> Option<String> {
    env::var("DISCORD_BOT_TOKEN").ok().filter(|token| !token.is_empty())
}

async fn publish_commands(global: bool) -> anyhow::Result<()> {
    let http = Client::new(token_of_env().context("DISCORD_BOT_TOKEN is not set")?);
    let application_id: Id<ApplicationMarker> = env::var("DISCORD_APP_ID")?.parse().context("DISCORD_APP_ID")?;
    let commands = bot::commands::all_commands();
    tracing::info!(commands = ?commands.iter().map(|command| &command.name).collect::<Vec<_>>(), "Publishing commands");
    let interaction = http.interaction(application_id);
    if global {
        interaction.set_global_commands(&commands).await?;
    } else {
        let guild_id: Id<GuildMarker> = env::var("DISCORD_GUILD_ID")?.parse().context("DISCORD_GUILD_ID")?;
        interaction.set_guild_commands(guild_id, &commands).await?;
    }
    tracing::info!("Commands published");
    Ok(())
}

async fn run(config: Config) -> anyhow::Result<()> {
    if let Some(directory) = config.database_path.parent() {
        std::fs::create_dir_all(directory)?;
    }
    let db = Database::open(&config.database_path)?;
    let Some(token) = token_of_env() else {
        tracing::error!("DISCORD_BOT_TOKEN is not set in the environment variables");
        let bot = Arc::new(Bot::new(Client::new(String::new()), Id::new(1), Id::new(1), db, config.links));
        return web::serve(bot, &config.http_host, config.http_port).await;
    };
    let http = Client::new(token.clone());
    let application_id = http.current_user_application().await?.model().await?.id;
    let user_id = http.current_user().await?.model().await?.id;
    let bot = Arc::new(Bot::new(http, application_id, user_id, db, config.links));
    let web_bot = Arc::clone(&bot);
    tokio::spawn(async move {
        if let Err(error) = web::serve(web_bot, &config.http_host, config.http_port).await {
            tracing::error!(?error, "HTTP server failed");
        }
    });
    gateway::connect(bot, token).await
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let _ = dotenvy::dotenv();
    let level = env::var("LOG_LEVEL").unwrap_or_else(|_| "info".to_owned());
    tracing_subscriber::fmt().with_env_filter(EnvFilter::new(level)).init();
    rustls::crypto::ring::default_provider()
        .install_default()
        .map_err(|_| anyhow::anyhow!("another TLS crypto provider was installed first"))?;
    let arguments: Vec<String> = env::args().skip(1).collect();
    if arguments.first().map(String::as_str) == Some("publish") {
        return publish_commands(arguments.iter().any(|argument| argument == "--global")).await;
    }
    run(Config::from_env()?).await
}
