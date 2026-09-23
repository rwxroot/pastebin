pub mod fetch;
pub mod icon;
pub mod lost;
pub mod paste;

use askama::Template;

#[derive(Template)]
#[template(path = "paste.html")]
struct PasteTemplate;

#[derive(Template)]
#[template(path = "fetch.html")]
struct FetchTemplate<'a> {
    id: &'a str,
    content: &'a str,
}

#[derive(Template)]
#[template(path = "lost.html")]
struct NotFoundTemplate;
