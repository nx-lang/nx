use crate::codegen::options::{BraceStyle, FormatOptions};

#[derive(Clone, Debug)]
pub struct CodeWriter {
    out: String,
    indent_level: usize,
    opts: FormatOptions,
}

impl CodeWriter {
    pub fn new(opts: FormatOptions) -> Self {
        Self {
            out: String::new(),
            indent_level: 0,
            opts,
        }
    }

    pub fn finish(self) -> String {
        self.out
    }

    pub fn blank_line(&mut self) {
        self.out.push_str(self.opts.newline_str());
    }

    pub fn line(&mut self, text: &str) {
        let indent = self.opts.indent_unit();
        for _ in 0..self.indent_level {
            self.out.push_str(&indent);
        }
        self.out.push_str(text);
        self.out.push_str(self.opts.newline_str());
    }

    pub fn indent(&mut self) {
        self.indent_level += 1;
    }

    pub fn dedent(&mut self) {
        self.indent_level = self.indent_level.saturating_sub(1);
    }

    pub fn block<F>(&mut self, header: &str, body: F)
    where
        F: FnOnce(&mut Self),
    {
        match self.opts.brace_style {
            BraceStyle::Allman => {
                self.line(header);
                self.line("{");
            }
            BraceStyle::KAndR => {
                self.line(&format!("{header} {{"));
            }
        }

        self.indent();
        body(self);
        self.dedent();

        self.line("}");
    }
}
