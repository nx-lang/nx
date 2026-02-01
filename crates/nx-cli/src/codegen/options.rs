use crate::codegen::TargetLanguage;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BraceStyle {
    Allman,
    KAndR,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IndentStyle {
    Tabs,
    Spaces,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NewlineStyle {
    Lf,
    CrLf,
}

#[derive(Clone, Debug)]
pub struct FormatOptions {
    pub indent_style: IndentStyle,
    pub indent_size: usize,
    pub brace_style: BraceStyle,
    pub newline_style: NewlineStyle,
}

impl FormatOptions {
    fn default_newline_style() -> NewlineStyle {
        if cfg!(windows) {
            NewlineStyle::CrLf
        } else {
            NewlineStyle::Lf
        }
    }

    pub fn defaults_for(language: TargetLanguage) -> Self {
        let newline_style = Self::default_newline_style();
        match language {
            TargetLanguage::CSharp => Self {
                indent_style: IndentStyle::Spaces,
                indent_size: 4,
                brace_style: BraceStyle::Allman,
                newline_style,
            },
            TargetLanguage::TypeScript => Self {
                indent_style: IndentStyle::Spaces,
                indent_size: 2,
                brace_style: BraceStyle::KAndR,
                newline_style,
            },
        }
    }

    pub fn newline_str(&self) -> &'static str {
        match self.newline_style {
            NewlineStyle::Lf => "\n",
            NewlineStyle::CrLf => "\r\n",
        }
    }

    pub fn indent_unit(&self) -> String {
        match self.indent_style {
            IndentStyle::Tabs => "\t".to_string(),
            IndentStyle::Spaces => " ".repeat(self.indent_size),
        }
    }
}
