pub use dry_handlebars_macros::dry_handlebars_directory as directory;
pub use dry_handlebars_macros::dry_handlebars_file as file;
pub use dry_handlebars_macros::dry_handlebars_str as str;

#[cfg(test)]
mod tests {

    #[test]
    fn basic_usage() {
        mod template {
            crate::str!("test", r#"<p>{{firstname}} {{lastname}}</p>"#);
        }
        assert_eq!(
            template::test("King", "Tubby").render(),
            "<p>King Tubby</p>"
        );
    }

    struct Person {
        firstname: String,
        lastname: String,
    }

    #[test]
    fn path_expressions() {
        mod template {
            crate::str!(
                "test",
                //language=handlebars
                r#"{{person.firstname}} {{person.lastname}}"#,
                ("person", super::Person)
            );
        }
        let person = Person {
            firstname: "King".to_string(),
            lastname: "Tubby".to_string(),
        };
        assert_eq!(template::test(person).render(), "King Tubby");
    }

    struct Author {
        first_name: String,
        last_name: String,
    }

    #[test]
    fn if_helper() {
        mod template {
            crate::str!(
                "test",
                //language=handlebars
                r#"<div>{{#if has_author}}<h1>{{first_name}} {{last_name}}</h1>{{/if}}</div>"#
            );
        }
        assert_eq!(
            template::test(true, "King", "Tubby").render(),
            //language=html
            "<div><h1>King Tubby</h1></div>"
        );
        assert_eq!(
            template::test(false, "King", "Tubby").render(),
            //language=html
            "<div></div>"
        );
    }

    #[test]
    fn unless_helper() {
        mod template {
            crate::str!(
                "test",
                //language=handlebars
                r#"<div>{{#unless has_author}}<h1>Unknown</h1>{{/unless}}</div>"#
            );
        }
        assert_eq!(
            template::test(false).render(),
            //language=html
            "<div><h1>Unknown</h1></div>"
        );
        assert_eq!(
            template::test(true).render(),
            //language=html
            "<div></div>"
        );
    }

    #[test]
    fn if_else_helper() {
        mod template {
            crate::str!(
                "test",
                //language=handlebars
                r#"<div>{{#if has_author}}<h1>{{first_name}}</h1>{{else}}<h1>Unknown</h1>{{/if}}</div>"#,
                ("author", Option<super::Author>)
            );
        }
        assert_eq!(
            template::test(true, "King").render(),
            //language=html
            r#"<div><h1>King</h1></div>"#
        );
        assert_eq!(
            template::test(false, "King").render(),
            //language=html
            r#"<div><h1>Unknown</h1></div>"#
        );
    }

    #[test]
    fn with_helper_option() {
        mod template {
            crate::str!(
                "test",
                //language=handlebars
                r#"<div>{{#with author}}<h1>{{first_name}} {{last_name}}</h1>{{/with}}</div>"#,
                ("author", Option<super::Author>)
            );
        }
        let author = Author {
            first_name: "King".to_string(),
            last_name: "Tubby".to_string(),
        };
        assert_eq!(
            template::test(Some(author)).render(),
            //language=html
            "<div><h1>King Tubby</h1></div>"
        );
        assert_eq!(
            template::test(None).render(),
            //language=html
            "<div></div>"
        );
    }

    #[test]
    fn with_helper() {
        mod template {
            crate::str!(
                "test",
                //language=handlebars
                r#"<div>{{#with author}}<h1>{{first_name}} {{last_name}}</h1>{{/with}}</div>"#,
                ("author", super::Author)
            );
        }
        let author = Author {
            first_name: "King".to_string(),
            last_name: "Tubby".to_string(),
        };
        assert_eq!(
            template::test(author).render(),
            //language=html
            "<div><h1>King Tubby</h1></div>"
        );
    }

    #[test]
    fn for_helper() {
        mod template {
            crate::str!(
                "test",
                //language=handlebars
                r#"<div>{{#each authors}}<p>Hello {{first_name}}</p>{{/each}}</div>"#,
                ("authors", Vec<super::Author>)
            );
        }
        let author = Author {
            first_name: "King".to_string(),
            last_name: "Tubby".to_string(),
        };
        assert_eq!(
            template::test(vec![author]).render(),
            //language=html
            "<div><p>Hello King</p></div>"
        );
    }

    /// The template says `rows` is a list whose items have a `name`, so the macro generates the
    /// item type. Nothing here declares a type or implements a trait.
    #[test]
    fn each_generates_the_item_type() {
        mod template {
            crate::str!(
                "test",
                //language=handlebars
                r#"<ul>{{#each rows}}<li>{{name}} {{email}}</li>{{/each}}</ul>"#
            );
        }
        assert_eq!(
            template::test(vec![
                template::test_rows_item::new("King", "king@example.com"),
                template::test_rows_item::new("Tubby", "tubby@example.com"),
            ])
            .render(),
            //language=html
            "<ul><li>King king@example.com</li><li>Tubby tubby@example.com</li></ul>"
        );
    }

    /// An `{{#each}}` whose body only writes `{{this}}` iterates values, not records, so no item
    /// struct is generated.
    #[test]
    fn each_over_plain_values_needs_no_item_type() {
        mod template {
            crate::str!("test", r#"{{#each tags}}[{{this}}]{{/each}}"#);
        }
        assert_eq!(template::test(vec!["a", "b"]).render(), "[a][b]");
        assert_eq!(template::test([1, 2, 3]).render(), "[1][2][3]");
    }

    #[test]
    fn nested_each_generates_nested_item_types() {
        mod template {
            crate::str!(
                "test",
                //language=handlebars
                r#"{{#each rows}}<tr>{{#each cells}}<td>{{value}}</td>{{/each}}</tr>{{/each}}"#
            );
        }
        let rows = vec![
            template::test_rows_item::new(vec![
                template::test_rows_item_cells_item::new(1),
                template::test_rows_item_cells_item::new(2),
            ]),
            template::test_rows_item::new(vec![template::test_rows_item_cells_item::new(3)]),
        ];
        assert_eq!(
            template::test(rows).render(),
            //language=html
            "<tr><td>1</td><td>2</td></tr><tr><td>3</td></tr>"
        );
    }

    /// Without a declared type, `{{ person.name }}` generates the record it implies.
    #[test]
    fn dotted_paths_generate_a_record_type() {
        mod template {
            crate::str!("test", r#"{{person.firstname}} {{person.lastname}}"#);
        }
        assert_eq!(
            template::test(template::test_person::new("King", "Tubby")).render(),
            "King Tubby"
        );
    }

    #[test]
    fn each_can_reach_the_enclosing_scope() {
        mod template {
            crate::str!(
                "test",
                //language=handlebars
                r#"{{#each rows}}<li>{{name}} of {{../company}}</li>{{/each}}"#
            );
        }
        assert_eq!(
            template::test(vec![template::test_rows_item::new("King")], "Studio One").render(),
            //language=html
            "<li>King of Studio One</li>"
        );
    }

    #[test]
    fn each_accepts_a_named_item() {
        mod template {
            crate::str!(
                "test",
                //language=handlebars
                r#"{{#each rows as |row|}}<li>{{row.name}}</li>{{/each}}"#
            );
        }
        assert_eq!(
            template::test(vec![template::test_rows_item::new("King")]).render(),
            //language=html
            "<li>King</li>"
        );
    }

    /// Rendering borrows, so a caller can pass a list they still own — no clone, no giving up the
    /// data.
    #[test]
    fn each_accepts_borrowed_lists() {
        mod template {
            crate::str!(
                "test",
                //language=handlebars
                r#"{{#each rows}}<li>{{name}}</li>{{/each}}"#
            );
        }
        let rows = vec![
            template::test_rows_item::new("King"),
            template::test_rows_item::new("Tubby"),
        ];
        let expected = "<li>King</li><li>Tubby</li>";

        assert_eq!(template::test(&rows).render(), expected);
        assert_eq!(template::test(rows.as_slice()).render(), expected);

        // The caller still owns it, and can hand it over afterwards if they want to.
        assert_eq!(rows.len(), 2);
        assert_eq!(template::test(rows).render(), expected);

        let array = [template::test_rows_item::new("King")];
        assert_eq!(template::test(&array).render(), "<li>King</li>");
        assert_eq!(template::test(array).render(), "<li>King</li>");
    }

    /// A declared type keeps `IntoIterator`, so the escape hatch still covers containers that
    /// aren't slice-backed.
    #[test]
    fn a_declared_list_type_need_not_be_slice_backed() {
        use std::collections::VecDeque;

        mod template {
            crate::str!(
                "test",
                //language=handlebars
                r#"{{#each authors}}<p>{{first_name}}</p>{{/each}}"#,
                ("authors", std::collections::VecDeque<super::Author>)
            );
        }
        let mut authors = VecDeque::new();
        authors.push_back(Author {
            first_name: "King".to_string(),
            last_name: "Tubby".to_string(),
        });
        assert_eq!(template::test(authors).render(), "<p>King</p>");
    }

    /// Rendering borrows rather than consumes, so a template may walk the same list twice.
    #[test]
    fn the_same_list_can_be_iterated_twice() {
        mod template {
            crate::str!(
                "test",
                //language=handlebars
                r#"{{#each rows}}{{name}}{{/each}}|{{#each rows}}{{name}}{{/each}}"#
            );
        }
        let page = template::test(vec![
            template::test_rows_item::new("a"),
            template::test_rows_item::new("b"),
        ]);
        assert_eq!(page.render(), "ab|ab");
        // …and the value is still usable afterwards.
        assert_eq!(page.render(), "ab|ab");
    }

    /// A type declared in Rust wins over the generated one, so existing domain types wire straight
    /// in — the fields just have to line up with what the template asks for.
    #[test]
    fn a_declared_type_replaces_the_generated_one() {
        mod template {
            crate::str!(
                "test",
                //language=handlebars
                r#"{{#each authors}}<p>{{first_name}}</p>{{/each}}"#,
                ("authors", Vec<super::Author>)
            );
        }
        assert_eq!(
            template::test(vec![Author {
                first_name: "King".to_string(),
                last_name: "Tubby".to_string(),
            }])
            .render(),
            //language=html
            "<p>King</p>"
        );
    }

    #[test]
    fn test_comment() {
        mod template {
            crate::str!(
                "test",
                //language=handlebars
                r#"Note: {{! This is a comment }} and {{!-- {{so is this}} --}}\\{{{{}}"#,
            );
        }
        assert_eq!(template::test().render(), "Note:  and \\{{");
    }

    #[test]
    fn test_trimming() {
        mod template {
            crate::str!(
                "test",
                //language=handlebars
                r#"  {{~#if some ~}}   Hello{{~/if~}}"#,
            );
        }
        assert_eq!(template::test(true).render(), "Hello");
    }

    ///
    ///
    ///
    ///
    ///
    ///
    ///
    ///
    ///
    ///

    #[test]
    fn it_works() {
        mod template {
            crate::str!("test", "Hello {{{name}}}!");
        }
        assert_eq!(template::test("King").render(), "Hello King!");
    }

    #[test]
    fn test_escaped() {
        mod template {
            crate::str!(
                "test",
                "{{{{skip}}}}wang doodle {{{{/dandy}}}}{{{{/skip}}}}"
            );
        }
        assert_eq!(template::test().render(), "wang doodle {{{{/dandy}}}}");
    }

    #[test]
    fn test_format_number() {
        mod template {
            crate::str!("test", "Price: ${{format \"{:.2}\" price}}");
        }
        assert_eq!(template::test(12.2345f64).render(), "Price: $12.23");
    }

    // #[test]
    // fn test_nesting() {
    //     let rust = compile("{{#if some}}{{#each some}}Hello {{this}}{{/each}}{{/if}}");
    //     assert_eq!(
    //         rust,
    //         "if self.some.as_bool(){for this_2 in self.some{write!(f, \"Hello {}\", this_2.as_display_html())?;}}"
    //     );
    // }
    //
    // #[test]
    // fn test_as() {
    //     let rust = compile(
    //         "{{#if some}}{{#each some as thing}}Hello {{thing}} {{thing.name}}{{/each}}{{/if}}",
    //     );
    //     assert_eq!(
    //         rust,
    //         "if self.some.as_bool(){for thing_2 in self.some{write!(f, \"Hello {} {}\", thing_2.as_display_html(), thing_2.name.as_display_html())?;}}"
    //     );
    // }
    //
    // #[test]
    // fn test_scoping() {
    //     let rust = compile(
    //         "{{#with some}}{{#with other}}Hello {{name}} {{../company}} {{/with}}{{/with}}",
    //     );
    //     assert_eq!(
    //         rust,
    //         "{let this_1 = self.some;{let this_2 = this_1.other;write!(f, \"Hello {} {} \", this_2.name.as_display_html(), this_1.company.as_display_html())?;}}"
    //     );
    // }
    //
    // #[test]
    // fn test_indexer() {
    //     let rust = compile(
    //         "{{#each things}}Hello{{{@index}}}{{#each things}}{{{lookup other @../index}}}{{{@index}}}{{/each}}{{/each}}",
    //     );
    //     assert_eq!(
    //         rust,
    //         "let mut i_1 = 0;for this_1 in self.things{write!(f, \"Hello{}\", i_1.as_display())?;let mut i_2 = 0;for this_2 in this_1.things{write!(f, \"{}{}\", this_2.other[i_1].as_display(), i_2.as_display())?;i_2+=1;}i_1+=1;}"
    //     );
    // }
    //
    // #[test]
    // fn test_map() {
    //     let rust = compile(
    //         "{{#each things}}Hello{{{@key}}}{{#each @value}}{{#if_some (try_lookup other @../key)}}{{{this}}}{{/if_some}}{{{@value}}}{{/each}}{{/each}}",
    //     );
    //     assert_eq!(
    //         rust,
    //         "for this_1 in self.things{write!(f, \"Hello{}\", this_1.0.as_display())?;for this_2 in this_1.1{if let Some(this_3) = this_2.other.get(this_1.0){write!(f, \"{}\", this_3.as_display())?;}write!(f, \"{}\", this_2.1.as_display())?;}}"
    //     );
    // }
    //
    //
    // #[test]
    // fn test_subexpression() {
    //     let rust = compile(
    //         "{{#each things}}{{#with (lookup ../other @index) as |other|}}{{{../name}}}: {{{other}}}{{/with}}{{/each}}",
    //     );
    //     assert_eq!(
    //         rust,
    //         "let mut i_1 = 0;for this_1 in self.things{{let other_2 = self.other[i_1];write!(f, \"{}: {}\", this_1.name.as_display(), other_2.as_display())?;}i_1+=1;}"
    //     );
    // }
    //
    // #[test]
    // fn test_selfless() {
    //     let rust = Compiler::new(Options{
    //         root_var_name: None,
    //         write_var_name: "f",
    //         variable_types: Default::default(),
    //     }, make_map()).compile("{{#each things}}{{#with (lookup ../other @index) as |other|}}{{{../name}}}: {{{other}}}{{/with}}{{/each}}").unwrap();
    //     assert_eq!(
    //         rust.uses("rusty_handlebars").to_string(),
    //         "use rusty_handlebars::AsDisplay"
    //     );
    //     assert_eq!(
    //         rust.code,
    //         "let mut i_1 = 0;for this_1 in things{{let other_2 = other[i_1];write!(f, \"{}: {}\", this_1.name.as_display(), other_2.as_display())?;}i_1+=1;}"
    //     );
    // }
    //
    // #[test]
    // fn javascript() {
    //     let rust = Compiler::new(opts(), make_map()).compile("<script>if (location.href.contains(\"localhost\")){ console.log(\"\\{{{{}}}}\") }</script>").unwrap();
    //     assert_eq!(rust.uses("rusty_handlebars").to_string(), "");
    //     assert_eq!(
    //         rust.code,
    //         "write!(f, \"<script>if (location.href.contains(\\\"localhost\\\")){{ console.log(\\\"{{{{}}}}\\\") }}</script>\")?;"
    //     );
    // }
}
