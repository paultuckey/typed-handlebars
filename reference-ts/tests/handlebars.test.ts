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

    it('each_block_param', () => {
        const template = Handlebars.compile('{{#each rows as |row|}}[{{row.name}}]{{/each}}');
        expect(template({ rows: [{ name: "a" }, { name: "b" }] })).toBe('[a][b]');
    });

    it('parent_scope', () => {
        const template = Handlebars.compile('{{#each rows}}{{name}} of {{../company}};{{/each}}');
        expect(template({ company: "Studio One", rows: [{ name: "King" }] }))
            .toBe('King of Studio One;');
    });

    it('whitespace_trimming', () => {
        const template = Handlebars.compile('  {{~#if some ~}}   Hello{{~/if~}}');
        expect(template({ some: true })).toBe('Hello');
    });

    // KNOWN DIVERGENCE. In handlebars.js a raw block calls a helper of that name and renders
    // whatever it returns, so with no helper registered the block renders as nothing. dry-handlebars
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

