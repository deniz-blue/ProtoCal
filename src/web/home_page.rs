use std::sync::Arc;

use axum::{extract::State, response::Response};
use twilight_model::{
    guild::Permissions,
    id::{Id, marker::ApplicationMarker},
};

use super::{
    html_answer,
    page::{escaped, page},
};
use crate::bot::Bot;

const BOT_PERMISSIONS: Permissions = Permissions::VIEW_CHANNEL
    .union(Permissions::SEND_MESSAGES)
    .union(Permissions::SEND_MESSAGES_IN_THREADS)
    .union(Permissions::READ_MESSAGE_HISTORY)
    .union(Permissions::ADD_REACTIONS)
    .union(Permissions::PIN_MESSAGES)
    .union(Permissions::MANAGE_ROLES)
    .union(Permissions::MANAGE_THREADS);

const EXTERNAL_LINK_ICON: &str = r#"<svg class="external" viewBox="0 0 16 16" aria-hidden="true"><path d="M6 3H3v10h10v-3M9 2h5v5M14 2 7 9" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round"/></svg>"#;

pub fn invite_url(application_id: Id<ApplicationMarker>) -> String {
    format!(
        "https://discord.com/oauth2/authorize?client_id={application_id}&scope=bot+applications.commands&permissions={}",
        BOT_PERMISSIONS.bits()
    )
}

pub fn home_page(application_id: Id<ApplicationMarker>) -> String {
    let invite = escaped(&invite_url(application_id));
    let icon = EXTERNAL_LINK_ICON;
    let body = format!(
        r#"<main class="home">
	<h1>ProtoCal</h1>
	<p class="lede">A Discord bot that turns forum posts into events.</p>
	<a class="button" href="{invite}">Add to a server</a>
	<h2>Commands</h2>
	<div class="surface">
		<p class="row"><span>/event new<span class="note">A form asks for name, start, end, place and description.</span></span></p>
		<p class="row"><span>/event track<span class="note">Run inside the post, by its poster or staff. Only staff in a forum with review.</span></span></p>
		<p class="row"><span>/config<span class="note">Manage Server only. Sets event forums, announce role and review channel.</span></span></p>
		<p class="row"><span>/mycalendar<span class="note">Private. Opens in <a href="https://eventsl.ink/">eventsl.ink{icon}</a>.</span></span></p>
	</div>
	<h2>In a server</h2>
	<div class="surface">
		<p class="row"><span>New posts<span class="note">A new post in an event forum with a default reaction becomes an event by itself.</span></span></p>
		<p class="row"><span>Event role<span class="note">Mentionable. Called Event: followed by the name. Deleted when the event ends.</span></span></p>
		<p class="row"><span>Attend reaction<span class="note">The forum's default reaction on the first message gives the role. Taking the reaction back takes it away.</span></span></p>
		<p class="row"><span>Pins<span class="note">The starter is pinned. Messages that ping the event role get pinned too.</span></span></p>
		<p class="row"><span>Announce<span class="note">A forum can ping one role when an event is posted.</span></span></p>
		<p class="row"><span>Changes<span class="note">A new status, time or place pings the role, and an old time or place is struck out. Other edits post a quiet note.</span></span></p>
		<p class="row"><span>Review<span class="note">A forum can send events to a review channel first. Moderators post or decline. The host can withdraw.</span></span></p>
		<p class="row"><span>Edits<span class="note">The organiser or staff change it with the Edit button on the post, in Discord or in the web editor.</span></span></p>
		<p class="row"><span>Ends<span class="note">Locking or deleting the post ends the event. So does a day after its last date.</span></span></p>
		<p class="row"><span>Public<span class="note">In a forum everyone can see, each post gets a View button, and the forum gets an Open Evnt index of its events.</span></span></p>
	</div>
</main>
<footer><a href="https://github.com/deniz-blue/ProtoCal">Source{icon}</a></footer>"#
    );
    page("ProtoCal", &body)
}

pub async fn show_home_page(State(bot): State<Arc<Bot>>) -> Response {
    html_answer(home_page(bot.application_id))
}
