pub mod fetch;
#[path = "404.rs"]
pub mod not_found;
pub mod paste;

use askama::Template;

#[derive(Template)]
#[template(path = "index.html")]
struct IndexTemplate;

#[derive(Template)]
#[template(path = "paste.html")]
struct PasteTemplate<'a> {
    id: &'a str,
    content: &'a str,
}

#[derive(Template)]
#[template(path = "404.html")]
struct NotFoundTemplate;
