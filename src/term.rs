use crate::typ::Type;
use crate::kind::Kind;
use crate::scope::{Scope,ScopeId};
use crate::tlc::TLC;
use crate::constant::Constant;
use std::collections::HashMap;

#[derive(Clone,Copy,Eq,PartialEq,Ord,PartialOrd,Hash)]
pub struct TermId {
   pub id: usize,
}

#[derive(Clone)]
pub struct LetTerm {
   pub scope: ScopeId,
   pub name: String,
   pub parameters: Vec<Vec<(Option<String>,Option<Type>,Kind)>>,
   pub body: Option<TermId>,
   pub rtype: Type,
   pub rkind: Kind,
}

//does not implement Clone because terms are uniquely identified by their id
#[derive(Clone)] //clone seems to be needed to deconflict mutable borrows :(
pub enum Term {
   Ident(String),
   Value(String),
   Arrow(TermId,TermId),
   App(TermId,TermId),
   Let(LetTerm),
   Tuple(Vec<TermId>),
   Block(ScopeId,Vec<TermId>),
   Ascript(TermId,Type),
   As(TermId,Type),
   Constructor(String,Vec<(String,TermId)>),
   Substitution(TermId,TermId,TermId),
   RuleApplication(TermId,String),
   Literal(TermId),
   Match(
      TermId,
      Vec<(TermId,TermId)>, //lhs's here don't need scopes because these bindings can't be polymorphic
   ),
}

impl Term {
   pub fn equals(tlc: &TLC, lt: TermId, rt: TermId) -> bool {
      match (&tlc.rows[lt.id].term, &tlc.rows[rt.id].term) {
         (Term::Ident(li), Term::Ident(ri)) => { li == ri },
         (Term::Value(lv), Term::Value(rv)) => { lv == rv },
         (Term::Arrow(lp,lb), Term::Arrow(rp,rb)) => {
            Term::equals(tlc, *lp, *rp) &&
            Term::equals(tlc, *lb, *rb)
         },
         (Term::App(lp,lb), Term::App(rp,rb)) => {
            Term::equals(tlc, *lp, *rp) &&
            Term::equals(tlc, *lb, *rb)
         },
         (Term::Tuple(ls), Term::Tuple(rs)) => {
            if ls.len() != rs.len() { return false; }
            for (lt, rt) in std::iter::zip(ls, rs) {
            if !Term::equals(tlc, *lt, *rt) {
               return false;
            }}
            true
         },
         _ => false
      }
   }
   pub fn reduce_lhs(tlc: &TLC, scope_constants: &mut HashMap<String,Constant>, lhs: TermId, dc: &Constant) -> bool {
      match &tlc.rows[lhs.id].term {
         Term::Ident(n) => {
            if n != "_" {
               scope_constants.insert(n.clone(), dc.clone());
            };
            true
         },
         Term::Value(lv) => {
            if let Some(lc) = Constant::parse(tlc, lv) {
               &lc == dc
            } else { false }
         },
         _ => unimplemented!("Term::reduce_lhs({})", tlc.print_term(lhs))
      }
   }
   pub fn reduce(tlc: &TLC, scope: &Option<ScopeId>, scope_constants: &HashMap<String,Constant>, term: TermId) -> Option<Constant> {
      //scope is only used to look up functions
      //all other variables should already be converted to values
      match &tlc.rows[term.id].term {
         Term::Value(v) => {
            Constant::parse(tlc, &v)
         },
         Term::Constructor(c,cps) if cps.len()==0 => {
            Constant::parse(tlc, &c)
         },
         Term::Tuple(ts) => {
            let mut cs = Vec::new();
            for ct in ts.iter() {
               if let Some(cc) = Term::reduce(tlc, scope, scope_constants, *ct) {
                  cs.push(cc);
               } else { return None; }
            }
            Some(Constant::Tuple(cs))
         },
         Term::App(g,x) => {
            if let Some(xc) = Term::reduce(tlc, scope, scope_constants, *x) {
               let sc = if let Some(sc) = scope { *sc } else { return None; };
               match &tlc.rows[g.id].term {
                  Term::Ident(gv) => {
                     if let Some(binding) = Scope::lookup_term(tlc, sc, gv, &tlc.rows[x.id].typ) {
                        if let Term::Let(lb) = &tlc.rows[binding.id].term {
                           if lb.parameters.len() != 1 { unimplemented!("Term::reduce, beta-reduce curried functions") }
                           let mut new_scope = scope_constants.clone();
                           let ref pars = lb.parameters[0];
                           let args = if pars.len()==1 { vec![xc] }
                                 else if let Constant::Tuple(xs) = xc { xs.clone() }
                                 else { vec![xc] };
                           if pars.len() != args.len() { panic!("Term::reduce, mismatched arity {}", tlc.print_term(term)) };
                           for ((pn,_pt,_pk),a) in std::iter::zip(pars,args) {
                              if let Some(pn) = pn {
                                 new_scope.insert(pn.clone(), a.clone());
                              }
                           }
                           if let Some(body) = lb.body {
                              Term::reduce(tlc, &Some(lb.scope), &new_scope, body)
                           } else { return None; }
                        } else {
                           panic!("Term::reduce, unexpected lambda format in beta-reduction {}", tlc.print_term(binding))
                        }
                     } else { return None; }
                  },
                  _ => unimplemented!("Term::reduce, implement Call-by-Value function call: {}({:?})", tlc.print_term(*g), xc)
               }
            } else { return None; }
         },
         Term::Literal(v) => {
            Constant::eval(tlc, scope_constants, *v)
         },
         Term::Match(dv,lrs) => {
            if let Some(ref dc) = Constant::eval(tlc, scope_constants, *dv) {
               for (l,r) in lrs.iter() {
                  let mut sc = scope_constants.clone();
                  if Term::reduce_lhs(tlc, &mut sc, *l, dc) {
                     return Term::reduce(tlc, scope, &sc, *r);
                  }
               }
               panic!("Term::reduce, pattern was not total: {}", tlc.print_term(term))
            } else { None }
         },
         _ => unimplemented!("Term::reduce, implement Call-by-Value term reduction: {}", tlc.print_term(term))
      }
   }
}
