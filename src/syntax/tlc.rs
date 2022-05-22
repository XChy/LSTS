use pest::Parser;
use pest::iterators::{Pair,Pairs};
use pest::error::{ErrorVariant,InputLocation,LineColLocation};

#[derive(Parser)]
#[grammar = "tlc.pest"]
struct TlcParser;

pub struct TLC;

pub struct TlcError {
   error_type: String,
   rule: String,
   filename: String,
   start: (usize,usize),
   end: (usize,usize),
   snippet: String
}
impl std::fmt::Debug for TlcError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}, expected {}, in {} --> {},{}{}", self.error_type, self.rule, self.filename,
               self.start.0, self.start.1, self.snippet)
    }
}

#[derive(Clone)]
pub enum TlcExpr {
   Nil,
   Ident(String),
   App(Box<TlcExpr>,Box<TlcExpr>),
   Let(Box<TlcExpr>,Box<TlcExpr>,Box<TlcExpr>),
   Tuple(Vec<TlcExpr>),
   Block(Vec<TlcExpr>),
   Ascript(Box<TlcExpr>,Box<TlcExpr>),
   TypNil,
   TypAny,
   TypIdent(String),
   TypOr(Vec<TlcExpr>),
   TypAnd(Vec<TlcExpr>),
   TypArrow(Box<TlcExpr>,Box<TlcExpr>),
   TypCompound(Box<TlcExpr>,Vec<TlcExpr>),
   TypTuple(Vec<TlcExpr>),
   TypAngle(Vec<TlcExpr>),
   TypBrack(Vec<TlcExpr>),
}

