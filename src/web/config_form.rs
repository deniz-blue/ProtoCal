use std::collections::HashMap;

use crate::bot::guild_config::GuildConfigChange;

pub const EVENT_FIELD: &str = "event";
pub const ANNOUNCE_FIELD_PREFIX: &str = "announce:";
pub const REVIEW_FIELD_PREFIX: &str = "review:";

fn values_by_forum(form: &HashMap<String, Vec<String>>, prefix: &str) -> HashMap<String, String> {
    form.iter()
        .filter_map(|(field, values)| match values.as_slice() {
            [value] => Some((field.strip_prefix(prefix)?.to_owned(), value.clone())),
            _ => None,
        })
        .collect()
}

pub fn guild_config_change_of_form(body: &[u8]) -> GuildConfigChange {
    let mut form: HashMap<String, Vec<String>> = HashMap::new();
    for (field, value) in url::form_urlencoded::parse(body) {
        form.entry(field.into_owned()).or_default().push(value.into_owned());
    }
    GuildConfigChange {
        event_forum_ids: form.get(EVENT_FIELD).into_iter().flatten().cloned().collect(),
        announce_role_id_of: values_by_forum(&form, ANNOUNCE_FIELD_PREFIX),
        review_channel_id_of: values_by_forum(&form, REVIEW_FIELD_PREFIX),
    }
}
