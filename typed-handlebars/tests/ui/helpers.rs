// A helper is a method on the frame type, so a name that is not one has to say so. What the
// template macro cannot check is whether the method exists — a proc macro sees tokens, not types —
// so these pin both halves: what the template rejects itself, and what the generated call reports.

mod reserved_name {
    pub struct Ctx;
    mod templates {
        typed_handlebars::register_helper!(super::Ctx);
        typed_handlebars::str!("page", "{{ lookup rows 1 }}");
    }
}

mod hash_argument {
    pub struct Ctx;
    mod templates {
        typed_handlebars::register_helper!(super::Ctx);
        typed_handlebars::str!("page", r#"{{ t "Hello {name}" name=user }}"#);
    }
}

mod helper_as_a_condition {
    pub struct Ctx;
    mod templates {
        typed_handlebars::register_helper!(super::Ctx);
        typed_handlebars::str!("page", r#"{{#if a}}1{{else if t "b"}}2{{/if}}"#);
    }
}

// The one the compiler catches rather than the template: the frame exists, the method does not.
mod no_such_method {
    pub struct Ctx;
    mod templates {
        typed_handlebars::register_helper!(super::Ctx);
        typed_handlebars::str!("page", r#"{{ t "Save" }}"#);
    }
}

fn main() {}
