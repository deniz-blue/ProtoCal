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

    pub fn as_text(&self) -> String {
        match &self.id {
            Some(id) => format!("<:{}:{id}>", self.name.as_deref().unwrap_or("emoji")),
            None => self.name.clone().unwrap_or_default(),
        }
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
    fn an_emoji_is_written_the_way_discord_shows_it_in_a_message() {
        assert_eq!(emoji(Some("1550491800844636291"), Some("hello")).as_text(), "<:hello:1550491800844636291>");
        assert_eq!(emoji(None, Some("👋")).as_text(), "👋");
    }

    #[test]
    fn a_role_name_is_cut_to_what_discord_accepts() {
        assert_eq!(role_name_of_event("Bar meet"), "Event: Bar meet");
        assert_eq!(role_name_of_event(&"x".repeat(200)).len(), 100);
    }
}
