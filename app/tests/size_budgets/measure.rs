//! Measures one source file: its code lines, each function's code lines and parameters, each struct's fields,
//! each type's public members, and the size-baseline attributes.

use crate::budget::{Kind, Measures};
use crate::lines::{block_text, code_lines};
use std::collections::BTreeMap;
use syn::punctuated::Punctuated;
use syn::visit::{self, Visit};
use syn::{Attribute, Block, Expr, ImplItem, Lit, Meta, Signature, Token, Type, Visibility};

/// The budget lints a suppression must not silence outside the baseline.
const BUDGET_LINTS: [&str; 5] = [
    "too_many_lines",
    "too_many_arguments",
    "excessive_nesting",
    "cognitive_complexity",
    "type_complexity",
];

/// What one file holds; `suppressions` names every budget lint silenced without the size-baseline reason.
pub struct FileMeasures {
    pub measures: Measures,
    pub code_lines: usize,
    pub suppressions: Vec<String>,
}

/// Measures the file at workspace-relative `path` holding `source`.
pub fn measure_source(path: &str, source: &str) -> FileMeasures {
    let file = syn::parse_file(source).unwrap_or_else(|e| panic!("{path}: cannot parse: {e}"));
    let mut visitor = Visitor {
        source,
        scope: vec![path.to_owned()],
        in_trait_impl: false,
        measures: Measures::default(),
        public: BTreeMap::new(),
        suppressions: Vec::new(),
    };
    visitor.check_suppressions(&file.attrs);
    visitor.visit_file(&file);
    let lines = code_lines(source);
    visitor
        .measures
        .record(Kind::FileLines, path.to_owned(), lines);
    for (id, count) in std::mem::take(&mut visitor.public) {
        visitor.measures.record(Kind::PublicMembers, id, count);
    }
    FileMeasures {
        measures: visitor.measures,
        code_lines: lines,
        suppressions: visitor.suppressions,
    }
}

struct Visitor<'a> {
    source: &'a str,
    scope: Vec<String>,
    in_trait_impl: bool,
    measures: Measures,
    public: BTreeMap<String, usize>,
    suppressions: Vec<String>,
}

impl Visitor<'_> {
    fn id(&self, name: &str) -> String {
        format!("{}::{name}", self.scope.join("::"))
    }

    fn function(&mut self, attrs: &[Attribute], sig: &Signature, block: &Block) {
        let id = self.id(&sig.ident.to_string());
        let span = block.brace_token.span;
        let async_fn = sig.asyncness.is_some();
        let body = block_text(
            self.source,
            span.open().start(),
            span.close().start(),
            async_fn,
        );
        self.measures
            .record(Kind::FunctionLines, id.clone(), code_lines(&body));
        if !self.in_trait_impl {
            self.measures
                .record(Kind::Parameters, id.clone(), sig.inputs.len());
        }
        for kind in size_baseline_kinds(attrs) {
            self.measures.markers.insert((kind, id.clone()));
        }
        self.check_suppressions(attrs);
        let in_trait_impl = std::mem::replace(&mut self.in_trait_impl, false);
        self.scope.push(sig.ident.to_string());
        visit::visit_block(self, block);
        self.scope.pop();
        self.in_trait_impl = in_trait_impl;
    }

    fn check_suppressions(&mut self, attrs: &[Attribute]) {
        for attr in attrs {
            let Some(lints) = suppressed_lints(attr) else {
                continue;
            };
            for lint in lints
                .iter()
                .filter(|lint| BUDGET_LINTS.contains(&lint.as_str()))
            {
                self.suppressions
                    .push(format!("{}: clippy::{lint}", self.scope.join("::")));
            }
        }
    }

    fn count_public(&mut self, ty: &str, items: &[ImplItem]) {
        let public = items.iter().filter(|item| is_public_member(item)).count();
        *self.public.entry(self.id(ty)).or_insert(0) += public;
    }
}

