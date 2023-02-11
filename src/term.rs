use crate::typ::Type;
use crate::kind::Kind;
use crate::scope::{Scope,ScopeId};
use crate::tlc::TLC;
use crate::constant::Constant;
use crate::debug::{Error};
use crate::token::{Span};
use std::collections::HashMap;
use l1_ir::value::Value;
use l1_ir::opt::{JProgram};
use l1_ir::ast::{self,Expression,Program,FunctionDefinition,LHSPart};

#[derive(Clone,Copy,Eq,PartialEq,Ord,PartialOrd,Hash)]
pub struct TermId {
   pub id: usize,
}

#[derive(Clone)]
pub struct LetTerm {
   pub is_extern: bool,
   pub scope: ScopeId,
   pub name: String,
   pub parameters: Vec<Vec<(Option<String>,Option<Type>,Kind)>>,
   pub body: Option<TermId>,
   pub rtype: Type,
   pub rkind: Kind,
}

#[derive(Clone)]
pub enum Literal {
   Var(String),
   Char(char,String),
   String(String,String),
   Range(Vec<(char,char)>,String),
}
impl std::fmt::Debug for Literal {
   fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      match self {
         Literal::Var(v)          => write!(f, "{}", v),
         Literal::Char(c,v)       => write!(f, "'{}'{}", c, v),
         Literal::String(s,v)     => write!(f, r#""{}"{}"#, s, v),
         Literal::Range(_r,v)     => write!(f, "[?]{}", v),
      }
   }
}

//does not implement Clone because terms are uniquely identified by their id
#[derive(Clone)] //clone seems to be needed to deconflict mutable borrows :(
pub enum Term {
   Ident(String),
   Value(String),
   Project(Constant),
   Arrow(Option<ScopeId>,TermId,Option<Type>,TermId),
   App(TermId,TermId),
   Let(LetTerm),
   Tuple(Vec<TermId>),
   Block(ScopeId,Vec<TermId>),
   Ascript(TermId,Type),
   As(TermId,Type),
   Constructor(String,Vec<(String,TermId)>),
   RuleApplication(TermId,String),
   Match(
      TermId,
      Vec<(ScopeId,TermId,TermId)>, //lhs's here don't need scopes because these bindings can't be polymorphic
   ),
   Fail, //indicates that Term does not return a Value
   Literal(Vec<Literal>),
}

