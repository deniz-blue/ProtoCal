use super::page::page;

const BODY: &str = r#"<main class="return">
	<p id="status"><span class="spinner" role="status" aria-label="Saving"></span></p>
	<a class="button" id="back">Back to the post</a>
</main>
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
			}
		})
		.catch(() => {
			status.textContent = "ProtoCal could not be reached, so nothing was saved.";
		});
</script>"#;

pub fn edit_return_page() -> String {
    page("ProtoCal", BODY)
}
