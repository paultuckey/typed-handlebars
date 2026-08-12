//! Resolves `{{> partial}}` by splicing the partial's text into whoever included it.
//!
//! Inlining is what gives partials the semantics an `.hbs` author expects. In handlebars.js a
//! partial renders against the context it was called from, so
//!
//! ```handlebars
//! {{#each rows}}{{> row}}{{/each}}
//! ```
//!
//! makes `row.hbs`'s `{{ name }}` mean the current row. Splicing the text achieves exactly that:
//! the partial's variables land in whatever scope surrounds the call, and the context tree sees one
//! template. It also means a partial costs nothing at run time — there is no second `render` call
//! and no intermediate `String` — and that its output can never be double-escaped, because it is
//! generated code rather than a value being written.
//!
//! Each `.hbs` file still gets its own type from `directory!`, so a partial can be rendered on its
//! own as well as included.
//!
//! # Positions
//!
//! Splicing moves text around, which would wreck the line and column numbers in template errors.
//! An [`Assembly`] therefore records where each run of text came from, so a position in the
//! assembled template can be turned back into a file, line and column.

use std::fs;
use std::path::{Path, PathBuf};

use crate::parser::expression::Expression;

/// A template and every partial it pulls in, as one piece of text.
pub struct Assembly {
    /// The template with each `{{> partial}}` replaced by that partial's text.
    pub text: String,
    /// Every file that went into it, so generated code can depend on all of them.
    pub includes: Vec<String>,
    files: Vec<Source>,
    segments: Vec<Segment>,
}

/// One file that contributed text.
struct Source {
    path: Option<String>,
    content: String,
}

/// A run of assembled text copied verbatim from one file.
struct Segment {
    /// Where the run starts in [`Assembly::text`].
    start: usize,
    /// Which file it came from.
    file: usize,
    /// Where it starts in that file.
    offset: usize,
}

/// Where a position in the assembled text really lives.
pub struct Location {
    /// The `.hbs` file, when the text came from one.
    pub path: Option<String>,
    pub line: usize,
    pub column: usize,
}

impl Assembly {
    /// Assembles a template.
    ///
    /// `directory` is where partials are looked up; `None` means the template has nowhere to look,
    /// which is the case for `str!`.
    pub fn build(
        content: &str,
        path: Option<&str>,
        directory: Option<&Path>,
    ) -> Result<Self, String> {
        let mut assembly = Assembly {
            text: String::new(),
            includes: Vec::new(),
            files: Vec::new(),
            segments: Vec::new(),
        };
        if let Some(path) = path {
            assembly.includes.push(path.to_string());
        }
        let file = assembly.add(path, content);
        let mut stack = Vec::new();
        assembly.append(file, content, directory, &mut stack)?;
        Ok(assembly)
    }

    fn add(&mut self, path: Option<&str>, content: &str) -> usize {
        self.files.push(Source {
            path: path.map(str::to_string),
            content: content.to_string(),
        });
        self.files.len() - 1
    }

    /// Copies `content` into the assembled text, splicing in any partials it names.
    fn append(
        &mut self,
        file: usize,
        content: &str,
        directory: Option<&Path>,
        stack: &mut Vec<PathBuf>,
    ) -> Result<(), String> {
        let mut cursor = 0usize;
        let mut expression = Expression::from(content).map_err(|e| e.to_string())?;

        while let Some(expr) = expression {
            if let Some(name) = partial_name(expr.content) {
                let name = name?;

                // `prefix` and `postfix` already account for `~` trim markers, so using their
                // bounds keeps trimming working across a splice.
                let up_to = offset_of(expr.prefix, content) + expr.prefix.len();
                let resume = offset_of(expr.postfix, content);

                self.copy(file, content, cursor, up_to);
                self.splice(&name, directory, stack)?;
                cursor = resume;
            }
            expression = expr.next().map_err(|e| e.to_string())?;
        }

        self.copy(file, content, cursor, content.len());
        Ok(())
    }

    /// Copies `content[from..to]` into the assembled text, remembering where it came from.
    fn copy(&mut self, file: usize, content: &str, from: usize, to: usize) {
        if from >= to {
            return;
        }
        self.segments.push(Segment {
            start: self.text.len(),
            file,
            offset: from,
        });
        self.text.push_str(&content[from..to]);
    }

