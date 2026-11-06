use super::{
    config_form::{ANNOUNCE_FIELD_PREFIX, MEET_FIELD, REVIEW_FIELD_PREFIX},
    page::{escaped, page},
};
use crate::{
    bot::guild_config::ForumConfig,
    config::Links,
    discord::cache::{CachedChannel, CachedRole},
};

pub struct ConfigPageOptions<'a> {
    pub guild_name: &'a str,
    pub forums: &'a [ForumConfig],
    pub roles: &'a [CachedRole],
    pub review_channels: &'a [CachedChannel],
    pub saved: bool,
}

fn forum_notes(forum: &ForumConfig) -> Vec<&'static str> {
    let named = forum.name.is_some();
    [
        (!named, "deleted"),
        (named && !forum.everyone_can_view, "hidden from @everyone"),
        (forum.is_meet_forum && named && !forum.has_default_reaction, "no default reaction"),
        (
            forum.review_channel_id.is_some() && forum.everyone_can_post,
            "screened, but @everyone can still make posts by hand",
        ),
    ]
    .into_iter()
    .filter_map(|(shown, note)| shown.then_some(note))
    .collect()
}

fn shown_if(shown: bool, text: &str) -> &str {
    if shown { text } else { "" }
}

fn role_option(role: &CachedRole, forum: &ForumConfig) -> String {
    let role_id = role.id.to_string();
    let selected = forum.announce_role_id.as_deref() == Some(role_id.as_str());
    format!(
        "<option value=\"{}\" {}>\n\t\t\t\t\t\t@{}\n\t\t\t\t\t</option>",
        escaped(&role_id),
        shown_if(selected, "selected"),
        escaped(&role.name)
    )
}

fn channel_option(channel: &CachedChannel, forum: &ForumConfig) -> String {
    let channel_id = channel.id.to_string();
    let selected = forum.review_channel_id.as_deref() == Some(channel_id.as_str());
    format!(
        "<option\n\t\t\t\t\t\tvalue=\"{}\"\n\t\t\t\t\t\t{}\n\t\t\t\t\t>\n\t\t\t\t\t\t#{}\n\t\t\t\t\t</option>",
        escaped(&channel_id),
        shown_if(selected, "selected"),
        escaped(&channel.name)
    )
}

fn select(name: &str, label: &str, options: &str) -> String {
    format!(
        "\t\t<select\n\t\t\tname=\"{}\"\n\t\t\taria-label=\"{}\"\n\t\t>\n\t\t\t<option value=\"\">None</option>\n\t\t\t{options}\n\t\t</select>",
        escaped(name),
        escaped(label)
    )
}

fn forum_row(links: &Links, forum: &ForumConfig, options: &ConfigPageOptions) -> String {
    let id = escaped(&forum.forum_id.to_string());
    let shown_name = forum.name.as_ref().map_or_else(|| forum.forum_id.to_string(), |name| format!("#{name}"));
    let label_name = forum.name.clone().unwrap_or_else(|| forum.forum_id.to_string());
    let notes: String = forum_notes(forum)
        .iter()
        .map(|note| format!("<br /><span class=\"hint\">{note}</span>"))
        .collect();
    let index_link = if forum.is_meet_forum && forum.everyone_can_view {
        let index_url = links.forum_index_url(forum.forum_id);
        format!("<br /><a class=\"hint\" href=\"{}\">Open Evnt index</a>", escaped(&index_url))
    } else {
        String::new()
    };
    let roles: String = options.roles.iter().map(|role| role_option(role, forum)).collect();
    let channels: String = options.review_channels.iter().map(|channel| channel_option(channel, forum)).collect();
    let announce = select(&format!("{ANNOUNCE_FIELD_PREFIX}{}", forum.forum_id), &format!("Announce role for {label_name}"), &roles);
    let review = select(&format!("{REVIEW_FIELD_PREFIX}{}", forum.forum_id), &format!("Review channel for {label_name}"), &channels);
    format!(
        " <tr>\n\t<td>\n\t\t<label for=\"meet-{id}\"\n\t\t\t>{}</label\n\t\t>\n\t\t{notes}\n\t\t{index_link}\n\t</td>\n\
         \t<td>\n\t\t<input\n\t\t\ttype=\"checkbox\"\n\t\t\tid=\"meet-{id}\"\n\t\t\tname=\"{MEET_FIELD}\"\n\t\t\tvalue=\"{id}\"\n\t\t\t{}\n\t\t/>\n\t</td>\n\
         \t<td>\n{announce}\n\t</td>\n\t<td>\n{review}\n\t</td>\n</tr>",
        escaped(&shown_name),
        shown_if(forum.is_meet_forum, "checked"),
    )
}

const MEET_FORUMS_HINTS: &str = "\t\t\t\t<p class=\"hint\">
\t\t\t\t\tEvery new post in a meet forum becomes a meet. The announce role is pinged when one is
\t\t\t\t\tposted. Members react with the forum's default reaction to attend.
\t\t\t\t</p>
\t\t\t\t<p class=\"hint\">
\t\t\t\t\tWith a review channel, a new meet goes there first, and anyone with Manage Threads in the
\t\t\t\t\tforum posts, edits or declines it. Turn Create Posts off for members in a screened forum,
\t\t\t\t\tso the bot's form is the only way in.
\t\t\t\t</p>";

const TABLE_HEAD: &str = "<table>
\t\t\t\t\t\t\t<thead>
\t\t\t\t\t\t\t\t<tr>
\t\t\t\t\t\t\t\t\t<th>Forum</th>
\t\t\t\t\t\t\t\t\t<th>Meets</th>
\t\t\t\t\t\t\t\t\t<th>Announce role</th>
\t\t\t\t\t\t\t\t\t<th>Review channel</th>
\t\t\t\t\t\t\t\t</tr>
\t\t\t\t\t\t\t</thead>
\t\t\t\t\t\t\t<tbody>
\t\t\t\t\t\t\t\t";

const TABLE_FOOT: &str = "
\t\t\t\t\t\t\t</tbody>
\t\t\t\t\t\t</table>";

pub fn config_page(links: &Links, options: &ConfigPageOptions) -> String {
    let title = format!("ProtoCal · {}", options.guild_name);
    let forums = if options.forums.is_empty() {
        "<p>This server has no forums.</p>".to_owned()
    } else {
        let rows: String = options.forums.iter().map(|forum| forum_row(links, forum, options)).collect();
        format!("{TABLE_HEAD}{rows}{TABLE_FOOT}")
    };
    let body = format!(
        "<h1>{}</h1>\n\t\t\t{}\n\t\t\t<form method=\"post\">\n\t\t\t\t<h2>Meet forums</h2>\n{MEET_FORUMS_HINTS}\n\t\t\t\t{forums}\n\t\t\t\t<button>Save</button>\n\t\t\t</form>",
        escaped(&title),
        shown_if(options.saved, "<p>Saved.</p>"),
    );
    page(&title, &body)
}
