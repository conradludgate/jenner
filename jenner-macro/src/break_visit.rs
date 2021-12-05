use syn::{
    parse_quote,
    visit_mut::{visit_expr_break_mut, visit_expr_mut, visit_item_mut, VisitMut},
    Label,
};

pub struct BreakVisitor<'f> {
    pub label: &'f Option<Label>,
    pub outside: bool,
    pub breaks: usize,
}

impl<'f> VisitMut for BreakVisitor<'f> {
    fn visit_expr_break_mut(&mut self, i: &mut syn::ExprBreak) {
        visit_expr_break_mut(self, i);
        if !self.outside // not outside of the original for scope
            || i.label
                .as_ref()
                .zip(self.label.as_ref())
                .map_or(false, |(a, b)| *a == b.name)
        // or breaking the specific label
        {
            self.breaks += 1;
            let expr = i.expr.get_or_insert_with(|| Box::new(parse_quote! { () }));
            *expr = parse_quote! { ::jenner::ForResult::Break(#expr) };
        }
    }

    fn visit_expr_mut(&mut self, i: &mut syn::Expr) {
        match i {
            // don't propagate search through closures
            syn::Expr::Closure(_) => {}

            // propagate through other loops
            // but make sure the break context is different
            syn::Expr::ForLoop(_) | syn::Expr::Loop(_) | syn::Expr::While(_) => {
                let store = self.outside;
                self.outside = true;

                visit_expr_mut(self, i);

                self.outside = store;
            }

            // propagate as normal
            i => visit_expr_mut(self, i),
        }
    }

    fn visit_item_mut(&mut self, i: &mut syn::Item) {
        match i {
            syn::Item::Fn(_)
            | syn::Item::ForeignMod(_)
            | syn::Item::Impl(_)
            | syn::Item::Macro(_)
            | syn::Item::Macro2(_)
            | syn::Item::Mod(_)
            | syn::Item::Trait(_) => {}

            // propagate as normal
            i => visit_item_mut(self, i),
        }
    }
}
