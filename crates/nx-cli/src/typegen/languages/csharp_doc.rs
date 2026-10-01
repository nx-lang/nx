//! Converts NX documentation, which is CommonMark, to a C# XML documentation comment.
//!
//! <para>The target elements are the ones Visual Studio and Rider tooltips render: `<b>`, `<i>`,
//! `<c>`, `<code>`, `<a href>`, `<list>`, and `<para>`. Markdown with no counterpart among them — a
//! table, an image, raw HTML, a block quote — is kept as the text the author wrote. Every piece of
//! text is XML-escaped, so no documentation can produce an element this mapping does not.</para>

use pulldown_cmark::{Event, Options, Parser, Tag, TagEnd};
use std::ops::Range;

/// The lines of the C# XML documentation comment for `markdown`, each starting `///`.
///
/// <para>The first block is the `<summary>`. Any further blocks are the `<remarks>`, each in its
/// own `<para>` when there is more than one.</para>
pub(crate) fn csharp_doc_lines(markdown: &str) -> Vec<String> {
    let blocks = Blocks::read(markdown);
    let Some((summary, remarks)) = blocks.split_first() else {
        return Vec::new();
    };

    let mut lines = Vec::new();
    element(
        &mut lines,
        "summary",
        &summary.lines(false),
        summary.is_text(),
    );
    let (remarks, inline) = match remarks {
        [] => (Vec::new(), false),
        [only] => (only.lines(false), only.is_text()),
        several => (
            several.iter().flat_map(|block| block.lines(true)).collect(),
            false,
        ),
    };
    element(&mut lines, "remarks", &remarks, inline);

    lines
        .into_iter()
        .map(|line| {
            if line.is_empty() {
                "///".to_string()
            } else {
                format!("/// {line}")
            }
        })
        .collect()
}

/// One top-level Markdown block, rendered to XML with `\n` between its lines.
enum Block {
    Paragraph(String),
    Heading(String),
    Code(String),
    List {
        ordered: bool,
        items: Vec<String>,
    },
    /// A block with no XML counterpart, as its escaped source text.
    Written(String),
}

impl Block {
    /// Whether the block renders as a run of text rather than as an element of its own.
    fn is_text(&self) -> bool {
        matches!(self, Block::Paragraph(_) | Block::Written(_))
    }

    /// The block's lines. `in_paragraphs` is set when it is one of several blocks, where a
    /// paragraph of text takes its own `<para>`.
    fn lines(&self, in_paragraphs: bool) -> Vec<String> {
        let mut lines = Vec::new();
        match self {
            Block::Paragraph(text) | Block::Written(text) if in_paragraphs => {
                element(&mut lines, "para", &split(text), true);
            }
            Block::Paragraph(text) | Block::Written(text) => lines.extend(split(text)),
            Block::Heading(text) => {
                let mut bold = split(text);
                if let Some(first) = bold.first_mut() {
                    first.insert_str(0, "<b>");
                }
                if let Some(last) = bold.last_mut() {
                    last.push_str("</b>");
                }
                element(&mut lines, "para", &bold, true);
            }
            Block::Code(code) => {
                lines.push("<code>".to_string());
                lines.extend(code.split('\n').map(str::to_string));
                lines.push("</code>".to_string());
            }
            Block::List { ordered, items } => {
                let kind = if *ordered { "number" } else { "bullet" };
                lines.push(format!("<list type=\"{kind}\">"));
                for item in items {
                    let mut description = split(item);
                    if let Some(first) = description.first_mut() {
                        first.insert_str(0, "<item><description>");
                    }
                    if let Some(last) = description.last_mut() {
                        last.push_str("</description></item>");
                    }
                    lines.extend(description);
                }
                lines.push("</list>".to_string());
            }
        }
        lines
    }
}

/// `<tag>body</tag>` on one line when the body is one line of text, or else the tags on lines of
/// their own. `inline` says the body is text rather than elements such as a `<list>`.
fn element(lines: &mut Vec<String>, tag: &str, body: &[String], inline: bool) {
    match body {
        [] => {}
        [only] if inline => lines.push(format!("<{tag}>{only}</{tag}>")),
        _ => {
            lines.push(format!("<{tag}>"));
            lines.extend(body.iter().cloned());
            lines.push(format!("</{tag}>"));
        }
    }
}

fn split(text: &str) -> Vec<String> {
    text.trim_end()
        .split('\n')
        .map(|line| line.trim_end().to_string())
        .collect()
}

/// The event stream of one Markdown text, read into blocks.
struct Blocks<'a> {
    source: &'a str,
    events: Vec<(Event<'a>, Range<usize>)>,
    next: usize,
}

