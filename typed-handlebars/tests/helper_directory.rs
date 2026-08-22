//! The frame reaching templates through a directory, which `str!` cannot cover.
//!
//! A template names the frame as `super::Frame`, so every directory between it and the module
//! `register_helper!` was invoked in has to pass the name along. This pins that it does — at the
//! root, one level down, and two — and that a partial's helper call makes its includer take the
//! frame as well.

pub struct Ctx;

impl Ctx {
    pub fn t(&self, key: &str) -> String {
        format!("[{}]", key)
    }
}

mod templates {
    typed_handlebars::register_helper!(super::Ctx);
    typed_handlebars::directory!("tests/helper-templates/");
}

/// A template beside the `register_helper!` line reaches the frame directly.
#[test]
fn a_template_at_the_root_takes_the_frame() {
    assert_eq!(
        templates::row::Vars { name: "save" }.render(&Ctx),
        "<li>[save]</li>\n"
    );
}

/// Two directories deep, where `super::Frame` only resolves because each level re-imported it.
#[test]
fn a_template_two_directories_down_takes_the_frame() {
    assert_eq!(
        templates::deep::nested::leaf::Vars.render(&Ctx),
        "<span>[leaf]</span>\n"
    );
}

/// A partial is spliced into whoever includes it, so its helper call is the includer's too — and
/// the includer asks for the frame even though its own text never calls one directly.
#[test]
fn a_partial_helper_makes_its_includer_take_the_frame() {
    assert_eq!(
        templates::page::Vars {
            rows: vec![
                templates::page::RowsItem { name: "one" },
                templates::page::RowsItem { name: "two" },
            ],
        }
        .render(&Ctx),
        "<h1>[title]</h1><ul><li>[one]</li>\n<li>[two]</li>\n</ul>\n"
    );
}

/// A template in the same directory that calls no helper is untouched.
#[test]
fn a_template_calling_no_helper_renders_without_a_frame() {
    assert_eq!(
        templates::plain::Vars { name: "King" }.render(),
        "<p>King</p>\n"
    );
}