    /// Reads a partial and appends it in place of the `{{> … }}` that named it.
    fn splice(
        &mut self,
        name: &str,
        directory: Option<&Path>,
        stack: &mut Vec<PathBuf>,
    ) -> Result<(), String> {
        let directory = directory.ok_or_else(|| {
            format!(
                "`{{{{> {}}}}}` needs a template directory to look in, and this template has none \
                 — partials work with `directory!` and `file!`, not `str!`",
                name
            )
        })?;

        let path = directory.join(format!("{}.hbs", name));
        if !path.is_file() {
            return Err(format!(
                "no partial named `{}` — looked for {}{}",
                name,
                crate::relative(&path.to_string_lossy()),
                suggestions(directory)
            ));
        }

        if stack.contains(&path) {
            let mut chain: Vec<String> = stack
                .iter()
                .map(|p| file_stem(p).to_string())
                .collect::<Vec<_>>();
            chain.push(name.to_string());
            return Err(format!(
                "partial `{}` includes itself: {}",
                name,
                chain.join(" → ")
            ));
        }

        let content = fs::read_to_string(&path).map_err(|e| {
            format!(
                "could not read partial {}: {}",
                crate::relative(&path.to_string_lossy()),
                e
            )
        })?;
        let path_str = path.to_string_lossy().to_string();

        // Generated code has to depend on the partial too, or editing it would not rebuild the
        // template that includes it.
        if !self.includes.contains(&path_str) {
            self.includes.push(path_str.clone());
        }

        let file = self.add(Some(&path_str), &content);
        stack.push(path.clone());
        let result = self.append(file, &content, Some(directory), stack);
        stack.pop();
        result
    }

    /// Maps a byte offset in the assembled text back to the file it came from.
    pub fn locate(&self, offset: usize) -> Option<Location> {
        let segment = self
            .segments
            .iter()
            .rev()
            .find(|segment| segment.start <= offset)?;
        let source = &self.files[segment.file];
        let in_file = segment.offset + (offset - segment.start);
        let before = source.content.get(..in_file)?;
        Some(Location {
            path: source.path.clone(),
            line: before.matches('\n').count() + 1,
            column: before.rsplit('\n').next().map_or(in_file, str::len) + 1,
        })
    }

    /// The path of the template itself, if it came from a file.
    pub fn path(&self) -> Option<&str> {
        self.files.first().and_then(|f| f.path.as_deref())
    }
}

/// Recognises `{{> name}}`, returning the partial's name.
///
/// Returns `None` for anything that is not a partial, and an error for a partial this crate cannot
/// resolve yet.
fn partial_name(content: &str) -> Option<Result<String, String>> {
    let rest = content.trim().strip_prefix('>')?.trim();
    if rest.is_empty() {
        return Some(Err("`{{> }}` does not name a partial".to_string()));
    }
    let mut words = rest.split_whitespace();
    let name = words.next().unwrap_or_default();
    if words.next().is_some() {
        return Some(Err(format!(
            "`{{{{> {} …}}}}` passes arguments to a partial, which is not supported yet — a \
             partial sees the context it was included from",
            name
        )));
    }
    Some(Ok(name.to_string()))
}

/// Lists the partials that do exist, to turn a typo into an obvious fix.
fn suggestions(directory: &Path) -> String {
    let mut names: Vec<String> = Vec::new();
    if let Ok(entries) = fs::read_dir(directory) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().is_some_and(|ext| ext == "hbs") {
                names.push(file_stem(&path).to_string());
            }
        }
    }
    names.sort();
    if names.is_empty() {
        String::new()
    } else {
        format!(". Available: {}", names.join(", "))
    }
}

fn file_stem(path: &Path) -> std::borrow::Cow<'_, str> {
    path.file_stem().unwrap_or_default().to_string_lossy()
}

/// Byte offset of `part` within `whole`, which it must be a slice of.
fn offset_of(part: &str, whole: &str) -> usize {
    part.as_ptr() as usize - whole.as_ptr() as usize
}
