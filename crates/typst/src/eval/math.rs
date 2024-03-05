use ecow::eco_format;

use crate::diag::{At, SourceResult};
use crate::eval::{Eval, Vm};
use crate::foundations::{Content, NativeElement, Value};
use crate::math::{AlignPointElem, AttachElem, FracElem, LrElem, PrimesElem, RootElem};
use crate::syntax::ast::{self, AstNode, Expr};
use crate::text::TextElem;

impl Eval for ast::Math<'_> {
    type Output = Content;
    fn eval(self, vm: &mut Vm) -> SourceResult<Self::Output> {
        Ok(Content::sequence(
            self.exprs()
                .map(|expr| expr.eval_display(vm))
                .collect::<SourceResult<Vec<_>>>()?,
        ))
    }
}


fn _runtime_math_parse<'a>(math_node: ast::Math<'a>, _vm: &mut Vm) -> SourceResult<Content> {
    let node = math_node.syntax_node();
    let _kids = node.children().filter_map(Expr::cast_with_space);

    // the plan:
    // - split up the math node's children and operate over their indices
    // - create a pratt parser function that gets nodes
    // - parse out the different types of bois
    //   - basic content
    //   - LR paren groupings
    //   - (don't forget to fix the shorthand `[|` `|]` symbols)
    //   - field access through identifiers (this one might be rough)
    //   - function calls
    //   - fractions (and yeeting parentheses around them)
    //   - attachments: including sub/super-script and primes (they join as superscripts)
    //   - roots as prefix operators
    //   -
    //   -
    //   - stop at an exclamation mark to separate factorials
    //   - spaces, need to explicitly handle spaces everywhere
    // - return a big ole vector or iterator of content
    // - then stuff it into a SourceResult

    unreachable!()
}

impl Eval for ast::Opening<'_> {
    type Output = Content;

    fn eval(self, _vm: &mut Vm) -> SourceResult<Self::Output> {
        // still textelems for now
        Ok(TextElem::packed(self.delim().clone()))
    }
}

impl Eval for ast::Closing<'_> {
    type Output = Content;

    fn eval(self, _vm: &mut Vm) -> SourceResult<Self::Output> {
        // still textelems for now
        Ok(TextElem::packed(self.delim().clone()))
    }
}

impl Eval for ast::MathIdent<'_> {
    type Output = Value;

    fn eval(self, vm: &mut Vm) -> SourceResult<Self::Output> {
        vm.scopes.get_in_math(&self).cloned().at(self.span())
    }
}

impl Eval for ast::MathAlignPoint<'_> {
    type Output = Content;

    fn eval(self, _: &mut Vm) -> SourceResult<Self::Output> {
        Ok(AlignPointElem::new().pack())
    }
}

// Note: Single characters like $f(x)$ always parse as "Text" plus delimiting the next thing
// we only care about multi-character values like $pi(x)$ where it does parse as FuncCall

impl Eval for ast::MathDelimited<'_> {
    type Output = Content;

    fn eval(self, vm: &mut Vm) -> SourceResult<Self::Output> {
        let open = self.open().eval_display(vm)?;
        let body = self.body().eval(vm)?;
        let close = self.close().eval_display(vm)?;
        Ok(LrElem::new(open + body + close).pack())
    }
}

impl Eval for ast::MathAttach<'_> {
    type Output = Content;

    fn eval(self, vm: &mut Vm) -> SourceResult<Self::Output> {
        let base = self.base().eval_display(vm)?;
        let mut elem = AttachElem::new(base);

        if let Some(expr) = self.top() {
            elem.push_t(Some(expr.eval_display(vm)?));
        } else if let Some(primes) = self.primes() {
            elem.push_tr(Some(primes.eval(vm)?));
        }

        if let Some(expr) = self.bottom() {
            elem.push_b(Some(expr.eval_display(vm)?));
        }

        Ok(elem.pack())
    }
}

impl Eval for ast::MathPrimes<'_> {
    type Output = Content;

    fn eval(self, _: &mut Vm) -> SourceResult<Self::Output> {
        Ok(PrimesElem::new(self.count()).pack())
    }
}

impl Eval for ast::MathFrac<'_> {
    type Output = Content;

    fn eval(self, vm: &mut Vm) -> SourceResult<Self::Output> {
        let num = self.num().eval_display(vm)?;
        let denom = self.denom().eval_display(vm)?;
        Ok(FracElem::new(num, denom).pack())
    }
}

impl Eval for ast::MathRoot<'_> {
    type Output = Content;

    fn eval(self, vm: &mut Vm) -> SourceResult<Self::Output> {
        let index = self.index().map(|i| TextElem::packed(eco_format!("{i}")));
        let radicand = self.radicand().eval_display(vm)?;
        Ok(RootElem::new(radicand).with_index(index).pack())
    }
}

trait ExprExt {
    fn eval_display(&self, vm: &mut Vm) -> SourceResult<Content>;
}

impl ExprExt for ast::Expr<'_> {
    fn eval_display(&self, vm: &mut Vm) -> SourceResult<Content> {
        Ok(self.eval(vm)?.display().spanned(self.span()))
    }
}


#[cfg(test)]
mod tests {

    // use super::*;
    use crate::{FileResult, Library, World};
    use crate::diag::FileError;
    use crate::eval::{eval_string, EvalMode};
    use crate::text::{Font, FontBook};
    use crate::syntax::{Span, Source, FileId};
    use crate::foundations::{Scope, Datetime, Bytes};
    use comemo::{Prehashed, Track};
    use ecow::EcoString;
    use once_cell::sync::Lazy;
    use include_dir::include_dir;

    /// A world for example compilations.
    struct MinimalWorld {
        source: Source,
        library: Prehashed<Library>,
    }

    static FONTS: Lazy<(Prehashed<FontBook>, Vec<Font>)> = Lazy::new(|| {
        let fonts: Vec<_> = include_dir!("$CARGO_MANIFEST_DIR/../../assets/fonts")
            .files()
            .flat_map(|file| Font::iter(file.contents().into()))
            .collect();
        let book = FontBook::from_fonts(&fonts);
        (Prehashed::new(book), fonts)
    });

    impl MinimalWorld {
        fn new(text: &str) -> Self {
            Self{
                source: Source::detached(text),
                library: Prehashed::new(Library::default()),
            }
        }
    }

    impl World for MinimalWorld {
        fn library(&self) -> &Prehashed<Library> {
            &self.library
        }

        fn book(&self) -> &Prehashed<FontBook> {
            &FONTS.0
        }

        fn main(&self) -> Source {
            self.source.clone()
        }

        fn source(&self, _id: FileId) -> FileResult<Source> {
            Ok(self.source.clone())
        }

        fn file(&self, _id: FileId) -> FileResult<Bytes> {
            Err(FileError::Other(Some(EcoString::from("No file result"))))
        }

        fn font(&self, index: usize) -> Option<Font> {
            Some(FONTS.1[index].clone())
        }

        fn today(&self, _: Option<i64>) -> Option<Datetime> {
            Some(Datetime::from_ymd(1970, 1, 1).unwrap())
        }
    }

    fn test(text: &str) {
        println!("Code:\n{text}");
        let world = MinimalWorld::new(text);
        let tracked = (&world as &dyn World).track();
        let src_result = eval_string(
            tracked, text, Span::detached(), EvalMode::Markup, Scope::default()
        );
        println!("Result:\n{:#?}", src_result);
    }

    #[test]
    fn ian_math() {
        // test("#let x = 5; #x");
        test("$a_b^' / c^d'_e(x)$");
    }
}
