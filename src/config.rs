use std::{env, path::PathBuf};

use anyhow::Context as _;
use url::Url;

#[derive(Clone, Debug)]
pub struct Links {
    pub public_url: String,
    pub editor_url: String,
    pub viewer_url: String,
    pub index_viewer_url: String,
}

pub struct Config {
    pub http_host: String,
    pub http_port: u16,
    pub database_path: PathBuf,
    pub links: Links,
}

fn env_or(name: &str, default: &str) -> String {
    env::var(name).unwrap_or_else(|_| default.to_owned())
}

pub fn database_path_of_url(database_url: &str) -> PathBuf {
    PathBuf::from(database_url.strip_prefix("file:").unwrap_or(database_url))
}

impl Config {
    pub fn from_env() -> anyhow::Result<Self> {
        let http_port = env_or("PROTOCAL_HTTP_PORT", "8660")
            .parse()
            .context("PROTOCAL_HTTP_PORT is not a port")?;
        let public_url = env_or("PROTOCAL_PUBLIC_URL", &format!("http://localhost:{http_port}"));
        Ok(Self {
            http_host: env_or("PROTOCAL_HTTP_HOST", "127.0.0.1"),
            http_port,
            database_path: database_path_of_url(&env_or("DATABASE_URL", "file:./data/protocal.db")),
            links: Links {
                public_url: public_url.strip_suffix('/').unwrap_or(&public_url).to_owned(),
                editor_url: env_or("PROTOCAL_EDITOR_URL", "https://eventsl.ink/edit"),
                viewer_url: env_or("PROTOCAL_VIEWER_URL", "https://eventsl.ink/event"),
                index_viewer_url: env_or("PROTOCAL_INDEX_VIEWER_URL", "https://eventsl.ink/index"),
            },
        })
    }
}

pub fn with_query(address: &str, pairs: &[(&str, &str)]) -> String {
    let Ok(mut url) = Url::parse(address) else {
        return address.to_owned();
    };
    let kept: Vec<(String, String)> = url
        .query_pairs()
        .filter(|(name, _)| !pairs.iter().any(|(replaced, _)| replaced == name))
        .map(|(name, value)| (name.into_owned(), value.into_owned()))
        .collect();
    url.query_pairs_mut().clear().extend_pairs(kept).extend_pairs(pairs);
    url.to_string()
}

impl Links {
    pub fn event_document_url(&self, thread_id: impl std::fmt::Display) -> String {
        format!("{}/events/{thread_id}.evnt.json", self.public_url)
    }

    pub fn forum_index_url(&self, forum_id: impl std::fmt::Display) -> String {
        format!("{}/forums/{forum_id}.json", self.public_url)
    }

    pub fn calendar_index_url(&self, token: &str) -> String {
        format!("{}/calendars/{token}.json", self.public_url)
    }

    pub fn calendar_document_url(&self, token: &str, thread_id: impl std::fmt::Display) -> String {
        format!("{}/calendars/{token}/events/{thread_id}.evnt.json", self.public_url)
    }

    pub fn config_page_url(&self, token: &str) -> String {
        format!("{}/config/{token}", self.public_url)
    }

    pub fn edit_link_url(&self, token: &str) -> String {
        format!("{}/edit/{token}", self.public_url)
    }

    pub fn edit_return_url(&self) -> String {
        format!("{}/edit/return", self.public_url)
    }

    pub fn index_view_url(&self, index_url: &str) -> String {
        with_query(&self.index_viewer_url, &[("url", index_url)])
    }

    pub fn event_view_url(&self, thread_id: impl std::fmt::Display) -> String {
        with_query(&self.viewer_url, &[("url", &self.event_document_url(thread_id))])
    }

    pub fn editor_url(&self, document: &str, token: &str) -> String {
        let return_url = self.edit_return_url();
        with_query(
            &self.editor_url,
            &[("data", document), ("return", &return_url), ("state", token)],
        )
    }
}

#[cfg(test)]
pub fn test_links() -> Links {
    Links {
        public_url: "https://p.test".to_owned(),
        editor_url: "https://eventsl.ink/edit".to_owned(),
        viewer_url: "https://eventsl.ink/event".to_owned(),
        index_viewer_url: "https://eventsl.ink/index".to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn query_values_are_form_encoded_like_url_search_params() {
        assert_eq!(
            with_query("https://eventsl.ink/edit", &[("data", r#"{"a":"b c/ą"}"#)]),
            "https://eventsl.ink/edit?data=%7B%22a%22%3A%22b+c%2F%C4%85%22%7D"
        );
        assert_eq!(
            with_query("https://x.test/v?url=old&keep=1", &[("url", "new")]),
            "https://x.test/v?keep=1&url=new"
        );
    }
}
