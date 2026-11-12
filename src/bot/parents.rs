use std::{
    collections::HashMap,
    sync::RwLock,
    time::{Duration, Instant},
};

use serde_json::Value;

use super::Bot;
use crate::{
    event::parent::{ParentLink, PartOf, parent_url_of},
    open_evnt::remote::{FetchProblem, fetch_event_document},
};

const FRESH_FOR: Duration = Duration::from_secs(15 * 60);

#[derive(Default)]
pub struct ParentDocuments(RwLock<HashMap<String, (Instant, Option<Value>)>>);

impl ParentDocuments {
    fn fresh(&self, url: &str) -> Option<Option<Value>> {
        let known = self.0.read().expect("parent documents lock poisoned");
        let (fetched_at, document) = known.get(url)?;
        (fetched_at.elapsed() < FRESH_FOR).then(|| document.clone())
    }

    fn remember(&self, url: &str, fetched: Option<Value>) -> Option<Value> {
        let mut known = self.0.write().expect("parent documents lock poisoned");
        let last_good = known.remove(url).and_then(|(_, document)| document);
        let kept = fetched.or(last_good);
        known.insert(url.to_owned(), (Instant::now(), kept.clone()));
        kept
    }
}

impl Bot {
    async fn fetch_parent(&self, url: &str) -> Result<Value, FetchProblem> {
        let fetched = fetch_event_document(url).await;
        self.parent_documents.remember(url, fetched.as_ref().ok().cloned());
        fetched
    }

    async fn outside_parent(&self, url: &str) -> Option<Value> {
        match self.parent_documents.fresh(url) {
            Some(known) => known,
            None => {
                let fetched = fetch_event_document(url).await.ok();
                self.parent_documents.remember(url, fetched)
            }
        }
    }

    pub async fn part_of(&self, document: &Value) -> Option<PartOf> {
        let url = parent_url_of(document)?;
        if let Some(thread_id) = self.links.thread_of_event_document_url(&url)
            && let Ok(Some(parent)) = self.db.event_of_thread(thread_id)
        {
            return Some(PartOf::of_parent(ParentLink::Thread(thread_id), Some(&parent.document)));
        }
        let parent = self.outside_parent(&url).await;
        let link = ParentLink::outside(self.links.document_view_url(&url), parent.as_ref());
        Some(PartOf::of_parent(link, parent.as_ref()))
    }

    pub async fn problem_with_new_parent(&self, base: &Value, document: &Value) -> Option<String> {
        let url = parent_url_of(document)?;
        if parent_url_of(base).as_deref() == Some(url.as_str()) || self.links.thread_of_event_document_url(&url).is_some() {
            return None;
        }
        let problem = self.fetch_parent(&url).await.err()?;
        Some(format!("Part of: `{url}` {problem}."))
    }
}