impl TLC {
   pub fn normalize_file(ps: Pairs<crate::syntax::tlc::Rule>) -> Result<TlcExpr,TlcError> {
      TLC::normalize_ast(ps.peek().unwrap())
   }
   pub fn normalize_ast(p: Pair<crate::syntax::tlc::Rule>) -> Result<TlcExpr,TlcError> {
      match p.as_rule() {
         //entry point rule
         Rule::file => {
            let mut es = Vec::new();
            for e in p.into_inner() { match e.as_rule() {
               Rule::EOI => (),
               _ => es.push(TLC::normalize_ast(e).expect("TLC Grammar Error in rule [file]"))
            }}
            if es.len()==1 {
               Ok(es[0].clone())
            } else {
               Ok(TlcExpr::Block(es))
            }
         },

         //passthrough rules
         Rule::stmt => TLC::normalize_ast(p.into_inner().next().expect("TLC Grammar Error in rule [stmt]")),
         Rule::term => TLC::normalize_ast(p.into_inner().next().expect("TLC Grammar Error in rule [term]")),
         Rule::ident_term => TLC::normalize_ast(p.into_inner().next().expect("TLC Grammar Error in rule [ident_term]")),
         Rule::tuple_term => TLC::normalize_ast(p.into_inner().next().expect("TLC Grammar Error in rule [tuple_term]")),
         Rule::ident_typ => TLC::normalize_ast(p.into_inner().next().expect("TLC Grammar Error in rule [ident_typ]")),
         Rule::atom_typ => TLC::normalize_ast(p.into_inner().next().expect("TLC Grammar Error in rule [atom_typ]")),
         Rule::typ => TLC::normalize_ast(p.into_inner().next().expect("TLC Grammar Error in rule [typ]")),

         //literal value rules
         Rule::ident => Ok(TlcExpr::Ident(p.into_inner().concat())),
         Rule::ident_typ => Ok(TlcExpr::TypIdent(p.into_inner().concat())),
         Rule::any_typ => Ok(TlcExpr::TypAny),

         //complex rules
         Rule::let_stmt => {
            let mut es = p.into_inner();
            Ok(TlcExpr::Let(
               Box::new(TLC::normalize_ast(es.next().expect("TLC Grammar Error in rule [let_stmt]"))?),
               Box::new(TLC::normalize_ast(es.next().expect("TLC Grammar Error in rule [let_stmt]"))?),
               Box::new(TLC::normalize_ast(es.next().expect("TLC Grammar Error in rule [let_stmt]"))?),
            ))
         },
         Rule::let_stmt_val => {
            match p.into_inner().next() {
               None => Ok(TlcExpr::Nil),
               Some(e) => TLC::normalize_ast(e)
            }
         },
         Rule::let_stmt_typ => {
            match p.into_inner().next() {
               None => Ok(TlcExpr::Nil),
               Some(e) => TLC::normalize_ast(e)
            }
         },
         Rule::ascript_term => {
            let mut es = p.into_inner();
            let e = es.next().expect("TLC Grammar Error in rule [ascript_term]");
            match es.next() {
               None => TLC::normalize_ast(e),
               Some(tt) => Ok(TlcExpr::Ascript(
                  Box::new(TLC::normalize_ast(e)?), //term
                  Box::new(TLC::normalize_ast(tt)?) //type
               )),
            }
         },
         Rule::term_atom => {
            let mut es = p.into_inner();
            let mut e = TLC::normalize_ast(es.next().expect("TLC Grammar Error in rule [term_atom]"))?;
            for args in es {
               e = TlcExpr::App(
                  Box::new(e),
                  Box::new(TLC::normalize_ast(args)?)
               );
            }
            Ok(e)
         },
         Rule::paren_atom => {
            let es = p.into_inner().map(|e|TLC::normalize_ast(e).expect("TLC Grammar Error in rule [paren_atom]"))
                      .collect::<Vec<TlcExpr>>();
            if es.len()==0 {
               Ok(TlcExpr::Nil)
            } else if es.len()==1 {
               Ok(es[0].clone())
            } else {
               Ok(TlcExpr::Tuple(es))
            }
         },
         Rule::paren_typ => {
            let ts = p.into_inner().map(|e|TLC::normalize_ast(e).expect("TLC Grammar Error in rule [paren_typ]"))
                      .collect::<Vec<TlcExpr>>();
            if ts.len()==0 {
               Ok(TlcExpr::Nil)
            } else if ts.len()==1 {
               Ok(ts[0].clone())
            } else {
               Ok(TlcExpr::TypTuple(ts))
            }
         },
         Rule::arrow_typ => {
            let mut ts = p.into_inner();
            let mut t = TLC::normalize_ast(ts.next().expect("TLC Grammar Error in rule [arrow_typ]"))?;
            for tr in ts {
               t = TlcExpr::TypArrow(
                  Box::new(t),
                  Box::new(TLC::normalize_ast(tr)?)
               );
            }
            Ok(t)
         },
         Rule::suffix_typ => {
            let mut ts = p.into_inner();
            let mut t = TLC::normalize_ast(ts.next().expect("TLC Grammar Error in rule [suffix_typ]"))?;
            let ts = ts.map(|e|TLC::normalize_ast(e).expect("TLC Grammar Error in rule [suffix_typ]"))
                       .collect::<Vec<TlcExpr>>();
            if ts.len()==0 {
               Ok(t)
            } else {
               Ok(TlcExpr::TypCompound(
                  Box::new(t),
                  ts
               ))
            }
         },
         Rule::or_typ => {
            let ts = p.into_inner().map(|e|TLC::normalize_ast(e).expect("TLC Grammar Error in rule [or_typ]"))
                      .collect::<Vec<TlcExpr>>();
            if ts.len()==1 {
               Ok(ts[0].clone())
            } else {
               Ok(TlcExpr::TypOr(ts))
            }
         },
         Rule::and_typ => {
            let ts = p.into_inner().map(|e|TLC::normalize_ast(e).expect("TLC Grammar Error in rule [and_typ]"))
                      .collect::<Vec<TlcExpr>>();
            if ts.len()==1 {
               Ok(ts[0].clone())
            } else {
               Ok(TlcExpr::TypAnd(ts))
            }
         },
         Rule::angle_typ => {
            let ts = p.into_inner().map(|e|TLC::normalize_ast(e).expect("TLC Grammar Error in rule [angle_typ]"))
                      .collect::<Vec<TlcExpr>>();
            Ok(TlcExpr::TypAngle(ts))
         },
         Rule::brack_typ => {
            let ts = p.into_inner().map(|e|TLC::normalize_ast(e).expect("TLC Grammar Error in rule [brack_typ]"))
                      .collect::<Vec<TlcExpr>>();
            Ok(TlcExpr::TypBrack(ts))
         },

         rule => panic!("unexpected rule: {:?}", rule)
      }
   }
   pub fn typecheck(e: TlcExpr) -> Result<(),TlcError> {
      Ok(())
   }
   pub fn check(src:&str) -> Result<(),TlcError> {
      let ast = TLC::parse(src)?;
      TLC::typecheck(ast)
   }
   pub fn parse(src:&str) -> Result<TlcExpr,TlcError> {
      let parse_result = TlcParser::parse(Rule::file, src);
      match parse_result {
        Ok(parse_ast) => TLC::normalize_file(parse_ast),
        Err(pe) => {
          let (start,end) = match pe.line_col {
             LineColLocation::Pos(s) => (s,s),
             LineColLocation::Span(s,e) => (s,e),
          };
          let (istart,iend) = match pe.location {
             InputLocation::Pos(s) => (s,s),
             InputLocation::Span((s,e)) => (s,e),
          };
          let rule = match pe.variant {
             ErrorVariant::ParsingError {
                positives:p,
                negatives:n
             } => {
                p.iter().map(|r|{format!("{:?}",r)}).collect::<Vec<String>>().join(" or ")
             }, _ => {format!("")}
          };
          Err(TlcError { 
             error_type: "Parse Error".to_string(),
             rule: rule,
             filename:"[string]".to_string(),
             start:start, end:end,
             snippet: if iend>istart { format!("\n{}", &src[istart..iend]) }
                      else { format!(" {:?}", &src[istart..std::cmp::min(src.len(),istart+1)])}
          })
        }
      } 
   }
}
