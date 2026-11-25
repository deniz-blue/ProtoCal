use crate::text::cut_to_utf16_length;

// Discord refuses a role name longer than 100 characters.
const LONGEST_ROLE_NAME: usize = 100;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReactionEmoji {
    pub id: Option<String>,
    pub name: Option<String>,
}

impl ReactionEmoji {
    pub fn key(&self) -> Option<String> {
        self.id.clone().or_else(|| self.name.clone())
    }
}

// Discord draws a custom emoji from its id alone; the name is only the text shown when it can't.
pub fn emoji_text_of_reaction_key(key: &str) -> String {
    if !key.is_empty() && key.bytes().all(|byte| byte.is_ascii_digit()) {
        format!("<:emoji:{key}>")
    } else {
        key.to_owned()
    }
}

pub fn role_name_of_event(event_name: &str) -> String {
    cut_to_utf16_length(&format!("Event: {event_name}"), LONGEST_ROLE_NAME).to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn emoji(id: Option<&str>, name: Option<&str>) -> ReactionEmoji {
        ReactionEmoji {
            id: id.map(str::to_owned),
            name: name.map(str::to_owned),
        }
    }

    #[test]
    fn a_custom_emoji_is_known_by_its_id_a_unicode_one_by_itself() {
        assert_eq!(emoji(Some("1550491800844636291"), Some("hello")).key().as_deref(), Some("1550491800844636291"));
        assert_eq!(emoji(None, Some("👋")).key().as_deref(), Some("👋"));
        assert_eq!(emoji(None, None).key(), None);
    }

    #[test]
    fn a_reaction_key_is_written_the_way_discord_shows_it_in_a_message() {
        assert_eq!(emoji_text_of_reaction_key("1550491800844636291"), "<:emoji:1550491800844636291>");
        assert_eq!(emoji_text_of_reaction_key("👋"), "👋");
    }

    #[test]
    fn a_role_name_is_cut_to_what_discord_accepts() {
        assert_eq!(role_name_of_event("Bar meet"), "Event: Bar meet");
        assert_eq!(role_name_of_event(&"x".repeat(200)).len(), 100);
    }
}
