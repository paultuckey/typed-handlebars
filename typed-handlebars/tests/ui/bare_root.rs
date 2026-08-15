// `{{@root.title}}` names the top-level context, but `{{@root}}` on its own names the whole of it.
// handlebars.js writes `[object Object]` for that; there is no useful Rust equivalent, and picking
// one field or another would be a guess, so it is refused by name.
mod template {
    typed_handlebars::str!("page", r#"<h1>{{@root}}</h1>"#);
}

fn main() {}
