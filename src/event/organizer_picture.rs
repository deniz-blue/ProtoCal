use sha2::{Digest as _, Sha256};
use twilight_model::{
    guild::Member,
    id::{
        Id,
        marker::{GuildMarker, UserMarker},
    },
};

use super::stacked_avatars::{StackFace, StackLayout};

const CDN: &str = "https://cdn.discordapp.com";
const ATTACHMENT_SCHEME: &str = "attachment://";
const STACK_KEY_BYTES: usize = 6;

// Discord's CDN answers a .png request for an animated hash with its first frame.
pub fn avatar_url_of(guild_id: Id<GuildMarker>, user_id: Id<UserMarker>, member: Option<&Member>) -> String {
    let server = member.and_then(|member| member.avatar);
    let user = member.and_then(|member| member.user.avatar);
    match (server, user) {
        (Some(hash), _) => format!("{CDN}/guilds/{guild_id}/users/{user_id}/avatars/{hash}.png?size=256"),
        (None, Some(hash)) => format!("{CDN}/avatars/{user_id}/{hash}.png?size=256"),
        (None, None) => format!("{CDN}/embed/avatars/{}.png", (user_id.get() >> 22) % 6),
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OrganizerPicture {
    pub avatar_urls: Vec<String>,
}

impl OrganizerPicture {
    pub fn stack_layout(&self) -> Option<StackLayout> {
        (self.avatar_urls.len() > 1).then(|| StackLayout::of_organizer_count(self.avatar_urls.len()))
    }

    pub fn stack_file_name(&self) -> Option<String> {
        let faces: Vec<String> = self
            .stack_layout()?
            .circles
            .iter()
            .map(|circle| match circle.face {
                StackFace::Avatar(index) => self.avatar_urls[index].clone(),
                StackFace::HiddenCount(count) => format!("+{count}"),
            })
            .collect();
        let digest = Sha256::digest(faces.join("\n").as_bytes());
        let key: String = digest[..STACK_KEY_BYTES].iter().map(|byte| format!("{byte:02x}")).collect();
        Some(format!("organizers-{key}.png"))
    }

    pub fn thumbnail_url(&self) -> Option<String> {
        match self.stack_file_name() {
            Some(file_name) => Some(format!("{ATTACHMENT_SCHEME}{file_name}")),
            None => self.avatar_urls.first().cloned(),
        }
    }
}

// Discord answers an attachment:// thumbnail with the uploaded file's CDN URL, which ends in the same file name.
pub fn thumbnail_shows(posted_url: &str, wanted_url: &str) -> bool {
    match wanted_url.strip_prefix(ATTACHMENT_SCHEME) {
        Some(file_name) => file_name_of_url(posted_url) == file_name,
        None => posted_url == wanted_url,
    }
}

fn file_name_of_url(url: &str) -> &str {
    let path = url.split(['?', '#']).next().unwrap_or(url);
    path.rsplit('/').next().unwrap_or(path)
}