impl<'a> Blocks<'a> {
    fn read(source: &'a str) -> Vec<Block> {
        let mut reader = Blocks {
            source,
            events: Parser::new_ext(source, Options::empty())
                .into_offset_iter()
                .collect(),
            next: 0,
        };
        let mut blocks = Vec::new();
        while let Some((event, range)) = reader.take() {
            let block = match event {
                Event::Start(Tag::Paragraph) => Block::Paragraph(reader.inline(TagEnd::Paragraph)),
                Event::Start(Tag::Heading { level, .. }) => {
                    Block::Heading(reader.inline(TagEnd::Heading(level)))
                }
                Event::Start(Tag::CodeBlock(_)) => Block::Code(reader.code_block()),
                Event::Start(Tag::List(start)) => Block::List {
                    ordered: start.is_some(),
                    items: reader.list_items(),
                },
                Event::Start(_) => {
                    reader.skip_to_end();
                    Block::Written(escape(source[range].trim_end()))
                }
                Event::Rule | Event::Html(_) | Event::Text(_) => {
                    Block::Written(escape(source[range].trim_end()))
                }
                _ => continue,
            };
            blocks.push(block);
        }
        blocks
    }

    fn take(&mut self) -> Option<(Event<'a>, Range<usize>)> {
        let taken = self.events.get(self.next).cloned();
        self.next += 1;
        taken
    }

    /// Skips past the end of the element whose start was just taken.
    fn skip_to_end(&mut self) {
        let mut depth = 1;
        while let Some((event, _)) = self.take() {
            match event {
                Event::Start(_) => depth += 1,
                Event::End(_) => {
                    depth -= 1;
                    if depth == 0 {
                        return;
                    }
                }
                _ => {}
            }
        }
    }

    /// Renders inline content up to and past `end`.
    fn inline(&mut self, end: TagEnd) -> String {
        let mut out = String::new();
        while let Some((event, range)) = self.take() {
            match event {
                Event::End(tag) if tag == end => break,
                Event::Text(text) => out.push_str(&escape(&text)),
                Event::Code(code) => {
                    out.push_str("<c>");
                    out.push_str(&escape(&code));
                    out.push_str("</c>");
                }
                Event::SoftBreak | Event::HardBreak => out.push('\n'),
                Event::Start(Tag::Emphasis) => out.push_str("<i>"),
                Event::End(TagEnd::Emphasis) => out.push_str("</i>"),
                Event::Start(Tag::Strong) => out.push_str("<b>"),
                Event::End(TagEnd::Strong) => out.push_str("</b>"),
                Event::Start(Tag::Link { dest_url, .. }) => {
                    out.push_str(&format!("<a href=\"{}\">", escape_attribute(&dest_url)));
                }
                Event::End(TagEnd::Link) => out.push_str("</a>"),
                // A loose list item holds paragraphs; they run on as lines of one description.
                Event::Start(Tag::Paragraph) => {
                    if !out.is_empty() && !out.ends_with('\n') {
                        out.push('\n');
                    }
                }
                Event::End(TagEnd::Paragraph) => {}
                Event::Start(_) => {
                    self.skip_to_end();
                    out.push_str(&escape(self.source[range].trim_end()));
                }
                Event::InlineHtml(html) | Event::Html(html) => out.push_str(&escape(&html)),
                _ => out.push_str(&escape(&self.source[range])),
            }
        }
        out
    }

    fn code_block(&mut self) -> String {
        let mut code = String::new();
        while let Some((event, _)) = self.take() {
            match event {
                Event::End(TagEnd::CodeBlock) => break,
                Event::Text(text) => code.push_str(&text),
                _ => {}
            }
        }
        escape(code.trim_end_matches('\n'))
    }

    fn list_items(&mut self) -> Vec<String> {
        let mut items = Vec::new();
        while let Some((event, _)) = self.take() {
            match event {
                Event::Start(Tag::Item) => items.push(self.inline(TagEnd::Item)),
                Event::End(TagEnd::List(_)) => break,
                _ => {}
            }
        }
        items
    }
}

fn escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

fn escape_attribute(text: &str) -> String {
    escape(text).replace('"', "&quot;")
}

#[cfg(test)]
mod tests {
    use super::csharp_doc_lines;

    fn doc(markdown: &str) -> String {
        csharp_doc_lines(markdown).join("\n")
    }

    #[test]
    fn a_single_line_is_a_one_line_summary() {
        assert_eq!(doc("Hint text."), "/// <summary>Hint text.</summary>");
    }

    #[test]
    fn nothing_writes_no_comment() {
        assert!(csharp_doc_lines("").is_empty());
    }

    #[test]
    fn a_multi_line_summary_puts_the_tags_on_their_own_lines() {
        assert_eq!(
            doc("Hint text\nshown while empty."),
            "/// <summary>\n/// Hint text\n/// shown while empty.\n/// </summary>"
        );
    }

    #[test]
    fn a_second_paragraph_is_the_remarks() {
        assert_eq!(
            doc("A contact.\n\nShown in lists."),
            "/// <summary>A contact.</summary>\n/// <remarks>Shown in lists.</remarks>"
        );
    }