impl Term {
   pub fn equals(tlc: &TLC, lt: TermId, rt: TermId) -> bool {
      match (&tlc.rows[lt.id].term, &tlc.rows[rt.id].term) {
         (Term::Ident(li), Term::Ident(ri)) => { li == ri },
         (Term::Value(lv), Term::Value(rv)) => { lv == rv },
         (Term::Arrow(_ls,lp,lr,lb), Term::Arrow(_rs,rp,rr,rb)) => {
            Term::equals(tlc, *lp, *rp) &&
            lr == rr &&
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
   pub fn scope_of_lhs_impl(tlc: &mut TLC, children: &mut Vec<(String,HashMap<Type,Kind>,Type,Option<TermId>)>, lhs: TermId) {
      match &tlc.rows[lhs.id].term.clone() {
         Term::Ident(n) if n=="_" => {},
         Term::Ident(n) => {
            children.push((n.clone(), HashMap::new(), tlc.rows[lhs.id].typ.clone(), Some(lhs)));
         },
         Term::Ascript(lt,ltt) => {
            tlc.rows[lt.id].typ = ltt.clone();
            tlc.rows[lhs.id].typ = ltt.clone();
            Term::scope_of_lhs_impl(tlc, children, *lt);
         },
         _ => unimplemented!("destructure lhs in Term::scope_of_lhs({})", tlc.print_term(lhs)),
      }
   }
   pub fn scope_of_lhs(tlc: &mut TLC, scope: Option<ScopeId>, lhs: TermId) -> ScopeId {
      let mut children = Vec::new();
      Term::scope_of_lhs_impl(tlc, &mut children, lhs);
      let sid = tlc.push_scope(Scope {
         parent: scope,
         children: children,
      });
      sid
   }
   pub fn compile_lhs(tlc: &TLC, scope: ScopeId, term: TermId) -> Result<LHSPart,Error> {
      let tt = tlc.rows[term.id].typ.clone();
      match &tlc.rows[term.id].term {
         Term::Value(lv) => {
            Ok(LHSPart::literal(lv))
         },
         Term::Ident(ln) => {
            if ln == "_" {
               Ok(LHSPart::any())
            } else {
               let term = Scope::lookup_term(tlc, scope, ln, &tt).expect("Term::compile_lhs identifier not found in scope");
               Ok(LHSPart::variable(term.id))
            }
         },
         Term::Ascript(t,_tt) => {
            Term::compile_lhs(tlc, scope, *t)
         },
         _ => unimplemented!("compile_lhs: {}", tlc.print_term(term))
      }
   }
   pub fn compile_function(tlc: &TLC, scope: &Option<ScopeId>, funcs: &mut Vec<FunctionDefinition<Span>>, term: TermId) -> Result<String,Error> {
      let mangled = if let Term::Let(ref lt) = tlc.rows[term.id].term {
         let mut name = lt.name.clone();
         name += ":";
         for ps in lt.parameters.iter() {
            name += "(";
            for (ai,args) in ps.iter().enumerate() {
            if let Some(at) = args.1.clone() {
               if ai > 0 { name += ","; }
               name += &format!("{:?}", at);
            }}
            name += ")->";
         }
         name += &format!("{:?}", lt.rtype);
         name
      } else { panic!("Term::compile_function must be a Let binding") };
      println!("mangled: {}", mangled);
      for fd in funcs.iter() {
         if fd.name == mangled { return Ok(mangled); }
      }
      let mut l1_args = Vec::new();
      if let Term::Let(ref lt) = tlc.rows[term.id].term {
         if lt.parameters.len()==0 { unimplemented!("Term::compile_function valued let binding") }
         if lt.parameters.len()>1 { unimplemented!("Term::compile_function curried let binding") }
         for args in lt.parameters[0].iter() {
            let name = if let Some(n) = args.0.clone() { n } else { unimplemented!("Term::compile_function parameters must be named") };
            let typ = if let Some(t) = args.1.clone() { t } else { unimplemented!("Term::compile_function parameters must be typed") };
            let dt = typ.datatype();
            let term = Scope::lookup_term(tlc, lt.scope, &name, &typ).expect("Term::compile_function parameter not found in scope");
            l1_args.push(( term.id, ast::Type::nominal(&dt) ));
         }
      }
      funcs.push(FunctionDefinition::define(
         &mangled,
         l1_args,
         vec![],
      ));
      let mut preamble = Vec::new();
      if let Term::Let(ref lt) = tlc.rows[term.id].term {
      if let Some(body) = lt.body {
         let ret = Term::compile_expr(tlc, &Some(lt.scope), funcs, &mut preamble, body)?;
         preamble.push(ret);
      }}
      for ref mut fd in funcs.iter_mut() {
      if fd.name == mangled {
         fd.body = preamble; break;
      }}
      Ok(mangled)
   }
   pub fn compile_expr(tlc: &TLC, scope: &Option<ScopeId>, funcs: &mut Vec<FunctionDefinition<Span>>,
                       preamble: &mut Vec<Expression<Span>>, term: TermId) -> Result<Expression<Span>,Error> {
      let tt = tlc.rows[term.id].typ.clone();
      let span = tlc.rows[term.id].span.clone();
      match &tlc.rows[term.id].term {
         Term::Let(_) => {
            Ok(Expression::unit(span))
         },
         Term::Tuple(ts) if ts.len()==0 => {
            Ok(Expression::unit(span))
         },
         Term::Value(v) => {
            let e = Expression::literal(&v, span).typed(&tt.datatype());
            Ok(e)
         },
         Term::Ident(n) => {
            let tt = tlc.rows[term.id].typ.clone();
            let span = tlc.rows[term.id].span.clone();
            let sc = scope.expect("Term::compile_expr scope was None");
            let term = Scope::lookup_term(tlc, sc, &n, &tt).expect("Term::compile_expr variable not found in scope");
            let e = Expression::variable(term.id, span).typed(&tt.datatype());
            Ok(e)
         },
         Term::Ascript(t,_tt) => {
            //TODO gradual type
            Term::compile_expr(tlc, scope, funcs, preamble, *t)
         },
         Term::Block(sc,es) => {
            if es.len()==0 {
               Ok(Expression::unit(tlc.rows[term.id].span.clone()))
            } else {
               for ei in 0..(es.len()-1) {
                  let pe = Term::compile_expr(tlc, &Some(*sc), funcs, preamble, es[ei])?;
                  preamble.push(pe);
               }
               Term::compile_expr(tlc, &Some(*sc), funcs, preamble, es[es.len()-1])
            }
         },
         Term::Match(dv,lrs) => {
            //These panics are OK, because the type-checker should disprove them
            let pe = Term::compile_expr(tlc, scope, funcs, preamble, *dv)?;
            let mut plrs = Vec::new();
            for (lrc,l,r) in lrs.iter() {
               let lhs = Term::compile_lhs(tlc, *lrc, *l)?;
               let rhs = Term::compile_expr(tlc, &Some(*lrc), funcs, preamble, *r)?;
               plrs.push((lhs,rhs));
            }
            Ok(Expression::pattern(pe, plrs, span).typed(&tt.datatype()))
         },
         Term::App(g,x) => {
            let sc = if let Some(sc) = scope { *sc } else { panic!("Term::reduce, function application has no scope at {:?}", &tlc.rows[term.id].span) };
            match (&tlc.rows[g.id].term,&tlc.rows[x.id].term) {
               (Term::Ident(gv),Term::Tuple(ps)) if gv==".flatmap" && ps.len()==2 => {
                  let iterable = tlc.rows[ps[0].id].term.clone();
                  let Term::Arrow(asc,lhs,att,rhs) = tlc.rows[ps[1].id].term.clone()
                  else { panic!(".flatmap second argument must be an arrow: {}", tlc.print_term(ps[1])) };
                  if let Term::Match(me,mlrs) = &tlc.rows[rhs.id].term {
                     unimplemented!(".flatmap guarded {} {}", tlc.print_term(ps[0]), tlc.print_term(ps[1]));
                  } else {
                     let map_lhs = Term::compile_lhs(tlc, asc.expect("map_lhs expected a scope on left hand side"), lhs)?;
                     unimplemented!(".flatmap unguarded {} {}", tlc.print_term(ps[0]), tlc.print_term(ps[1]));
                     /*
                     Expression::map(
                        LHSPart::variable(10),
                        Expression::apply("range:(U64)->U64[]",vec![
                           Expression::literal("5", ()).typed("U64"),
                        ],()).typed("Value"),
                        TIPart::variable(10)
                    ,()).typed("Value")
                    */
                  }
               },
               (Term::Ident(gv),Term::Tuple(ps)) => {
                  let mut args = Vec::new();
                  for p in ps.iter() {
                     args.push(Term::compile_expr(tlc, scope, funcs, preamble, *p)?);
                  }
                  if let Some(binding) = Scope::lookup_term(tlc, sc, gv, &tlc.rows[g.id].typ) {
                     if let Term::Let(lb) = &tlc.rows[binding.id].term {
                        if lb.parameters.len() > 1 { unimplemented!("Term::reduce, beta-reduce curried functions") }
                        if lb.is_extern {
                           let body = lb.body.expect(&format!("extern function body must be a mangled symbol: {}", gv));
                           if let Term::Ident(mangled) = &tlc.rows[body.id].term {
                              let e = Expression::apply(&mangled, args, span);
                              let e = e.typed(&tt.datatype());
                              Ok(e)
                           } else { unreachable!("extern function body must be a mangled symbol: {}", gv) }
                        } else {
                           let mangled = Term::compile_function(tlc, scope, funcs, binding)?;
                           let e = Expression::apply(&mangled, args, span);
                           let e = e.typed(&tt.datatype());
                           Ok(e)
                        }
                     } else {
                        panic!("Term::reduce, unexpected lambda format in beta-reduction {}", tlc.print_term(binding))
                     }
                  } else { panic!("Term::reduce, failed to lookup function {}: {:?}", gv, &tlc.rows[x.id].typ) }
               },
               _ => unimplemented!("Term::reduce, implement Call-by-Value function call: {}({})", tlc.print_term(*g), tlc.print_term(*x))
            }
         },
         _ => unimplemented!("Term::compile_expr {}", tlc.print_term(term)),
      }
   }
   pub fn reduce(tlc: &TLC, scope: &Option<ScopeId>, term: TermId) -> Result<Constant,Error> {
      let mut preamble = Vec::new();
      let mut funcs = Vec::new();
      let pe = Term::compile_expr(tlc, scope, &mut funcs, &mut preamble, term)?;
      preamble.push(pe);

      let nojit = Program::program(
         funcs,
         preamble,
      );
      println!("debug program");
      let jit = JProgram::compile(&nojit);
      let jval = jit.eval(&[Value::u64(321,"U64")]);

      Ok(Constant::from_value(
         jval
      ))
      /*
         Term::As(t,tt) => {
            let c = Term::reduce(tlc, scope, scope_constants, *t)?;
            Term::check_hard_cast(tlc, &c, tt, term)?;
            Ok(c)
         },
         Term::Constructor(c,cps) if cps.len()==0 => {
            Ok(Constant::parse(tlc, &c).unwrap())
         },
         Term::Tuple(ts) => {
            let mut cs = Vec::new();
            for ct in ts.iter() {
               let cc = Term::reduce(tlc, scope, scope_constants, *ct)?;
               Term::check_hard_cast(tlc, &cc, &tlc.rows[ct.id].typ, *ct)?;
               cs.push(cc);
            }
            Ok(Constant::Tuple(cs))
         },
         Term::Literal(lps) => {
            let mut v = "".to_string();
            for lp in lps.iter() {
            match lp {
               Literal::Char(lc,_) => { v += &lc.to_string(); },
               Literal::String(ls,_) => { v += ls; },
               Literal::Range(_,_) => { panic!("Term::Reduce(Term::Literal) is literal range at {:?}", tlc.rows[term.id].span) }, //not a Literal Value
               Literal::Var(lv) => {
                  if let Some(Constant::Literal(ls)) = scope_constants.get(lv) {
                     v += ls;
                  } else { panic!("Term::reduce free variable in literal {} at {:?}", lv, &tlc.rows[term.id].span) }
               },
            }}
            let cl = Constant::Literal(v);
            Term::check_hard_cast(tlc, &cl, &tlc.rows[term.id].typ, term)?;
            Ok(cl)
         },
         _ => unimplemented!("Term::reduce, implement Call-by-Value term reduction: {}", tlc.print_term(term))
      }
      */
   }
}
