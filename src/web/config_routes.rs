use std::{collections::HashMap, sync::Arc};

use axum::{
    body::Bytes,
    extract::{OriginalUri, Path, Query, State},
    http::{StatusCode, header},
    response::IntoResponse,
};

use super::{
    Answered, html_answer, text_answer,
    config_form::guild_config_change_of_form,
    config_page::{ConfigPageOptions, config_page},
};
use crate::bot::Bot;

const CONFIG_LINK_EXPIRED: &str =
    "This settings link has run out, or you can no longer manage this server. Run /config again.";

pub async fn show_config_page(
    State(bot): State<Arc<Bot>>,
    Path(token): Path<String>,
    Query(query): Query<HashMap<String, String>>,
) -> Answered {
    let Some(guild) = bot.guild_of_config_grant(&token).await else {
        return Ok(text_answer(StatusCode::FORBIDDEN, CONFIG_LINK_EXPIRED));
    };
    let forums = bot.forum_configs_of(guild.id)?;
    let roles = bot.announceable_roles_of(&guild)?;
    let review_channels = bot.review_channels_of(guild.id);
    let options = ConfigPageOptions {
        guild_name: &guild.name,
        forums: &forums,
        roles: &roles,
        review_channels: &review_channels,
        saved: query.contains_key("saved"),
    };
    Ok(html_answer(config_page(&bot.links, &options)))
}

pub async fn save_config(
    State(bot): State<Arc<Bot>>,
    Path(token): Path<String>,
    OriginalUri(uri): OriginalUri,
    body: Bytes,
) -> Answered {
    let Some(guild) = bot.guild_of_config_grant(&token).await else {
        return Ok(text_answer(StatusCode::FORBIDDEN, CONFIG_LINK_EXPIRED));
    };
    bot.save_guild_config(&guild, &guild_config_change_of_form(&body))?;
    let saved_url = format!("{}?saved", uri.path());
    Ok((StatusCode::SEE_OTHER, [(header::LOCATION, saved_url)]).into_response())
}
