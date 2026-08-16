import {describe, it} from 'node:test';

import Handlebars from 'handlebars';
import assert from "node:assert";

describe('Handlebars Reference Tests', () => {
    it('basic_usage', () => {
        const template = Handlebars.compile('<p>{{firstname}} {{lastname}}</p>');
        const result = template({ firstname: "King", lastname: "Tubby" });
        assert.strictEqual(result, '<p>King Tubby</p>');
    });

    it('path_expressions', () => {
        const template = Handlebars.compile('{{person.firstname}} {{person.lastname}}');
        const person = {
            firstname: "King",
            lastname: "Tubby",
        };
        const result = template({ person });
        assert.strictEqual(result, 'King Tubby');
    });

    it('if_helper', () => {
        const template = Handlebars.compile('<div>{{#if has_author}}<h1>{{first_name}} {{last_name}}</h1>{{/if}}</div>');

        const resultTrue = template({ has_author: true, first_name: "King", last_name: "Tubby" });
        assert.strictEqual(resultTrue, '<div><h1>King Tubby</h1></div>');

        const resultFalse = template({ has_author: false, first_name: "King", last_name: "Tubby" });
        assert.strictEqual(resultFalse, '<div></div>');
    });

    it('with_helper', () => {
        const template = Handlebars.compile('<div>{{#with author}}<h1>{{first_name}} {{last_name}}</h1>{{/with}}</div>');
        const author = {
            first_name: "King",
            last_name: "Tubby",
        };
        const resultTrue = template({ author });
        assert.strictEqual(resultTrue, '<div><h1>King Tubby</h1></div>');

        const resultFalse = template({ author: null });
        assert.strictEqual(resultFalse, '<div></div>');
    });

    it('with_else_helper', () => {
        const template = Handlebars.compile('<div>{{#with author}}<h1>{{first_name}}</h1>{{else}}<h1>Unknown</h1>{{/with}}</div>');

        const author = {
            first_name: "King",
            last_name: "Tubby",
        };

        const resultTrue = template({ author });
        assert.strictEqual(resultTrue, '<div><h1>King</h1></div>');

        const resultFalse = template({ author: null });
        assert.strictEqual(resultFalse, '<div><h1>Unknown</h1></div>');
    });

    // Mirrors `a_variable_can_be_tested_and_printed` in the Rust suite: testing a variable must not
    // stop you printing it.
    it('a_variable_can_be_tested_and_printed', () => {
        const template = Handlebars.compile('[{{#if title}}{{title}}{{/if}}]');
        assert.strictEqual(template({ title: "Dub" }), '[Dub]');
        assert.strictEqual(template({ title: "" }), '[]');
        assert.strictEqual(template({ title: 7 }), '[7]');
        assert.strictEqual(template({ title: 0 }), '[]');
        assert.strictEqual(template({ title: true }), '[true]');
        assert.strictEqual(template({ title: false }), '[]');
    });

    // Mirrors `falsiness_follows_handlebars` in the Rust suite. This is the definition the Rust
    // `Truthy` trait is written against.
    it('falsiness_follows_handlebars', () => {
        const template = Handlebars.compile('[{{#if value}}yes{{/if}}]');
        assert.strictEqual(template({}), '[]');
        assert.strictEqual(template({ value: false }), '[]');
        assert.strictEqual(template({ value: true }), '[yes]');
        assert.strictEqual(template({ value: "" }), '[]');
        assert.strictEqual(template({ value: "x" }), '[yes]');
        assert.strictEqual(template({ value: 0 }), '[]');
        assert.strictEqual(template({ value: -1 }), '[yes]');
        assert.strictEqual(template({ value: [] }), '[]');
        assert.strictEqual(template({ value: [1] }), '[yes]');
    });

    // Mirrors `absent.rs` in the Rust suite. This is the definition the Rust `Render` impl for
    // `Option` is written against: null and undefined write nothing at all, and `false` and `0`
    // are values that write themselves.
    it('null_and_undefined_write_nothing', () => {
        const template = Handlebars.compile('[{{ value }}]');
        assert.strictEqual(template({ value: null }), '[]');
        assert.strictEqual(template({ value: undefined }), '[]');
        assert.strictEqual(template({}), '[]');
        assert.strictEqual(template({ value: "" }), '[]');
        assert.strictEqual(template({ value: false }), '[false]');
        assert.strictEqual(template({ value: 0 }), '[0]');
        assert.strictEqual(template({ value: "Dub" }), '[Dub]');
    });

    // The same, unescaped, and one level down in a record.
    it('null_writes_nothing_raw_or_nested', () => {
        const raw = Handlebars.compile('[{{{ value }}}]');
        assert.strictEqual(raw({ value: null }), '[]');
        const nested = Handlebars.compile('[{{ person.nickname }}]');
        assert.strictEqual(nested({ person: { nickname: null } }), '[]');
    });

    // A nullable column across a loop — the case the Rust side accepts an `Option` for.
    it('null_writes_nothing_inside_each', () => {
        const rows = Handlebars.compile('{{#each rows}}<td>{{ when }}</td>{{/each}}');
        assert.strictEqual(rows({ rows: [{ when: "now" }, { when: null }] }), '<td>now</td><td></td>');
        const items = Handlebars.compile('{{#each tags}}[{{this}}]{{/each}}');
        assert.strictEqual(items({ tags: ["a", null, "c"] }), '[a][][c]');
    });

    // Mirrors `length.rs` in the Rust suite. `length` is an ordinary property lookup in
    // handlebars.js that happens to land on the one JS arrays carry — which is why a designer
    // writes it without thinking, and why the Rust side has to support it.
    it('a_list_reports_how_many_items_it_holds', () => {
        const template = Handlebars.compile('[{{ rows.length }}]');
        assert.strictEqual(template({ rows: [1, 2, 3] }), '[3]');
        assert.strictEqual(template({ rows: [] }), '[0]');
    });

    // The distinction the Rust `Absent` type exists for: undefined counts as nothing, where an
    // empty list counts 0.
    it('an_unset_list_counts_as_nothing_rather_than_zero', () => {
        const template = Handlebars.compile('[{{ rows.length }}]');
        assert.strictEqual(template({}), '[]');
        assert.strictEqual(template({ rows: undefined }), '[]');
        assert.strictEqual(template({ rows: [] }), '[0]');
    });

    it('a_count_can_be_tested', () => {
        const template = Handlebars.compile('{{#if rows.length}}some{{else}}none{{/if}}');
        assert.strictEqual(template({ rows: [1] }), 'some');
        assert.strictEqual(template({ rows: [] }), 'none');
        assert.strictEqual(template({}), 'none');
    });

    it('a_list_can_be_counted_and_iterated', () => {
        const template = Handlebars.compile('{{ rows.length }}:{{#each rows}}{{ name }}{{/each}}');
        assert.strictEqual(template({ rows: [{ name: "King" }, { name: "Tubby" }] }), '2:KingTubby');
    });

    // Mirrors `root.rs` in the Rust suite. `@root` is absolute where `@index` and friends are loop
    // state, which is why the Rust side resolves it before the outward walk rather than through it.
    it('the_top_level_is_reachable_from_any_depth', () => {
        const ctx = { title: "Dub", rows: [1, 2], person: { name: "King" } };
        assert.strictEqual(Handlebars.compile('{{#each rows}}[{{@root.title}}]{{/each}}')(ctx), '[Dub][Dub]');
        assert.strictEqual(Handlebars.compile('{{#with person}}[{{@root.title}}]{{/with}}')(ctx), '[Dub]');
        assert.strictEqual(Handlebars.compile('[{{@root.title}}]')(ctx), '[Dub]');
        assert.strictEqual(Handlebars.compile('{{#each rows}}[{{@root.person.name}}]{{/each}}')(ctx), '[King][King]');
    });

    // The reason `../` is stripped rather than walked on the Rust side.
    it('a_parent_prefix_makes_no_difference_to_root', () => {
        const ctx = { title: "Dub", rows: [1, 2] };
        assert.strictEqual(Handlebars.compile('{{#each rows}}[{{@../root.title}}]{{/each}}')(ctx), '[Dub][Dub]');
    });

    it('the_root_can_be_tested_counted_and_used_as_a_subject', () => {
        const ctx = { title: "Dub", rows: [{ name: "King" }], person: { name: "Tubby" } };
        assert.strictEqual(Handlebars.compile('{{#each rows}}[{{#if @root.title}}y{{/if}}]{{/each}}')(ctx), '[y]');
        assert.strictEqual(Handlebars.compile('{{#each rows}}[{{@root.rows.length}}]{{/each}}')(ctx), '[1]');
        assert.strictEqual(Handlebars.compile('{{#each @root.rows}}[{{ name }}]{{/each}}')(ctx), '[King]');
        assert.strictEqual(Handlebars.compile('{{#with @root.person}}[{{ name }}]{{/with}}')(ctx), '[Tubby]');
    });

    // Why bare `{{@root}}` is a named error on the Rust side rather than a guess: there is nothing
    // useful to write for the whole context.
    it('bare_root_writes_the_object_itself', () => {
        assert.strictEqual(Handlebars.compile('[{{@root}}]')({ title: "Dub" }), '[[object Object]]');
    });

    it('a_list_can_be_tested_and_iterated', () => {
        const template = Handlebars.compile('{{#if rows}}<ul>{{#each rows}}<li>{{name}}</li>{{/each}}</ul>{{/if}}');
        assert.strictEqual(template({ rows: [{ name: "King" }] }), '<ul><li>King</li></ul>');
        assert.strictEqual(template({}), '');
    });

    // Mirrors `double_braces_escape_and_triple_braces_do_not` in the Rust suite.
    it('double_braces_escape_and_triple_braces_do_not', () => {
        const template = Handlebars.compile('<p>{{ two }}|{{{ three }}}</p>');
        const result = template({ two: "a&b<c>", three: "a&b<c>" });
        assert.strictEqual(result, '<p>a&amp;b&lt;c&gt;|a&b<c></p>');
    });

    // Mirrors `escaping_covers_the_handlebars_character_set`. This is the definition the Rust
    // escaper is written against.
    it('escaping_covers_the_handlebars_character_set', () => {
        const template = Handlebars.compile('{{ value }}');
        assert.strictEqual(template({ value: `& < > " ' \` =` }),
            '&amp; &lt; &gt; &quot; &#x27; &#x60; &#x3D;');
        assert.strictEqual(template({ value: "plain text 123" }), 'plain text 123');
        assert.strictEqual(template({ value: "héllo → <b>" }), 'héllo → &lt;b&gt;');
    });

    it('escaping_applies_inside_blocks_and_records', () => {
        const list = Handlebars.compile('{{#each rows}}<li>{{name}}</li>{{/each}}');
        assert.strictEqual(list({ rows: [{ name: "Tom & Jerry" }] }), '<li>Tom &amp; Jerry</li>');

        const record = Handlebars.compile('{{person.name}}');
        assert.strictEqual(record({ person: { name: "<script>" } }), '&lt;script&gt;');
    });

    // Mirrors `a_partial_renders_against_the_context_it_was_included_from`. A partial rendering
    // against the surrounding context is the behaviour the Rust side inlines to reproduce.
    it('a_partial_renders_against_the_context_it_was_included_from', () => {
        Handlebars.registerPartial('header', '<h1>{{ title }}</h1>');
        Handlebars.registerPartial('row', '<li id="r{{ id }}">{{ name }}</li>');

        const template = Handlebars.compile('{{> header}}<ul>{{#each rows}}{{> row}}{{/each}}</ul>');
        const result = template({ title: "Dub", rows: [{ id: 1, name: "King" }, { id: 2, name: "Tubby" }] });
        assert.strictEqual(result, '<h1>Dub</h1><ul><li id="r1">King</li><li id="r2">Tubby</li></ul>');
    });

    it('values_written_by_a_partial_are_escaped', () => {
        Handlebars.registerPartial('row', '<li id="r{{ id }}">{{ name }}</li>');
        const template = Handlebars.compile('{{> row}}');
        assert.strictEqual(template({ id: 1, name: "Tom & Jerry" }), '<li id="r1">Tom &amp; Jerry</li>');
    });

    // The rest of the supported subset, so every construct the README lists as working is checked
    // against real handlebars.js as well as against the Rust implementation.

    it('each_index', () => {
        const template = Handlebars.compile('{{#each rows}}{{@index}}:{{name}} {{/each}}');
        assert.strictEqual(template({ rows: [{ name: "a" }, { name: "b" }] }), '0:a 1:b ');
    });

    it('each_first_and_last', () => {
        const template = Handlebars.compile('{{#each xs}}[{{@first}},{{@last}},{{@index}}]{{/each}}');
        assert.strictEqual(template({ xs: [1, 2, 3] }),
            '[true,false,0][false,false,1][false,true,2]');
        // A one-item list is both, which is what makes them a pair of independent tests rather
        // than "is the index zero".
        assert.strictEqual(Handlebars.compile('{{#each xs}}[{{@first}},{{@last}}]{{/each}}')({ xs: [9] }),
            '[true,true]');
    });

    it('each_first_and_last_are_conditions_too', () => {
        const separator = Handlebars.compile('{{#each xs}}{{this}}{{#unless @last}}, {{/unless}}{{/each}}');
        assert.strictEqual(separator({ xs: [1, 2, 3] }), '1, 2, 3');

        const firstOnly = Handlebars.compile('{{#each xs}}{{#if @first}}F{{else}}-{{/if}}{{/each}}');
        assert.strictEqual(firstOnly({ xs: [1, 2, 3] }), 'F--');
    });

    // An `@…` comes from the loop, and blocks that supply nothing are transparent to it. These pin
    // the rule the Rust lookup was written against.
    it('a_private_is_visible_through_blocks_that_supply_nothing', () => {
        const insideIf = Handlebars.compile('{{#each xs}}{{#if on}}[{{@index}}]{{/if}}{{/each}}');
        assert.strictEqual(insideIf({ xs: [{ on: true }, { on: false }] }), '[0]');

        const insideWith = Handlebars.compile('{{#each rows}}{{#with p}}[{{@index}}:{{n}}]{{/with}}{{/each}}');
        assert.strictEqual(insideWith({ rows: [{ p: { n: 'a' } }, { p: { n: 'b' } }] }), '[0:a][1:b]');

        const chained = Handlebars.compile('{{#each xs}}{{#if @last}}L{{else if @first}}F{{else}}m{{/if}}{{/each}}');
        assert.strictEqual(chained({ xs: [1, 2, 3] }), 'FmL');
    });

    // `../` on a private counts loops, not blocks: both of these read the *outer* loop's index
    // even though a block sits in between.
    it('a_parent_private_steps_out_one_loop_not_one_block', () => {
        const throughIf = Handlebars.compile('{{#each rows}}{{#each cells}}{{#if on}}{{@../index}}{{/if}}{{/each}};{{/each}}');
        assert.strictEqual(throughIf({ rows: [{ cells: [{ on: true }, { on: true }] }, { cells: [{ on: true }] }] }),
            '00;1;');

        const throughWith = Handlebars.compile('{{#each rows}}{{#each cells}}{{#with q}}{{@../index}}{{/with}}{{/each}};{{/each}}');
        assert.strictEqual(throughWith({ rows: [{ cells: [{ q: {} }, { q: {} }] }, { cells: [{ q: {} }] }] }),
            '00;1;');
    });

    it('each_first_reaches_the_enclosing_loop', () => {
        const template = Handlebars.compile('{{#each rows}}{{#each cells}}{{@../first}}/{{@first}};{{/each}}|{{/each}}');
        assert.strictEqual(template({ rows: [{ cells: [1, 2] }, { cells: [3] }] }),
            'true/true;true/false;|false/true;|');
    });

    it('each_else', () => {
        const template = Handlebars.compile('{{#each rows}}{{name}}{{else}}none{{/each}}');
        assert.strictEqual(template({ rows: [{ name: "a" }] }), 'a');
        assert.strictEqual(template({ rows: [] }), 'none');
    });

    it('unless_else', () => {
        const template = Handlebars.compile('{{#unless a}}no{{else}}yes{{/unless}}');
        assert.strictEqual(template({ a: false }), 'no');
        assert.strictEqual(template({ a: true }), 'yes');
    });

    it('else_if_chains', () => {
        const template = Handlebars.compile('{{#if a}}A{{else if b}}B{{else}}C{{/if}}');
        assert.strictEqual(template({ a: true, b: false }), 'A');
        assert.strictEqual(template({ a: false, b: true }), 'B');
        assert.strictEqual(template({ a: false, b: false }), 'C');
    });

    it('else_if_chains_more_than_once', () => {
        const template = Handlebars.compile('{{#if a}}A{{else if b}}B{{else if c}}C{{else}}D{{/if}}');
        assert.strictEqual(template({ c: true }), 'C');
        assert.strictEqual(template({}), 'D');

        const noFinalElse = Handlebars.compile('{{#if a}}A{{else if b}}B{{/if}}');
        assert.strictEqual(noFinalElse({}), '');
    });

    it('an_else_if_condition_is_truthy_not_bool', () => {
        const template = Handlebars.compile('{{#if a}}A{{else if name}}[{{name}}]{{else}}C{{/if}}');
        assert.strictEqual(template({ name: "King" }), '[King]');
        assert.strictEqual(template({ name: "" }), 'C');
    });

    // The chained helper decides the sense of the test, not the block it sits in. This is the
    // behaviour the Rust side was written against, so it is worth pinning here rather than
    // reasoning about it.
    it('a_chained_helper_sets_its_own_sense', () => {
        const insideUnless = Handlebars.compile('{{#unless a}}U{{else if b}}B{{else}}C{{/unless}}');
        assert.strictEqual(insideUnless({ a: false, b: true }), 'U');
        assert.strictEqual(insideUnless({ a: true, b: true }), 'B');
        assert.strictEqual(insideUnless({ a: true, b: false }), 'C');

        const elseUnless = Handlebars.compile('{{#if a}}A{{else unless b}}B{{else}}C{{/if}}');
        assert.strictEqual(elseUnless({}), 'B');
        assert.strictEqual(elseUnless({ b: true }), 'C');
    });

    it('an_else_if_condition_resolves_in_its_own_scope', () => {
        const dotted = Handlebars.compile('{{#if a}}A{{else if person.name}}B{{else}}C{{/if}}');
        assert.strictEqual(dotted({ person: { name: "King" } }), 'B');

        const inEach = Handlebars.compile('{{#each rows}}{{#if hot}}H{{else if warm}}W{{else}}C{{/if}};{{/each}}');
        assert.strictEqual(inEach({ rows: [{ hot: true }, { warm: true }, {}] }), 'H;W;C;');
    });

    it('else_may_be_spaced', () => {
        const template = Handlebars.compile('{{#if a}}A{{ else }}B{{/if}}');
        assert.strictEqual(template({ a: false }), 'B');

        // The word boundary matters: this one is a variable, not a branch.
        const elsewhere = Handlebars.compile('[{{ elsewhere }}]');
        assert.strictEqual(elsewhere({ elsewhere: "town" }), '[town]');
    });

    it('each_block_param', () => {
        const template = Handlebars.compile('{{#each rows as |row|}}[{{row.name}}]{{/each}}');
        assert.strictEqual(template({ rows: [{ name: "a" }, { name: "b" }] }), '[a][b]');
    });

    it('parent_scope', () => {
        const template = Handlebars.compile('{{#each rows}}{{name}} of {{../company}};{{/each}}');
        assert.strictEqual(template({ company: "Studio One", rows: [{ name: "King" }] }),
            'King of Studio One;');
    });

    // A comment's trimming close puts the `~` inside the token, which is what made the long form
    // hard: `--~}}` shares no prefix with `--}}`. These four pin what handlebars.js actually does.
    it('a_comment_can_trim_the_whitespace_after_it', () => {
        assert.strictEqual(Handlebars.compile('x{{!-- c --~}}   y')({}), 'xy');
        assert.strictEqual(Handlebars.compile('x{{! c ~}}   y')({}), 'xy');
        assert.strictEqual(Handlebars.compile('x   {{~!-- c --~}}\n\n   y')({}), 'xy');
        // Without the `~` the whitespace stays, which is what makes the rest mean something.
        assert.strictEqual(Handlebars.compile('x{{!-- c --}}   y')({}), 'x   y');
    });

    it('a_comment_ends_at_its_first_close', () => {
        assert.strictEqual(Handlebars.compile('x{{!-- a --}} b --~}}   y')({}), 'x b --~}}   y');
    });

    it('a_comment_may_be_empty', () => {
        assert.strictEqual(Handlebars.compile('x{{!}}y')({}), 'xy');
        assert.strictEqual(Handlebars.compile('x{{!----}}y')({}), 'xy');
        assert.strictEqual(Handlebars.compile('x{{!~}}   y')({}), 'xy');
        // `{{}}` has no name and is not a comment, so it stays a parse error in both.
        assert.throws(() => Handlebars.compile('x{{}}y')({}));
    });

    it('a_long_comment_swallows_braces_and_stray_tildes', () => {
        assert.strictEqual(Handlebars.compile('x{{!-- {{a}} and ~}} and -- --}}y')({}), 'xy');
    });

    // A tag alone on its line leaves no trace: its indentation and its trailing newline both go.
    // These pin the rule the Rust implementation was written against; each has a namesake in
    // `typed-handlebars/tests/standalone.rs`.

    it('a_list_over_several_lines_renders_as_written', () => {
        const template = Handlebars.compile('<ul>\n{{#each rows}}\n  <li>{{n}}</li>\n{{/each}}\n</ul>');
        assert.strictEqual(template({ rows: [{ n: 1 }, { n: 2 }] }),
            '<ul>\n  <li>1</li>\n  <li>2</li>\n</ul>');
    });

    it('a_standalone_tag_takes_its_indentation_and_its_newline', () => {
        assert.strictEqual(Handlebars.compile('a\n{{#if x}}\nB\n{{/if}}\nc')({ x: 1 }), 'a\nB\nc');
        assert.strictEqual(Handlebars.compile('a\n  {{#if x}}\n  B\n  {{/if}}\nc')({ x: 1 }), 'a\n  B\nc');
        assert.strictEqual(Handlebars.compile('a\n\t{{#if x}}\nB\n{{/if}}\nb')({ x: 1 }), 'a\nB\nb');
    });

    it('comments_and_else_stand_alone', () => {
        assert.strictEqual(Handlebars.compile('a\n{{! hi }}\nc')({}), 'a\nc');
        const branches = Handlebars.compile('a\n{{#if x}}\nB\n{{else}}\nC\n{{/if}}\nd');
        assert.strictEqual(branches({ x: 1 }), 'a\nB\nd');
        assert.strictEqual(branches({ x: 0 }), 'a\nC\nd');
    });

    // The line between the two halves of the rule: an interpolation is there to produce output,
    // so its line is real.
    it('an_interpolation_is_not_standalone', () => {
        assert.strictEqual(Handlebars.compile('a\n{{n}}\nb')({ n: 'N' }), 'a\nN\nb');
        assert.strictEqual(Handlebars.compile('a\n{{{n}}}\nb')({ n: 'N' }), 'a\nN\nb');
    });

    it('anything_else_on_the_line_cancels_it', () => {
        assert.strictEqual(Handlebars.compile('a\n{{#if x}} z\nB\n{{/if}}\nc')({ x: 1 }), 'a\n z\nB\nc');
        assert.strictEqual(Handlebars.compile('a\n{{#if x}}{{/if}}\nb')({ x: 1 }), 'a\n\nb');
        assert.strictEqual(Handlebars.compile('a\n{{! c }} {{! d }}\nz')({}), 'a\n \nz');
    });

    it('the_edges_of_the_template_bound_a_line', () => {
        assert.strictEqual(Handlebars.compile('{{#if x}}\nB\n{{/if}}\nc')({ x: 1 }), 'B\nc');
        assert.strictEqual(Handlebars.compile('  {{#if x}}\nB\n{{/if}}\nc')({ x: 1 }), 'B\nc');
        assert.strictEqual(Handlebars.compile('a\n{{#if x}}\nB\n{{/if}}')({ x: 1 }), 'a\nB\n');
        assert.strictEqual(Handlebars.compile('a\n{{#if x}}\nB\n{{/if}}   ')({ x: 1 }), 'a\nB\n');
        assert.strictEqual(Handlebars.compile('{{! c }}')({}), '');
    });

    it('standing_alone_carries_forward_to_the_next_tag', () => {
        assert.strictEqual(Handlebars.compile('a\n{{! c }}\n  {{#if x}}\nB\n{{/if}}\nz')({ x: 1 }), 'a\nB\nz');
        assert.strictEqual(Handlebars.compile('a\n{{! c }}\n{{! d }}\nz')({}), 'a\nz');
        assert.strictEqual(Handlebars.compile('a\n{{! c }}\n{{n}}\nz')({ n: 'N' }), 'a\nN\nz');
    });

    it('only_the_tags_own_newline_is_taken', () => {
        assert.strictEqual(Handlebars.compile('a\n\n{{! c }}\n\nb')({}), 'a\n\n\nb');
    });

    // A partial alone on its line is standalone too, and its indentation is applied to every line
    // it emits rather than dropped. Mirrors the `partials` module in
    // `typed-handlebars/tests/standalone.rs`, whose fixtures hold these same strings.
    it('a_standalone_partial_indents_every_line', () => {
        Handlebars.registerPartial('three_lines', '<a>\n<b>\n<c>');
        assert.strictEqual(Handlebars.compile('start\n    {{> three_lines}}\nend')({}),
            'start\n    <a>\n    <b>\n    <c>end');

        // Anything else on the line and it is an ordinary partial: no indent, newline kept.
        assert.strictEqual(Handlebars.compile('start\n  x{{> three_lines}}\nend')({}),
            'start\n  x<a>\n<b>\n<c>\nend');

        // The end of the template ends the line here too.
        assert.strictEqual(Handlebars.compile('start\n    {{> three_lines}}')({}),
            'start\n    <a>\n    <b>\n    <c>');
    });

    it('a_standalone_partial_leaves_no_dangling_indent', () => {
        Handlebars.registerPartial('ends_with_newline', '<a>\n');
        assert.strictEqual(Handlebars.compile('start\n    {{> ends_with_newline}}\nend')({}),
            'start\n    <a>\nend');

        Handlebars.registerPartial('nothing', '');
        assert.strictEqual(Handlebars.compile('start\n    {{> nothing}}\nend')({}), 'start\nend');
    });

    // Indents accumulate rather than replace: a partial included from inside another standalone
    // partial is indented by both. This is the rule the Rust assembler composes.
    it('nested_standalone_partials_add_their_indents', () => {
        Handlebars.registerPartial('one_line', '<a>');
        Handlebars.registerPartial('includes_another', 'X\n  {{> one_line}}\nY');
        assert.strictEqual(Handlebars.compile('start\n    {{> includes_another}}\nend')({}),
            'start\n    X\n      <a>Yend');

        assert.strictEqual(Handlebars.compile('start\n    {{> one_line}}\nend')({}), 'start\n    <a>end');
        assert.strictEqual(Handlebars.compile('start\n{{> one_line}}\nend')({}), 'start\n<a>end');
    });

    // Testing the loop item itself, rather than a field on it. The Rust side needs the item's
    // generated parameter bounded by `Truthy` for this, which is what makes it worth pinning.
    it('testing_the_item_itself_bounds_it', () => {
        assert.strictEqual(Handlebars.compile('{{#each xs}}{{#if this}}[{{this}}]{{/if}}{{/each}}')({ xs: [1, 0, 2] }),
            '[1][2]');
        assert.strictEqual(Handlebars.compile('{{#each xs}}{{#unless this}}n{{/unless}}{{/each}}')({ xs: [1, 0] }),
            'n');
        // An alias reaches the same scope as `this`.
        assert.strictEqual(Handlebars.compile('{{#each xs as |x|}}{{#if x}}[{{x}}]{{/if}}{{/each}}')({ xs: ['a', ''] }),
            '[a]');
    });

    it('whitespace_trimming', () => {
        const template = Handlebars.compile('  {{~#if some ~}}   Hello{{~/if~}}');
        assert.strictEqual(template({ some: true }), 'Hello');
    });

    // KNOWN DIVERGENCE. In handlebars.js a raw block calls a helper of that name and renders
    // whatever it returns, so with no helper registered the block renders as nothing. typed-handlebars
    // has no custom helpers, and always passes the content through — i.e. it behaves as though a
    // passthrough helper were registered, which is the second case below.
    it('literal_block_needs_a_helper_in_handlebars_js', () => {
        const source = '{{{{skip}}}}wang doodle{{{{/skip}}}}';
        assert.strictEqual(Handlebars.compile(source)({}), '');

        Handlebars.registerHelper('skip', function (this: unknown, options: any) {
            return options.fn();
        });
        assert.strictEqual(Handlebars.compile(source)({}), 'wang doodle');
    });

    it('comments', () => {
        const template = Handlebars.compile('Note: {{! ignored }}and {{!-- {{also_ignored}} --}}done');
        assert.strictEqual(template({}), 'Note: and done');
    });

    // Mirrors `variable_names_may_start_with_a_digit`. handlebars.js reads these as variable
    // references, so the Rust side renames them rather than rejecting them.
    it('variable_names_may_start_with_a_digit', () => {
        const template = Handlebars.compile('[{{ 2nd }}][{{ 42 }}]');
        assert.strictEqual(template({ '2nd': "silver", '42': "answer" }), '[silver][answer]');
    });

    it('it_works', () => {
        const template = Handlebars.compile('Hello {{{name}}}!');
        const result = template({ name: "King" });
        assert.strictEqual(result, 'Hello King!');
    });

    it.skip('test_escaped', () => {
        // Handlebars JS parser throws "skip doesn't match dandy"
        // It seems it gets confused by the inner {{{{/dandy}}}}
        const template = Handlebars.compile('{{{{skip}}}}wang doodle {{{{/dandy}}}}{{{{/skip}}}}');
        const result = template({});
        assert.strictEqual(result, 'wang doodle {{{{/dandy}}}}');
    });

});

