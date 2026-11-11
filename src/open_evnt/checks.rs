use std::{collections::HashMap, sync::LazyLock};

use regex::Regex;
use serde_json::Value;

use super::partial_date::PartialDate;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PathSegment {
    Key(String),
    Index(usize),
}

pub type Path = Vec<PathSegment>;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Issue {
    pub message: String,
    pub path: Path,
}

pub type Issues = Vec<Issue>;

pub type Check<'a> = &'a dyn Fn(&Value, &Path, &mut Issues);

pub static COMPONENT_TYPE_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^[A-Za-z0-9_-]+(\.[A-Za-z0-9_-]+)+#(0|[1-9]\d*)$").expect("component type")
});
pub static LANGUAGE_TAG_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^[A-Za-z]{1,8}(-[A-Za-z0-9]{1,8})*$").expect("language tag pattern")
});
static URI_SCHEME_PATTERN: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^[A-Za-z][A-Za-z0-9+.-]*:").expect("uri scheme pattern"));

pub fn key(path: &Path, key: &str) -> Path {
    let mut longer = path.clone();
    longer.push(PathSegment::Key(key.to_owned()));
    longer
}

pub fn index(path: &Path, index: usize) -> Path {
    let mut longer = path.clone();
    longer.push(PathSegment::Index(index));
    longer
}

pub fn report(issues: &mut Issues, message: impl Into<String>, path: Path) {
    issues.push(Issue {
        message: message.into(),
        path,
    });
}

pub fn string(value: &Value, path: &Path, issues: &mut Issues) {
    if !value.is_string() {
        report(issues, "must be a string", path.clone());
    }
}

pub fn boolean(value: &Value, path: &Path, issues: &mut Issues) {
    if !value.is_boolean() {
        report(issues, "must be true or false", path.clone());
    }
}

pub fn number_where(
    is_allowed: impl Fn(f64) -> bool,
    description: &str,
) -> impl Fn(&Value, &Path, &mut Issues) {
    move |value, path, issues| {
        if !value.as_f64().is_some_and(&is_allowed) {
            report(issues, format!("must be {description}"), path.clone());
        }
    }
}

pub fn matching<'a>(pattern: &'a Regex, description: &'a str) -> impl Fn(&Value, &Path, &mut Issues) {
    move |value, path, issues| {
        if !value.as_str().is_some_and(|text| pattern.is_match(text)) {
            report(issues, format!("must be {description}"), path.clone());
        }
    }
}

pub fn one_of<'a>(allowed: &'a [&'a str]) -> impl Fn(&Value, &Path, &mut Issues) {
    move |value, path, issues| {
        if !value.as_str().is_some_and(|text| allowed.contains(&text)) {
            let quoted: Vec<String> = allowed.iter().map(|text| format!("\"{text}\"")).collect();
            report(issues, format!("must be {}", quoted.join(" or ")), path.clone());
        }
    }
}

pub fn array_of(item: Check, non_empty: bool, value: &Value, path: &Path, issues: &mut Issues) {
    let Some(items) = value.as_array() else {
        return report(issues, "must be an array", path.clone());
    };
    if non_empty && items.is_empty() {
        report(issues, "must have at least one item", path.clone());
    }
    for (position, element) in items.iter().enumerate() {
        item(element, &index(path, position), issues);
    }
}

#[derive(Default)]
pub struct ObjectShape<'a> {
    pub required: &'a [(&'a str, Check<'a>)],
    pub optional: &'a [(&'a str, Check<'a>)],
    pub closed: bool,
    pub non_empty: bool,
}

pub fn object(shape: &ObjectShape, value: &Value, path: &Path, issues: &mut Issues) {
    let Some(fields) = value.as_object() else {
        return report(issues, "must be an object", path.clone());
    };
    for (name, check) in shape.required {
        match fields.get(*name) {
            Some(field) => check(field, &key(path, name), issues),
            None => report(issues, "is required", key(path, name)),
        }
    }
    for (name, check) in shape.optional {
        if let Some(field) = fields.get(*name) {
            check(field, &key(path, name), issues);
        }
    }
    let is_known = |name: &str| {
        shape.required.iter().chain(shape.optional).any(|(known, _)| *known == name)
    };
    if shape.closed {
        for name in fields.keys().filter(|name| !is_known(name)) {
            report(issues, "is not a field of this object", key(path, name));
        }
    }
    if shape.non_empty && fields.is_empty() {
        report(issues, "must have at least one field", path.clone());
    }
}

pub fn translations(value: &Value, path: &Path, issues: &mut Issues) {
    let Some(texts) = value.as_object() else {
        return report(issues, "must be an object of language tags to text", path.clone());
    };
    if texts.is_empty() {
        report(issues, "must have text in at least one language", path.clone());
    }
    let mut seen: HashMap<String, &str> = HashMap::new();
    for (tag, text) in texts {
        if !LANGUAGE_TAG_PATTERN.is_match(tag) {
            report(
                issues,
                "must be a BCP 47 language tag such as en or zh-Hant",
                key(path, tag),
            );
        }
        match seen.get(&tag.to_lowercase()) {
            Some(earlier) => report(issues, format!("differs from {earlier} only in case"), key(path, tag)),
            None => {
                seen.insert(tag.to_lowercase(), tag);
            }
        }
        if text.as_str().is_none_or(str::is_empty) {
            report(issues, "must be text that is not empty", key(path, tag));
        }
    }
}

pub fn absolute_uri(value: &Value, path: &Path, issues: &mut Issues) {
    let is_absolute = value
        .as_str()
        .is_some_and(|text| URI_SCHEME_PATTERN.is_match(text) && url::Url::parse(text).is_ok());
    if !is_absolute {
        report(
            issues,
            "must be an absolute URI such as https://example.com/events/1",
            path.clone(),
        );
    }
}

pub fn partial_date(value: &Value, path: &Path, issues: &mut Issues) {
    if !value.as_str().is_some_and(PartialDate::is_valid) {
        report(
            issues,
            "must be a PartialDate such as 2026-06-15T09:00[Europe/Vilnius]",
            path.clone(),
        );
    }
}
