// The template calls it `heading`; the call site still says `title`. Under the old positional API
// this compiled and silently kept working.
mod template {
    typed_handlebars::str!("page", r#"<h1>{{heading}}</h1>"#);
}

fn main() {
    let _ = template::page::builder().title("Dub").render();
}
