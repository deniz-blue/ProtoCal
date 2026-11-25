use serde_json::json;
use twilight_model::{guild::Member, id::Id};

use super::{
    organizer_picture::{OrganizerPicture, avatar_url_of, thumbnail_shows},
    stacked_avatars::{StackFace, StackLayout},
};

fn faces_and_centers(organizer_count: usize) -> (f32, Vec<(StackFace, [f32; 2])>) {
    let layout = StackLayout::of_organizer_count(organizer_count);
    let circles = layout.circles.iter().map(|circle| (circle.face, circle.center.map(|at| (at * 100.0).round() / 100.0))).collect();
    (layout.diameter, circles)
}

#[test]
fn organizers_stack_as_a_pair_a_triangle_a_square_and_three_with_a_count() {
    use StackFace::{Avatar, HiddenCount};
    assert_eq!(faces_and_centers(2), (0.66, vec![(Avatar(0), [0.33, 0.33]), (Avatar(1), [0.67, 0.67])]));
    assert_eq!(faces_and_centers(3), (0.56, vec![(Avatar(0), [0.5, 0.28]), (Avatar(1), [0.28, 0.72]), (Avatar(2), [0.72, 0.72])]));
    let square = [[0.27, 0.27], [0.73, 0.27], [0.27, 0.73], [0.73, 0.73]];
    let with_faces = |faces: [StackFace; 4]| (0.54, faces.into_iter().zip(square).collect::<Vec<_>>());
    assert_eq!(faces_and_centers(4), with_faces([Avatar(0), Avatar(1), Avatar(2), Avatar(3)]));
    assert_eq!(faces_and_centers(5), with_faces([Avatar(0), Avatar(1), Avatar(2), HiddenCount(2)]));
    assert_eq!(faces_and_centers(6), with_faces([Avatar(0), Avatar(1), Avatar(2), HiddenCount(3)]));
}

fn member(nick: &str, avatar: Option<&str>) -> Member {
    serde_json::from_value(json!({
        "deaf": false,
        "mute": false,
        "flags": 0,
        "joined_at": null,
        "nick": nick,
        "roles": [],
        "avatar": avatar,
        "user": { "id": "258638629839175681", "username": "deniz.blue", "discriminator": "0", "avatar": "207dca9beba6db77cb3f15ea0e39148d" },
    }))
    .unwrap()
}

fn stack_file_name(second: &Member) -> Option<String> {
    let (guild, first, second_id) = (Id::new(1), Id::new(323_412_914_574_983_168), Id::new(258_638_629_839_175_681));
    OrganizerPicture { avatar_urls: vec![avatar_url_of(guild, first, None), avatar_url_of(guild, second_id, Some(second))] }.stack_file_name()
}

#[test]
fn the_picture_file_changes_with_an_avatar_and_not_with_a_nickname() {
    let named = stack_file_name(&member("deniz", None)).unwrap();
    assert!(named.starts_with("organizers-") && named.ends_with(".png"), "{named}");
    assert_eq!(stack_file_name(&member("deniz 🌸", None)).unwrap(), named);
    assert_ne!(stack_file_name(&member("deniz", Some("a_0123456789abcdef0123456789abcdef"))).unwrap(), named);
}

#[test]
fn an_avatar_is_the_server_one_then_the_user_one_then_the_default() {
    let (guild, user) = (Id::new(1), Id::new(258_638_629_839_175_681));
    let server = avatar_url_of(guild, user, Some(&member("deniz", Some("a_0123456789abcdef0123456789abcdef"))));
    assert_eq!(server, "https://cdn.discordapp.com/guilds/1/users/258638629839175681/avatars/a_0123456789abcdef0123456789abcdef.png?size=256");
    let own = avatar_url_of(guild, user, Some(&member("deniz", None)));
    assert_eq!(own, "https://cdn.discordapp.com/avatars/258638629839175681/207dca9beba6db77cb3f15ea0e39148d.png?size=256");
    assert_eq!(avatar_url_of(guild, user, None), "https://cdn.discordapp.com/embed/avatars/1.png");
}

#[test]
fn one_organizer_is_shown_by_avatar_url_and_more_by_attachment() {
    let one = OrganizerPicture { avatar_urls: vec!["https://cdn.discordapp.com/embed/avatars/1.png".to_owned()] };
    assert_eq!(one.thumbnail_url().as_deref(), Some("https://cdn.discordapp.com/embed/avatars/1.png"));
    assert_eq!(one.stack_file_name(), None);
    let two = OrganizerPicture { avatar_urls: vec![one.avatar_urls[0].clone(), one.avatar_urls[0].clone()] };
    let wanted = two.thumbnail_url().unwrap();
    assert_eq!(wanted, format!("attachment://{}", two.stack_file_name().unwrap()));
    let uploaded = format!("https://cdn.discordapp.com/attachments/1/2/{}?ex=1&hm=2", two.stack_file_name().unwrap());
    assert!(thumbnail_shows(&uploaded, &wanted));
    assert!(!thumbnail_shows("https://cdn.discordapp.com/attachments/1/2/organizers-000000000000.png", &wanted));
}
