use super::{
    config_form::{ANNOUNCE_FIELD_PREFIX, EVENT_FIELD, REVIEW_FIELD_PREFIX},
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

const SAVE_BAR: &str = "<div class=\"bar\">
\t<button><span class=\"when-unsaved\">Save</span><span class=\"when-saved\">Saved</span></button>
</div>";

const SAVE_STATE_SCRIPT: &str = "<script>
\tconst form = document.querySelector(\"form\");
\thistory.replaceState(null, \"\", location.pathname);
\tform.addEventListener(\"input\", () => form.classList.remove(\"saved\"));
\tform.addEventListener(\"submit\", () => form.classList.add(\"busy\"));
\taddEventListener(\"pageshow\", () => form.classList.remove(\"busy\"));
</script>";

const fn shown_if(shown: bool, text: &str) -> &str {
    if shown { text } else { "" }
}

fn state_notes(forum: &ForumConfig) -> String {
    let named = forum.name.is_some();
    [(!named, "deleted"), (named && !forum.everyone_can_view, "hidden from @everyone")]
        .into_iter()
        .filter(|(shown, _)| *shown)
        .map(|(_, state)| format!("<span class=\"note\">{state}</span>"))
        .collect()
}

fn warning_row(shown: bool, warning: &str) -> String {
    if !shown {
        return String::new();
    }
    format!("\n\t\t\t<p class=\"row warning\">{warning}</p>")
}

fn option(value: &str, text: &str, selected: bool) -> String {
    format!("<option value=\"{}\"{}>{}</option>", escaped(value), shown_if(selected, " selected"), escaped(text))
}

fn select_row(label: &str, field: &str, aria_label: &str, options: &str) -> String {
    format!(
        "\n\t\t\t<div class=\"row\">\n\t\t\t\t<span>{label}</span>\n\t\t\t\t<select name=\"{}\" aria-label=\"{}\">\
         <option value=\"\">None</option>{options}</select>\n\t\t\t</div>",
        escaped(field),
        escaped(aria_label)
    )
}

fn announce_row(forum: &ForumConfig, roles: &[CachedRole], label_name: &str) -> String {
    if roles.is_empty() {
        return String::new();
    }
    let options: String = roles
        .iter()
        .map(|role| {
            let role_id = role.id.to_string();
            let selected = forum.announce_role_id.as_deref() == Some(role_id.as_str());
            option(&role_id, &format!("@{}", role.name), selected)
        })
        .collect();
    let field = format!("{ANNOUNCE_FIELD_PREFIX}{}", forum.forum_id);
    select_row("Announce role", &field, &format!("Announce role for {label_name}"), &options)
}

fn review_row(forum: &ForumConfig, channels: &[CachedChannel], label_name: &str) -> String {
    let options: String = channels
        .iter()
        .map(|channel| {
            let channel_id = channel.id.to_string();
            let selected = forum.review_channel_id.as_deref() == Some(channel_id.as_str());
            option(&channel_id, &format!("#{}", channel.name), selected)
        })
        .collect();
    let field = format!("{REVIEW_FIELD_PREFIX}{}", forum.forum_id);
    select_row("Review channel", &field, &format!("Review channel for {label_name}"), &options)
}

fn index_row(links: &Links, forum: &ForumConfig) -> String {
    if !(forum.is_event_forum && forum.everyone_can_view) {
        return String::new();
    }
    let index_url = links.forum_index_url(forum.forum_id);
    format!("\n\t\t\t<a class=\"row\" href=\"{}\">Open Evnt index</a>", escaped(&index_url))
}

fn forum_block(links: &Links, forum: &ForumConfig, options: &ConfigPageOptions) -> String {
    let id = forum.forum_id.to_string();
    let shown_name = forum.name.as_ref().map_or_else(|| id.clone(), |name| format!("#{name}"));
    let label_name = forum.name.as_ref().unwrap_or(&id);
    let details = [
        warning_row(forum.name.is_some() && !forum.has_default_reaction, "no default reaction"),
        announce_row(forum, options.roles, label_name),
        review_row(forum, options.review_channels, label_name),
        warning_row(forum.everyone_can_post, "@everyone has Create Posts"),
        index_row(links, forum),
    ]
    .concat();
    format!(
        "<section class=\"forum\">
\t<label class=\"row\">
\t\t<span>{}{}</span>
\t\t<input class=\"switch\" type=\"checkbox\" role=\"switch\" name=\"{EVENT_FIELD}\" value=\"{}\"{} />
\t</label>
\t<div class=\"details\">
\t\t<div>{details}
\t\t</div>
\t</div>
</section>
",
        escaped(&shown_name),
        state_notes(forum),
        escaped(&id),
        shown_if(forum.is_event_forum, " checked"),
    )
}

fn settings_form(links: &Links, options: &ConfigPageOptions) -> String {
    let forums: String = options.forums.iter().map(|forum| forum_block(links, forum, options)).collect();
    format!(
        "<form method=\"post\"{}>\n<h2>Event forums</h2>\n{forums}{SAVE_BAR}\n</form>\n{SAVE_STATE_SCRIPT}",
        shown_if(options.saved, " class=\"saved\""),
    )
}

pub fn config_page(links: &Links, options: &ConfigPageOptions) -> String {
    let content = if options.forums.is_empty() {
        "<p class=\"empty\">No forums</p>".to_owned()
    } else {
        settings_form(links, options)
    };
    let body = format!("<h1>{}</h1>\n{content}", escaped(options.guild_name));
    page(&format!("ProtoCal · {}", options.guild_name), &body)
}
