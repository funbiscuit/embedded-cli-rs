//! Code is mostly taken from clap/structopt
//!
//! Works in terms of "paragraphs". Paragraph is a sequence of
//! non-empty adjacent lines, delimited by sequences of blank (whitespace only) lines.

use std::iter;

pub struct Help {
    short: Option<String>,
    long: Option<String>,
}

impl Help {
    pub fn parse(attrs: &[syn::Attribute]) -> darling::Result<Self> {
        let lines = extract_doc_comment(attrs);

        let (short, long) = format_doc_comment(&lines);

        Ok(Self { short, long })
    }

    pub fn long(&self) -> Option<&str> {
        self.long.as_deref()
    }

    pub fn short(&self) -> Option<&str> {
        self.short.as_deref()
    }
}

fn extract_doc_comment(attrs: &[syn::Attribute]) -> Vec<String> {
    // multiline comments (`/** ... */`) may have LFs (`\n`) in them,
    // we need to split so we could handle the lines correctly
    //
    // we also need to remove leading and trailing blank lines
    let mut lines: Vec<_> = attrs
        .iter()
        .filter(|attr| attr.path().is_ident("doc"))
        .filter_map(|attr| {
            // non #[doc = "..."] attributes are not our concern
            // we leave them for rustc to handle
            match &attr.meta {
                syn::Meta::NameValue(syn::MetaNameValue {
                    value:
                        syn::Expr::Lit(syn::ExprLit {
                            lit: syn::Lit::Str(s),
                            ..
                        }),
                    ..
                }) => Some(s.value()),
                _ => None,
            }
        })
        .skip_while(|s| is_blank(s))
        .flat_map(|s| {
            let lines = s
                .split('\n')
                .map(|s| {
                    // remove one leading space no matter what
                    let s = s.strip_prefix(' ').unwrap_or(s);
                    s.to_owned()
                })
                .collect::<Vec<_>>();
            lines
        })
        .collect();

    while let Some(true) = lines.last().map(|s| is_blank(s)) {
        lines.pop();
    }

    lines
}

fn format_doc_comment(lines: &[String]) -> (Option<String>, Option<String>) {
    if lines.is_empty() {
        return (None, None);
    }
    if lines.iter().any(|s| is_blank(s)) {
        let paragraphs = split_paragraphs(lines);
        let short = paragraphs[0].clone();
        let long = paragraphs.join("\r\n\r\n");
        (Some(remove_period(short)), Some(long))
    } else {
        let short = merge_lines(lines);
        let long = short.clone();
        let short = remove_period(short);
        (Some(short), Some(long))
    }
}

fn split_paragraphs(lines: &[String]) -> Vec<String> {
    let mut last_line = 0;
    iter::from_fn(|| {
        let slice = &lines[last_line..];
        let start = slice.iter().position(|s| !is_blank(s)).unwrap_or(0);

        let slice = &slice[start..];
        let len = slice
            .iter()
            .position(|s| is_blank(s))
            .unwrap_or(slice.len());

        last_line += start + len;

        if len != 0 {
            Some(merge_lines(&slice[..len]))
        } else {
            None
        }
    })
    .collect()
}

fn remove_period(mut s: String) -> String {
    if s.ends_with('.') && !s.ends_with("..") {
        s.pop();
    }
    s
}

fn is_blank(s: &str) -> bool {
    s.trim().is_empty()
}

fn merge_lines(lines: impl IntoIterator<Item = impl AsRef<str>>) -> String {
    lines
        .into_iter()
        .map(|s| s.as_ref().trim().to_owned())
        .collect::<Vec<_>>()
        .join(" ")
}