impl<'ast> Visit<'ast> for Visitor<'_> {
    fn visit_item_mod(&mut self, item: &'ast syn::ItemMod) {
        self.check_suppressions(&item.attrs);
        self.scope.push(item.ident.to_string());
        visit::visit_item_mod(self, item);
        self.scope.pop();
    }

    fn visit_item_impl(&mut self, item: &'ast syn::ItemImpl) {
        self.check_suppressions(&item.attrs);
        let ty = type_name(&item.self_ty);
        let segment = if let Some((_, path, _)) = &item.trait_ {
            format!("<{ty} as {}>", last_segment(path))
        } else {
            self.count_public(&ty, &item.items);
            ty
        };
        let in_trait_impl = std::mem::replace(&mut self.in_trait_impl, item.trait_.is_some());
        self.scope.push(segment);
        visit::visit_item_impl(self, item);
        self.scope.pop();
        self.in_trait_impl = in_trait_impl;
    }

    fn visit_item_trait(&mut self, item: &'ast syn::ItemTrait) {
        self.scope.push(item.ident.to_string());
        visit::visit_item_trait(self, item);
        self.scope.pop();
    }

    fn visit_item_fn(&mut self, item: &'ast syn::ItemFn) {
        self.function(&item.attrs, &item.sig, &item.block);
    }

    fn visit_impl_item_fn(&mut self, item: &'ast syn::ImplItemFn) {
        self.function(&item.attrs, &item.sig, &item.block);
    }

    fn visit_trait_item_fn(&mut self, item: &'ast syn::TraitItemFn) {
        if let Some(block) = &item.default {
            self.function(&item.attrs, &item.sig, block);
        }
    }

    fn visit_item_struct(&mut self, item: &'ast syn::ItemStruct) {
        self.check_suppressions(&item.attrs);
        let id = self.id(&item.ident.to_string());
        self.measures
            .record(Kind::StructFields, id, item.fields.len());
        visit::visit_item_struct(self, item);
    }
}

fn is_public_member(item: &ImplItem) -> bool {
    let vis = match item {
        ImplItem::Fn(f) => &f.vis,
        ImplItem::Const(c) => &c.vis,
        _ => return false,
    };
    !matches!(vis, Visibility::Inherited)
}

fn type_name(ty: &Type) -> String {
    match ty {
        Type::Path(path) => last_segment(&path.path),
        Type::Reference(reference) => type_name(&reference.elem),
        _ => "_".to_owned(),
    }
}

fn last_segment(path: &syn::Path) -> String {
    path.segments
        .last()
        .map(|s| s.ident.to_string())
        .unwrap_or_default()
}

/// The arguments of an `#[allow(…)]` or `#[expect(…)]`, `None` for any other attribute.
fn lint_arguments(attr: &Attribute) -> Option<Punctuated<Meta, Token![,]>> {
    let path = attr.path();
    if !path.is_ident("allow") && !path.is_ident("expect") {
        return None;
    }
    attr.parse_args_with(Punctuated::<Meta, Token![,]>::parse_terminated)
        .ok()
}

fn clippy_lints(metas: &Punctuated<Meta, Token![,]>) -> Vec<String> {
    metas
        .iter()
        .filter_map(|meta| match meta {
            Meta::Path(path) if path.segments.len() == 2 && path.segments[0].ident == "clippy" => {
                Some(path.segments[1].ident.to_string())
            }
            _ => None,
        })
        .collect()
}

fn is_size_baseline(metas: &Punctuated<Meta, Token![,]>) -> bool {
    metas.iter().any(|meta| match meta {
        Meta::NameValue(nv) if nv.path.is_ident("reason") => {
            matches!(&nv.value, Expr::Lit(lit) if matches!(&lit.lit, Lit::Str(s) if s.value() == "size baseline"))
        }
        _ => false,
    })
}

/// The kinds a function's `#[expect(clippy::…, reason = "size baseline")]` attributes list it under.
fn size_baseline_kinds(attrs: &[Attribute]) -> Vec<Kind> {
    attrs
        .iter()
        .filter(|attr| attr.path().is_ident("expect"))
        .filter_map(lint_arguments)
        .filter(is_size_baseline)
        .flat_map(|metas| clippy_lints(&metas))
        .filter_map(|lint| Kind::from_lint(&lint))
        .collect()
}

/// The clippy lints an attribute silences; a size-baseline `expect` silences only the lints it may not list.
fn suppressed_lints(attr: &Attribute) -> Option<Vec<String>> {
    let metas = lint_arguments(attr)?;
    let size_baseline = attr.path().is_ident("expect") && is_size_baseline(&metas);
    let lints = clippy_lints(&metas).into_iter();
    Some(
        lints
            .filter(|lint| !size_baseline || Kind::from_lint(lint).is_none())
            .collect(),
    )
}
