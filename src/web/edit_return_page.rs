use super::page::page;

const BODY: &str = r#"<p id="status">Saving…</p>
<p><a id="back" hidden>Back to the post</a></p>
<script>
	const fragment = new URLSearchParams(location.hash.slice(1));
	history.replaceState(null, "", location.pathname);
	const status = document.getElementById("status");
	const back = document.getElementById("back");
	fetch(location.pathname, {
		method: "POST",
		headers: { "content-type": "application/json" },
		body: JSON.stringify({
			state: fragment.get("state"),
			cancelled: fragment.get("cancelled") === "true",
			event: fragment.get("event"),
		}),
	})
		.then((response) => response.json())
		.then((answer) => {
			status.textContent = answer.message;
			if (answer.threadUrl) {
				back.href = answer.threadUrl;
				back.hidden = false;
			}
		})
		.catch(() => {
			status.textContent = "ProtoCal could not be reached, so nothing was saved.";
		});
</script>"#;

pub fn edit_return_page() -> String {
    page("ProtoCal", BODY)
}
