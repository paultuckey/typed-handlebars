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

    it('test_format_number', () => {
        Handlebars.registerHelper('format', function(fmt, value) {
            if (fmt === "{:.2}" && typeof value === 'number') {
                return value.toFixed(2);
            }
            return value;
        });

        const template = Handlebars.compile('Price: ${{format "{:.2}" price}}');
        const result = template({ price: 12.2345 });
        expect(result).toBe('Price: $12.23');
    });
});

