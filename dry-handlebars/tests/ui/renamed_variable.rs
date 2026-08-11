// The template calls it `heading`; the call site still says `title`. Under the old positional API
// this compiled and silently kept working.
mod template {
    dry_handlebars::str!("page", r#"<h1>{{heading}}</h1>"#);
}

fn main() {
    let _ = template::page_builder::new().title("Dub").render();
}
