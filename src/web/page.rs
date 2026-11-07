use super::font::LEXEND_PATH;

const STYLE: &str = include_str!("page.css");

pub fn escaped(text: &str) -> String {
    text.chars()
        .fold(String::with_capacity(text.len()), |mut escaped, character| {
            match character {
                '&' => escaped.push_str("&amp;"),
                '<' => escaped.push_str("&lt;"),
                '>' => escaped.push_str("&gt;"),
                '"' => escaped.push_str("&quot;"),
                '\'' => escaped.push_str("&#39;"),
                other => escaped.push(other),
            }
            escaped
        })
}

pub fn page(title: &str, body: &str) -> String {
    format!(
        "<!doctype html>
<html lang=\"en\">
<meta charset=\"utf-8\" />
<meta name=\"viewport\" content=\"width=device-width, initial-scale=1, viewport-fit=cover\" />
<meta name=\"referrer\" content=\"no-referrer\" />
<title>{}</title>
<link rel=\"preload\" href=\"{LEXEND_PATH}\" as=\"font\" type=\"font/woff2\" crossorigin />
<style>
@font-face {{
	font-family: Lexend;
	src: url(\"{LEXEND_PATH}\") format(\"woff2\");
	font-weight: 100 900;
	font-display: block;
}}
{STYLE}</style>
{body}
</html>
",
        escaped(title)
    )
}
