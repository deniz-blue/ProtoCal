const STYLE: &str = r#"			:root {
				color-scheme: dark;
				accent-color: #b48cff;
			}
			body {
				background: #000;
				color: #ddd;
				font:
					17px/1.5 system-ui,
					sans-serif;
				margin: 0;
				padding: 2rem 1.5rem;
			}
			a {
				color: #b48cff;
			}
			h1 {
				font-size: 1.4rem;
				margin: 0 0 1.5rem;
			}
			h2 {
				font-size: 1.1rem;
				margin: 1.5rem 0 0.25rem;
			}
			.hint {
				color: #888;
				font-size: 0.85em;
			}
			table {
				border-collapse: collapse;
				margin: 0.75rem 0 1.25rem;
			}
			th {
				color: #888;
				font-weight: normal;
				text-align: left;
			}
			th,
			td {
				padding: 0.35rem 1.25rem 0.35rem 0;
				vertical-align: top;
			}
			select,
			button {
				font: inherit;
				color: inherit;
				background: #111;
				border: 1px solid #333;
				border-radius: 6px;
				padding: 0.3rem 0.5rem;
			}
			button {
				background: #b48cff;
				color: #000;
				border: 0;
				padding: 0.4rem 1.2rem;
				cursor: pointer;
			}"#;

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
        "<!doctype html>\n\t<html lang=\"en\">\n\t\t<meta charset=\"utf-8\" />\n\t\t<meta name=\"viewport\" content=\"width=device-width, initial-scale=1\" />\n\t\t<meta name=\"referrer\" content=\"no-referrer\" />\n\t\t<title>{}</title>\n\t\t<style>\n{STYLE}\n\t\t</style>\n\t\t{body}\n\t</html> ",
        escaped(title)
    )
}