    #[test]
    fn several_remarks_blocks_each_take_a_para() {
        assert_eq!(
            doc("A contact.\n\nShown in lists.\n\nSorted by name."),
            concat!(
                "/// <summary>A contact.</summary>\n",
                "/// <remarks>\n",
                "/// <para>Shown in lists.</para>\n",
                "/// <para>Sorted by name.</para>\n",
                "/// </remarks>",
            )
        );
    }

    #[test]
    fn emphasis_becomes_b_and_i() {
        assert_eq!(
            doc("Use **only** *once*."),
            "/// <summary>Use <b>only</b> <i>once</i>.</summary>"
        );
    }

    #[test]
    fn a_code_span_becomes_c() {
        assert_eq!(
            doc("Pass `Grid`."),
            "/// <summary>Pass <c>Grid</c>.</summary>"
        );
    }

    #[test]
    fn a_link_with_a_destination_becomes_a_href() {
        let doc = doc("Use **only** with `Grid`; see [the guide](https://nxlang.org/guide).");
        assert!(doc.contains("<b>only</b>"), "{doc}");
        assert!(doc.contains("<c>Grid</c>"), "{doc}");
        assert!(
            doc.contains("<a href=\"https://nxlang.org/guide\">the guide</a>"),
            "{doc}"
        );
        assert!(!doc.contains("**"), "{doc}");
    }

    #[test]
    fn a_fenced_code_block_becomes_code() {
        assert_eq!(
            doc("Example:\n\n```nx\nlet x = <A />\n\nlet y = 1\n```"),
            concat!(
                "/// <summary>Example:</summary>\n",
                "/// <remarks>\n",
                "/// <code>\n",
                "/// let x = &lt;A /&gt;\n",
                "///\n",
                "/// let y = 1\n",
                "/// </code>\n",
                "/// </remarks>",
            )
        );
    }

    #[test]
    fn an_indented_code_block_becomes_code() {
        assert!(doc("Example:\n\n    let x = 1").contains("/// <code>\n/// let x = 1\n/// </code>"));
    }

    #[test]
    fn a_bulleted_list_becomes_a_bullet_list() {
        assert_eq!(
            doc("Modes.\n\n- fill\n- cover"),
            concat!(
                "/// <summary>Modes.</summary>\n",
                "/// <remarks>\n",
                "/// <list type=\"bullet\">\n",
                "/// <item><description>fill</description></item>\n",
                "/// <item><description>cover</description></item>\n",
                "/// </list>\n",
                "/// </remarks>",
            )
        );
    }

    #[test]
    fn a_list_directly_after_the_first_paragraph_ends_the_summary() {
        assert_eq!(
            doc("Modes:\n- fill\n- cover"),
            doc("Modes:\n\n- fill\n- cover")
        );
    }

    #[test]
    fn a_numbered_list_becomes_a_number_list() {
        let doc = doc("Steps.\n\n1. Load *it*\n2. Show it");
        assert!(doc.contains("<list type=\"number\">"), "{doc}");
        assert!(
            doc.contains("<item><description>Load <i>it</i></description></item>"),
            "{doc}"
        );
    }

    #[test]
    fn a_heading_becomes_a_bold_para() {
        assert_eq!(
            doc("Summary.\n\n# Details\n\nMore."),
            concat!(
                "/// <summary>Summary.</summary>\n",
                "/// <remarks>\n",
                "/// <para><b>Details</b></para>\n",
                "/// <para>More.</para>\n",
                "/// </remarks>",
            )
        );
    }

    #[test]
    fn text_is_xml_escaped() {
        assert_eq!(
            doc("True when a < b & c."),
            "/// <summary>True when a &lt; b &amp; c.</summary>"
        );
        assert!(doc("Use `a<b>`.").contains("<c>a&lt;b&gt;</c>"));
    }

    #[test]
    fn raw_html_is_kept_as_text() {
        let doc = doc("Line one<br>line two.");
        assert!(doc.contains("&lt;br&gt;"), "{doc}");
        assert!(!doc.contains("<br>"), "{doc}");

        let block = csharp_doc_lines("Summary.\n\n<div>\nraw\n</div>").join("\n");
        assert!(block.contains("&lt;div&gt;"), "{block}");
        assert!(!block.contains("<div>"), "{block}");
    }

    #[test]
    fn an_image_and_a_block_quote_are_kept_as_written() {
        let image = doc("See ![the logo](logo.png).");
        assert!(image.contains("See ![the logo](logo.png)."), "{image}");

        let quote = doc("Summary.\n\n> Quoted *text*.");
        assert!(quote.contains("&gt; Quoted *text*."), "{quote}");
    }

    #[test]
    fn a_link_destination_is_escaped_as_an_attribute() {
        assert!(doc("See [it](https://x.test/?a=1&b=\"2\").")
            .contains("<a href=\"https://x.test/?a=1&amp;b=&quot;2&quot;\">it</a>"));
    }
}
