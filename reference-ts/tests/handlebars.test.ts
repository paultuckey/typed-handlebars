import { describe, it, expect } from 'vitest';
import Handlebars from 'handlebars';

describe('Handlebars Reference Tests', () => {
    it('basic_usage', () => {
        const template = Handlebars.compile('<p>{{firstname}} {{lastname}}</p>');
        const result = template({ firstname: "King", lastname: "Tubby" });
        expect(result).toBe('<p>King Tubby</p>');
    });

    it('path_expressions', () => {
        const template = Handlebars.compile('{{person.firstname}} {{person.lastname}}');
        const person = {
            firstname: "King",
            lastname: "Tubby",
        };
        const result = template({ person });
        expect(result).toBe('King Tubby');
    });

    it('if_helper', () => {
        const template = Handlebars.compile('<div>{{#if has_author}}<h1>{{first_name}} {{last_name}}</h1>{{/if}}</div>');

        const resultTrue = template({ has_author: true, first_name: "King", last_name: "Tubby" });
        expect(resultTrue).toBe('<div><h1>King Tubby</h1></div>');

        const resultFalse = template({ has_author: false, first_name: "King", last_name: "Tubby" });
        expect(resultFalse).toBe('<div></div>');
    });

    it('with_helper', () => {
        const template = Handlebars.compile('<div>{{#with author}}<h1>{{first_name}} {{last_name}}</h1>{{/with}}</div>');
        const author = {
            first_name: "King",
            last_name: "Tubby",
        };
        const resultTrue = template({ author });
        expect(resultTrue).toBe('<div><h1>King Tubby</h1></div>');

        const resultFalse = template({ author: null });
        expect(resultFalse).toBe('<div></div>');
    });

    it('with_else_helper', () => {
        const template = Handlebars.compile('<div>{{#with author}}<h1>{{first_name}}</h1>{{else}}<h1>Unknown</h1>{{/with}}</div>');

        const author = {
            first_name: "King",
            last_name: "Tubby",
        };

        const resultTrue = template({ author });
        expect(resultTrue).toBe('<div><h1>King</h1></div>');

        const resultFalse = template({ author: null });
        expect(resultFalse).toBe('<div><h1>Unknown</h1></div>');
    });

    // Mirrors `a_variable_can_be_tested_and_printed` in the Rust suite: testing a variable must not
    // stop you printing it.
    it('a_variable_can_be_tested_and_printed', () => {
        const template = Handlebars.compile('[{{#if title}}{{title}}{{/if}}]');
        expect(template({ title: "Dub" })).toBe('[Dub]');
        expect(template({ title: "" })).toBe('[]');
        expect(template({ title: 7 })).toBe('[7]');
        expect(template({ title: 0 })).toBe('[]');
        expect(template({ title: true })).toBe('[true]');
        expect(template({ title: false })).toBe('[]');
    });

    // Mirrors `falsiness_follows_handlebars` in the Rust suite. This is the definition the Rust
    // `Truthy` trait is written against.
    it('falsiness_follows_handlebars', () => {
        const template = Handlebars.compile('[{{#if value}}yes{{/if}}]');
        expect(template({})).toBe('[]');
        expect(template({ value: false })).toBe('[]');
        expect(template({ value: true })).toBe('[yes]');
        expect(template({ value: "" })).toBe('[]');
        expect(template({ value: "x" })).toBe('[yes]');
        expect(template({ value: 0 })).toBe('[]');
        expect(template({ value: -1 })).toBe('[yes]');
        expect(template({ value: [] })).toBe('[]');
        expect(template({ value: [1] })).toBe('[yes]');
    });

    it('a_list_can_be_tested_and_iterated', () => {
        const template = Handlebars.compile('{{#if rows}}<ul>{{#each rows}}<li>{{name}}</li>{{/each}}</ul>{{/if}}');
        expect(template({ rows: [{ name: "King" }] })).toBe('<ul><li>King</li></ul>');
        expect(template({})).toBe('');
    });

    // Mirrors `double_braces_escape_and_triple_braces_do_not` in the Rust suite.
    it('double_braces_escape_and_triple_braces_do_not', () => {
        const template = Handlebars.compile('<p>{{ two }}|{{{ three }}}</p>');
        const result = template({ two: "a&b<c>", three: "a&b<c>" });
        expect(result).toBe('<p>a&amp;b&lt;c&gt;|a&b<c></p>');
    });

    // Mirrors `escaping_covers_the_handlebars_character_set`. This is the definition the Rust
    // escaper is written against.
    it('escaping_covers_the_handlebars_character_set', () => {
        const template = Handlebars.compile('{{ value }}');
        expect(template({ value: `& < > " ' \` =` }))
            .toBe('&amp; &lt; &gt; &quot; &#x27; &#x60; &#x3D;');
        expect(template({ value: "plain text 123" })).toBe('plain text 123');
        expect(template({ value: "héllo → <b>" })).toBe('héllo → &lt;b&gt;');
    });

    it('escaping_applies_inside_blocks_and_records', () => {
        const list = Handlebars.compile('{{#each rows}}<li>{{name}}</li>{{/each}}');
        expect(list({ rows: [{ name: "Tom & Jerry" }] })).toBe('<li>Tom &amp; Jerry</li>');

        const record = Handlebars.compile('{{person.name}}');
        expect(record({ person: { name: "<script>" } })).toBe('&lt;script&gt;');
    });

    // Mirrors `a_partial_renders_against_the_context_it_was_included_from`. A partial rendering
    // against the surrounding context is the behaviour the Rust side inlines to reproduce.
    it('a_partial_renders_against_the_context_it_was_included_from', () => {
        Handlebars.registerPartial('header', '<h1>{{ title }}</h1>');
        Handlebars.registerPartial('row', '<li id="r{{ id }}">{{ name }}</li>');

        const template = Handlebars.compile('{{> header}}<ul>{{#each rows}}{{> row}}{{/each}}</ul>');
        const result = template({ title: "Dub", rows: [{ id: 1, name: "King" }, { id: 2, name: "Tubby" }] });
        expect(result).toBe('<h1>Dub</h1><ul><li id="r1">King</li><li id="r2">Tubby</li></ul>');
    });

    it('values_written_by_a_partial_are_escaped', () => {
        Handlebars.registerPartial('row', '<li id="r{{ id }}">{{ name }}</li>');
        const template = Handlebars.compile('{{> row}}');
        expect(template({ id: 1, name: "Tom & Jerry" })).toBe('<li id="r1">Tom &amp; Jerry</li>');
    });

    // The rest of the supported subset, so every construct the README lists as working is checked
    // against real handlebars.js as well as against the Rust implementation.

    it('each_index', () => {
        const template = Handlebars.compile('{{#each rows}}{{@index}}:{{name}} {{/each}}');
        expect(template({ rows: [{ name: "a" }, { name: "b" }] })).toBe('0:a 1:b ');
    });

    it('each_first_and_last', () => {
        const template = Handlebars.compile('{{#each xs}}[{{@first}},{{@last}},{{@index}}]{{/each}}');
        expect(template({ xs: [1, 2, 3] }))
            .toBe('[true,false,0][false,false,1][false,true,2]');
        // A one-item list is both, which is what makes them a pair of independent tests rather
        // than "is the index zero".
        expect(Handlebars.compile('{{#each xs}}[{{@first}},{{@last}}]{{/each}}')({ xs: [9] }))
            .toBe('[true,true]');
    });

    it('each_first_and_last_are_conditions_too', () => {
        const separator = Handlebars.compile('{{#each xs}}{{this}}{{#unless @last}}, {{/unless}}{{/each}}');
        expect(separator({ xs: [1, 2, 3] })).toBe('1, 2, 3');

        const firstOnly = Handlebars.compile('{{#each xs}}{{#if @first}}F{{else}}-{{/if}}{{/each}}');
        expect(firstOnly({ xs: [1, 2, 3] })).toBe('F--');
    });

    // An `@…` comes from the loop, and blocks that supply nothing are transparent to it. These pin
    // the rule the Rust lookup was written against.
    it('a_private_is_visible_through_blocks_that_supply_nothing', () => {
        const insideIf = Handlebars.compile('{{#each xs}}{{#if on}}[{{@index}}]{{/if}}{{/each}}');
        expect(insideIf({ xs: [{ on: true }, { on: false }] })).toBe('[0]');

        const insideWith = Handlebars.compile('{{#each rows}}{{#with p}}[{{@index}}:{{n}}]{{/with}}{{/each}}');
        expect(insideWith({ rows: [{ p: { n: 'a' } }, { p: { n: 'b' } }] })).toBe('[0:a][1:b]');

        const chained = Handlebars.compile('{{#each xs}}{{#if @last}}L{{else if @first}}F{{else}}m{{/if}}{{/each}}');
        expect(chained({ xs: [1, 2, 3] })).toBe('FmL');
    });

    // `../` on a private counts loops, not blocks: both of these read the *outer* loop's index
    // even though a block sits in between.
    it('a_parent_private_steps_out_one_loop_not_one_block', () => {
        const throughIf = Handlebars.compile('{{#each rows}}{{#each cells}}{{#if on}}{{@../index}}{{/if}}{{/each}};{{/each}}');
        expect(throughIf({ rows: [{ cells: [{ on: true }, { on: true }] }, { cells: [{ on: true }] }] }))
            .toBe('00;1;');

        const throughWith = Handlebars.compile('{{#each rows}}{{#each cells}}{{#with q}}{{@../index}}{{/with}}{{/each}};{{/each}}');
        expect(throughWith({ rows: [{ cells: [{ q: {} }, { q: {} }] }, { cells: [{ q: {} }] }] }))
            .toBe('00;1;');
    });

    it('each_first_reaches_the_enclosing_loop', () => {
        const template = Handlebars.compile('{{#each rows}}{{#each cells}}{{@../first}}/{{@first}};{{/each}}|{{/each}}');
        expect(template({ rows: [{ cells: [1, 2] }, { cells: [3] }] }))
            .toBe('true/true;true/false;|false/true;|');
    });

    it('each_else', () => {
        const template = Handlebars.compile('{{#each rows}}{{name}}{{else}}none{{/each}}');
        expect(template({ rows: [{ name: "a" }] })).toBe('a');
        expect(template({ rows: [] })).toBe('none');
    });

    it('unless_else', () => {
        const template = Handlebars.compile('{{#unless a}}no{{else}}yes{{/unless}}');
        expect(template({ a: false })).toBe('no');
        expect(template({ a: true })).toBe('yes');
    });

    it('else_if_chains', () => {
        const template = Handlebars.compile('{{#if a}}A{{else if b}}B{{else}}C{{/if}}');
        expect(template({ a: true, b: false })).toBe('A');
        expect(template({ a: false, b: true })).toBe('B');
        expect(template({ a: false, b: false })).toBe('C');
    });

    it('else_if_chains_more_than_once', () => {
        const template = Handlebars.compile('{{#if a}}A{{else if b}}B{{else if c}}C{{else}}D{{/if}}');
        expect(template({ c: true })).toBe('C');
        expect(template({})).toBe('D');

        const noFinalElse = Handlebars.compile('{{#if a}}A{{else if b}}B{{/if}}');
        expect(noFinalElse({})).toBe('');
    });

    it('an_else_if_condition_is_truthy_not_bool', () => {
        const template = Handlebars.compile('{{#if a}}A{{else if name}}[{{name}}]{{else}}C{{/if}}');
        expect(template({ name: "King" })).toBe('[King]');
        expect(template({ name: "" })).toBe('C');
    });

    // The chained helper decides the sense of the test, not the block it sits in. This is the
    // behaviour the Rust side was written against, so it is worth pinning here rather than
    // reasoning about it.
    it('a_chained_helper_sets_its_own_sense', () => {
        const insideUnless = Handlebars.compile('{{#unless a}}U{{else if b}}B{{else}}C{{/unless}}');
        expect(insideUnless({ a: false, b: true })).toBe('U');
        expect(insideUnless({ a: true, b: true })).toBe('B');
        expect(insideUnless({ a: true, b: false })).toBe('C');

        const elseUnless = Handlebars.compile('{{#if a}}A{{else unless b}}B{{else}}C{{/if}}');
        expect(elseUnless({})).toBe('B');
        expect(elseUnless({ b: true })).toBe('C');
    });

    it('an_else_if_condition_resolves_in_its_own_scope', () => {
        const dotted = Handlebars.compile('{{#if a}}A{{else if person.name}}B{{else}}C{{/if}}');
        expect(dotted({ person: { name: "King" } })).toBe('B');

        const inEach = Handlebars.compile('{{#each rows}}{{#if hot}}H{{else if warm}}W{{else}}C{{/if}};{{/each}}');
        expect(inEach({ rows: [{ hot: true }, { warm: true }, {}] })).toBe('H;W;C;');
    });

    it('else_may_be_spaced', () => {
        const template = Handlebars.compile('{{#if a}}A{{ else }}B{{/if}}');
        expect(template({ a: false })).toBe('B');

        // The word boundary matters: this one is a variable, not a branch.
        const elsewhere = Handlebars.compile('[{{ elsewhere }}]');
        expect(elsewhere({ elsewhere: "town" })).toBe('[town]');
    });

    it('each_block_param', () => {
        const template = Handlebars.compile('{{#each rows as |row|}}[{{row.name}}]{{/each}}');
        expect(template({ rows: [{ name: "a" }, { name: "b" }] })).toBe('[a][b]');
    });

    it('parent_scope', () => {
        const template = Handlebars.compile('{{#each rows}}{{name}} of {{../company}};{{/each}}');
        expect(template({ company: "Studio One", rows: [{ name: "King" }] }))
            .toBe('King of Studio One;');
    });

    // A comment's trimming close puts the `~` inside the token, which is what made the long form
    // hard: `--~}}` shares no prefix with `--}}`. These four pin what handlebars.js actually does.
    it('a_comment_can_trim_the_whitespace_after_it', () => {
        expect(Handlebars.compile('x{{!-- c --~}}   y')({})).toBe('xy');
        expect(Handlebars.compile('x{{! c ~}}   y')({})).toBe('xy');
        expect(Handlebars.compile('x   {{~!-- c --~}}\n\n   y')({})).toBe('xy');
        // Without the `~` the whitespace stays, which is what makes the rest mean something.
        expect(Handlebars.compile('x{{!-- c --}}   y')({})).toBe('x   y');
    });

    it('a_comment_ends_at_its_first_close', () => {
        expect(Handlebars.compile('x{{!-- a --}} b --~}}   y')({})).toBe('x b --~}}   y');
    });

    it('a_comment_may_be_empty', () => {
        expect(Handlebars.compile('x{{!}}y')({})).toBe('xy');
        expect(Handlebars.compile('x{{!----}}y')({})).toBe('xy');
        expect(Handlebars.compile('x{{!~}}   y')({})).toBe('xy');
        // `{{}}` has no name and is not a comment, so it stays a parse error in both.
        expect(() => Handlebars.compile('x{{}}y')({})).toThrow();
    });

    it('a_long_comment_swallows_braces_and_stray_tildes', () => {
        expect(Handlebars.compile('x{{!-- {{a}} and ~}} and -- --}}y')({})).toBe('xy');
    });

    // A tag alone on its line leaves no trace: its indentation and its trailing newline both go.
    // These pin the rule the Rust implementation was written against; each has a namesake in
    // `typed-handlebars/tests/standalone.rs`.

    it('a_list_over_several_lines_renders_as_written', () => {
        const template = Handlebars.compile('<ul>\n{{#each rows}}\n  <li>{{n}}</li>\n{{/each}}\n</ul>');
        expect(template({ rows: [{ n: 1 }, { n: 2 }] }))
            .toBe('<ul>\n  <li>1</li>\n  <li>2</li>\n</ul>');
    });

    it('a_standalone_tag_takes_its_indentation_and_its_newline', () => {
        expect(Handlebars.compile('a\n{{#if x}}\nB\n{{/if}}\nc')({ x: 1 })).toBe('a\nB\nc');
        expect(Handlebars.compile('a\n  {{#if x}}\n  B\n  {{/if}}\nc')({ x: 1 })).toBe('a\n  B\nc');
        expect(Handlebars.compile('a\n\t{{#if x}}\nB\n{{/if}}\nb')({ x: 1 })).toBe('a\nB\nb');
    });

    it('comments_and_else_stand_alone', () => {
        expect(Handlebars.compile('a\n{{! hi }}\nc')({})).toBe('a\nc');
        const branches = Handlebars.compile('a\n{{#if x}}\nB\n{{else}}\nC\n{{/if}}\nd');
        expect(branches({ x: 1 })).toBe('a\nB\nd');
        expect(branches({ x: 0 })).toBe('a\nC\nd');
    });

    // The line between the two halves of the rule: an interpolation is there to produce output,
    // so its line is real.
    it('an_interpolation_is_not_standalone', () => {
        expect(Handlebars.compile('a\n{{n}}\nb')({ n: 'N' })).toBe('a\nN\nb');
        expect(Handlebars.compile('a\n{{{n}}}\nb')({ n: 'N' })).toBe('a\nN\nb');
    });

    it('anything_else_on_the_line_cancels_it', () => {
        expect(Handlebars.compile('a\n{{#if x}} z\nB\n{{/if}}\nc')({ x: 1 })).toBe('a\n z\nB\nc');
        expect(Handlebars.compile('a\n{{#if x}}{{/if}}\nb')({ x: 1 })).toBe('a\n\nb');
        expect(Handlebars.compile('a\n{{! c }} {{! d }}\nz')({})).toBe('a\n \nz');
    });

    it('the_edges_of_the_template_bound_a_line', () => {
        expect(Handlebars.compile('{{#if x}}\nB\n{{/if}}\nc')({ x: 1 })).toBe('B\nc');
        expect(Handlebars.compile('  {{#if x}}\nB\n{{/if}}\nc')({ x: 1 })).toBe('B\nc');
        expect(Handlebars.compile('a\n{{#if x}}\nB\n{{/if}}')({ x: 1 })).toBe('a\nB\n');
        expect(Handlebars.compile('a\n{{#if x}}\nB\n{{/if}}   ')({ x: 1 })).toBe('a\nB\n');
        expect(Handlebars.compile('{{! c }}')({})).toBe('');
    });

    it('standing_alone_carries_forward_to_the_next_tag', () => {
        expect(Handlebars.compile('a\n{{! c }}\n  {{#if x}}\nB\n{{/if}}\nz')({ x: 1 })).toBe('a\nB\nz');
        expect(Handlebars.compile('a\n{{! c }}\n{{! d }}\nz')({})).toBe('a\nz');
        expect(Handlebars.compile('a\n{{! c }}\n{{n}}\nz')({ n: 'N' })).toBe('a\nN\nz');
    });

    it('only_the_tags_own_newline_is_taken', () => {
        expect(Handlebars.compile('a\n\n{{! c }}\n\nb')({})).toBe('a\n\n\nb');
    });

    // A partial alone on its line is standalone too, and its indentation is applied to every line
    // it emits rather than dropped. Mirrors the `partials` module in
    // `typed-handlebars/tests/standalone.rs`, whose fixtures hold these same strings.
    it('a_standalone_partial_indents_every_line', () => {
        Handlebars.registerPartial('three_lines', '<a>\n<b>\n<c>');
        expect(Handlebars.compile('start\n    {{> three_lines}}\nend')({}))
            .toBe('start\n    <a>\n    <b>\n    <c>end');

        // Anything else on the line and it is an ordinary partial: no indent, newline kept.
        expect(Handlebars.compile('start\n  x{{> three_lines}}\nend')({}))
            .toBe('start\n  x<a>\n<b>\n<c>\nend');

        // The end of the template ends the line here too.
        expect(Handlebars.compile('start\n    {{> three_lines}}')({}))
            .toBe('start\n    <a>\n    <b>\n    <c>');
    });

    it('a_standalone_partial_leaves_no_dangling_indent', () => {
        Handlebars.registerPartial('ends_with_newline', '<a>\n');
        expect(Handlebars.compile('start\n    {{> ends_with_newline}}\nend')({}))
            .toBe('start\n    <a>\nend');

        Handlebars.registerPartial('nothing', '');
        expect(Handlebars.compile('start\n    {{> nothing}}\nend')({})).toBe('start\nend');
    });

    // Indents accumulate rather than replace: a partial included from inside another standalone
    // partial is indented by both. This is the rule the Rust assembler composes.
    it('nested_standalone_partials_add_their_indents', () => {
        Handlebars.registerPartial('one_line', '<a>');
        Handlebars.registerPartial('includes_another', 'X\n  {{> one_line}}\nY');
        expect(Handlebars.compile('start\n    {{> includes_another}}\nend')({}))
            .toBe('start\n    X\n      <a>Yend');

        expect(Handlebars.compile('start\n    {{> one_line}}\nend')({})).toBe('start\n    <a>end');
        expect(Handlebars.compile('start\n{{> one_line}}\nend')({})).toBe('start\n<a>end');
    });

    it('whitespace_trimming', () => {
        const template = Handlebars.compile('  {{~#if some ~}}   Hello{{~/if~}}');
        expect(template({ some: true })).toBe('Hello');
    });

    // KNOWN DIVERGENCE. In handlebars.js a raw block calls a helper of that name and renders
    // whatever it returns, so with no helper registered the block renders as nothing. typed-handlebars
    // has no custom helpers, and always passes the content through — i.e. it behaves as though a
    // passthrough helper were registered, which is the second case below.
    it('literal_block_needs_a_helper_in_handlebars_js', () => {
        const source = '{{{{skip}}}}wang doodle{{{{/skip}}}}';
        expect(Handlebars.compile(source)({})).toBe('');

        Handlebars.registerHelper('skip', function (this: unknown, options: any) {
            return options.fn();
        });
        expect(Handlebars.compile(source)({})).toBe('wang doodle');
    });

    it('comments', () => {
        const template = Handlebars.compile('Note: {{! ignored }}and {{!-- {{also_ignored}} --}}done');
        expect(template({})).toBe('Note: and done');
    });

    // Mirrors `variable_names_may_start_with_a_digit`. handlebars.js reads these as variable
    // references, so the Rust side renames them rather than rejecting them.
    it('variable_names_may_start_with_a_digit', () => {
        const template = Handlebars.compile('[{{ 2nd }}][{{ 42 }}]');
        expect(template({ '2nd': "silver", '42': "answer" })).toBe('[silver][answer]');
    });

    it('it_works', () => {
        const template = Handlebars.compile('Hello {{{name}}}!');
        const result = template({ name: "King" });
        expect(result).toBe('Hello King!');
    });

    it.skip('test_escaped', () => {
        // Handlebars JS parser throws "skip doesn't match dandy"
        // It seems it gets confused by the inner {{{{/dandy}}}}
        const template = Handlebars.compile('{{{{skip}}}}wang doodle {{{{/dandy}}}}{{{{/skip}}}}');
        const result = template({});
        expect(result).toBe('wang doodle {{{{/dandy}}}}');
    });

});

