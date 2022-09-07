use std::collections::HashMap;
use crate::term::TermId;
use crate::kind::Kind;
use crate::tlc::TLC;

///Each Term has at least one Type.
///
///Types are composed of Atomic parts like Nameds.
///An Named type has a name and possibly some parameters.
///Atomic parts can be combined to form Compound parts like Arrows.
///Compound parts are formed by some combination of Arrows, Tuples, Products, and Ratios.
///At the Highest level a Compound type can be pluralized with an And to join it to other Compounds.
///
///And types are represented in Conjunctive-Normal-Form which requires the Ands to only occupy the
///highest level of a type. Some basic typing algorithms may not work correctly if a type is not in
///Conjunctive-Normal-Form.
///
///Subtyping is implemented with And types. An implication, A + A => B, may be rewritten as just A + B.
#[derive(Clone,Eq,PartialEq,Ord,PartialOrd,Hash)]
pub enum Type {
   Any,
   Named(String,Vec<Type>),
   And(Vec<Type>), //Bottom is the empty conjunctive
   Arrow(Box<Type>,Box<Type>),
   Tuple(Vec<Type>),   //Tuple is order-sensitive, Nil is the empty tuple
   Product(Vec<Type>), //Product is order-insensitive
   Ratio(Box<Type>,Box<Type>),
   Constant(bool,TermId),
}

impl Type {
   pub fn print(&self, kinds: &HashMap<Type,Kind>) -> String {
      let ts = match self {
         Type::Any => format!("?"),
         Type::Named(t,ts) => {
            if ts.len()==0 { format!("{}", t) }
            else { format!("{}<{}>", t, ts.iter().map(|t|t.print(kinds)).collect::<Vec<String>>().join(",") ) }
         }
         Type::And(ts) => format!("{{{}}}", ts.iter().map(|t|t.print(kinds)).collect::<Vec<String>>().join("+") ),
         Type::Tuple(ts) => format!("({})", ts.iter().map(|t|t.print(kinds)).collect::<Vec<String>>().join(",") ),
         Type::Product(ts) => format!("({})", ts.iter().map(|t|t.print(kinds)).collect::<Vec<String>>().join("*") ),
         Type::Arrow(p,b) => format!("({})=>({})", p.print(kinds), b.print(kinds)),
         Type::Ratio(n,d) => format!("({})/({})", n.print(kinds), d.print(kinds)),
         Type::Constant(v,c) => format!("[{}var#{}]", if *v {"'"} else {""}, c.id),
      };
      if let Some(k) = kinds.get(self) {
         format!("{}::{:?}", ts, k)
      } else { ts }
   }
   pub fn project_ratio(&self) -> (Vec<Type>,Vec<Type>) {
       match self {
         Type::Ratio(p,b) => {
            let (mut pn,mut pd) = p.project_ratio();
            let (mut bn,mut bd) = b.project_ratio();
            pn.append(&mut bd);
            pd.append(&mut bn);
            (pn, pd)
         },
         Type::Product(ts) => {
            let mut tn = Vec::new();
            let mut td = Vec::new();
            for tc in ts.iter() {
               let (mut tcn,mut tcd) = tc.project_ratio();
               tn.append(&mut tcn);
               td.append(&mut tcd);
            }
            (tn, td)
         },
         tt => (vec![tt.clone()], Vec::new())
      }
   }
   pub fn is_constant(&self) -> bool {
      match self {
         Type::Constant(_,_) => true,
         _ => false,
      }
   }
   pub fn term_id(&self) -> TermId {
      match self {
         Type::Constant(_,t) => *t,
         _ => TermId { id: 0 },
      }
   }
   pub fn mask(&self) -> Type {
      match self {
         Type::Any => Type::Any,
         Type::Named(tn,_ts) if tn.chars().all(char::is_uppercase) => Type::Any,
         Type::Named(tn,ts) => Type::Named(tn.clone(),ts.iter().map(|_|Type::Any).collect::<Vec<Type>>()),
         Type::Arrow(p,b) => Type::Arrow(Box::new(p.mask()),Box::new(b.mask())),
         Type::Ratio(p,b) => Type::Ratio(Box::new(p.mask()),Box::new(b.mask())),
         Type::And(ts) => Type::And(ts.iter().map(|ct|ct.mask()).collect::<Vec<Type>>()),
         Type::Tuple(ts) => Type::Tuple(ts.iter().map(|ct|ct.mask()).collect::<Vec<Type>>()),
         Type::Product(ts) => Type::Product(ts.iter().map(|ct|ct.mask()).collect::<Vec<Type>>()),
         Type::Constant(v,c) => Type::Constant(*v,*c)
      }
   }
   pub fn and(&self, other:&Type) -> Type {
      match (self,other) {
         (Type::Any,r) => r.clone(),
         (l,Type::Any) => l.clone(),
         (Type::Named(lv,_lps),rt) if lv.chars().all(char::is_uppercase) => rt.clone(),
         (lt,Type::Named(rv,_rps)) if rv.chars().all(char::is_uppercase) => lt.clone(),
         (Type::And(ls),Type::And(rs)) => {
            let mut ts = ls.clone();
            ts.append(&mut rs.clone());
            ts.sort(); ts.dedup();
            Type::And(ts)
         },
         (Type::And(ls),r) => {
            let mut ts = ls.clone();
            ts.push(r.clone());
            ts.sort(); ts.dedup();
            Type::And(ts)
         }
         (l,Type::And(rs)) => {
            let mut ts = rs.clone();
            ts.push(l.clone());
            ts.sort(); ts.dedup();
            Type::And(ts)
         },
         (l,r) => {
            Type::And(vec![l.clone(),r.clone()])
         }
      }
   }
   pub fn is_var(&self) -> bool {
      match self {
         Type::Named(tn,ts) => ts.len()==0 && tn.chars().all(char::is_uppercase),
         _ => false
      }
   }
   pub fn domain(&self) -> Type {
      match self {
         Type::Arrow(p,_b) => *p.clone(),
         Type::And(ts) => {
            let mut cts = Vec::new();
            for ct in ts.iter() {
               match ct.domain() {
                  Type::And(mut cta) => {
                     cts.append(&mut cta);
                  }, ctr => {
                     cts.push(ctr);
                  }
               }
            }
            if cts.len()==1 { cts[0].clone() }
            else { Type::And(cts) }
         },
         _ => Type::And(Vec::new()), //absurd
      }
   }
   pub fn range(&self) -> Type {
      match self {
         Type::Arrow(_p,b) => *b.clone(),
         Type::And(ts) => {
            let mut cts = Vec::new();
            for ct in ts.iter() {
               match ct.range() {
                  Type::And(mut cta) => {
                     cts.append(&mut cta);
                  }, ctr => {
                     cts.push(ctr);
                  }
               }
            }
            if cts.len()==1 { cts[0].clone() }
            else { Type::And(cts) }
         },
         _ => Type::And(Vec::new()), //absurd
      }
   }
   pub fn vars(&self) -> Vec<String> {
      match self {
         Type::Any => vec![],
         Type::Named(tn,ts) => {
            let mut nv = vec![tn.clone()];
            for tt in ts.iter() {
               nv.append(&mut tt.vars());
            }
            nv
         },
         Type::Arrow(p,b) => { let mut pv=p.vars(); pv.append(&mut b.vars()); pv },
         Type::Ratio(p,b) => { let mut pv=p.vars(); pv.append(&mut b.vars()); pv },
         Type::And(ts) => {
            let mut nv = Vec::new();
            for tt in ts.iter() {
               nv.append(&mut tt.vars());
            }
            nv
         },
         Type::Tuple(ts) => {
            let mut nv = Vec::new();
            for tt in ts.iter() {
               nv.append(&mut tt.vars());
            }
            nv
         },
         Type::Product(ts) => {
            let mut nv = Vec::new();
            for tt in ts.iter() {
               nv.append(&mut tt.vars());
            }
            nv
         },
         Type::Constant(_,_) => vec![]
      }
   }
   pub fn simplify_ratio(&self) -> Type {
      //assume Typee has already been normalized
      let (mut num, den) = self.project_ratio();
      let mut rden = Vec::new();
      for d in den.into_iter() {
         if let Some(ni) = num.iter().position(|n|n==&d) {
            num.remove(ni);
         } else {
            rden.push(d);
         }
      }
      let n = if num.len()==0 {
         Type::Tuple(Vec::new())
      } else if num.len()==1 {
         num[0].clone()
      } else {
         Type::Product(num)
      };
      if rden.len()==0 {
         n
      } else if rden.len()==1 {
         Type::Ratio(Box::new(n),Box::new(rden[0].clone()))
      } else {
         let d = Type::Product(rden);
         Type::Ratio(Box::new(n),Box::new(d))
      }
   }
   pub fn normalize(&self) -> Type {
      match self {
         Type::And(ts) => {
            let mut cnf = Vec::new();
            for ct in ts.iter() {
               let ct = ct.normalize();
               match ct {
                  Type::And(mut cts) => { cnf.append(&mut cts); },
                  _ => { cnf.push(ct); }
               }
            }
            cnf.sort(); cnf.dedup();
            if cnf.len()==1 {
               cnf[0].clone()
            } else {
               Type::And(cnf)
            }
         },
         Type::Product(ts) => {
            let mut ts = ts.iter().map(|tt|tt.normalize()).collect::<Vec<Type>>();
            ts.sort();
            Type::Product(ts).simplify_ratio()
         },
         Type::Tuple(ts) => {
            let ts = ts.iter().map(|tt|tt.normalize()).collect::<Vec<Type>>();
            Type::Tuple(ts)
         },
         Type::Named(tn,ts) => {
            let ts = ts.iter().map(|tt|tt.normalize()).collect::<Vec<Type>>();
            Type::Named(tn.clone(),ts)
         },
         Type::Arrow(p,b) => {
            Type::Arrow(Box::new(p.normalize()), Box::new(b.normalize()))
         },
         Type::Ratio(_p,_b) => self.simplify_ratio(),
         tt => tt.clone(),
      }
   }
   pub fn remove(&self, x:&Type) -> Type {
      if self == x { return Type::And(Vec::new()); }
      match self {
         Type::Any => Type::Any,
         Type::Arrow(p,b) => Type::Arrow(Box::new(p.remove(x)),Box::new(b.remove(x))),
         Type::Ratio(p,b) => Type::Ratio(Box::new(p.remove(x)),Box::new(b.remove(x))),
         Type::Named(tn,ts) => Type::Named(tn.clone(),ts.iter().map(|t| t.remove(x)).collect::<Vec<Type>>()),
         Type::And(ts) => Type::And(ts.iter().map(|t| t.remove(x)).collect::<Vec<Type>>()),
         Type::Tuple(ts) => Type::Tuple(ts.iter().map(|t| t.remove(x)).collect::<Vec<Type>>()),
         Type::Product(ts) => Type::Product(ts.iter().map(|t| t.remove(x)).collect::<Vec<Type>>()),
         Type::Constant(v,c) => Type::Constant(*v,*c)
      }.normalize()
   }
   pub fn substitute(&self, subs:&HashMap<Type,Type>) -> Type {
      if let Some(st) = subs.get(self) {
         return st.clone();
      }
      match self {
         Type::Any => Type::Any,
         Type::Arrow(p,b) => Type::Arrow(Box::new(p.substitute(subs)),Box::new(b.substitute(subs))),
         Type::Ratio(p,b) => Type::Ratio(Box::new(p.substitute(subs)),Box::new(b.substitute(subs))),
         Type::Named(tn,ts) => Type::Named(tn.clone(),ts.iter().map(|t| t.substitute(subs)).collect::<Vec<Type>>()),
         Type::And(ts) => Type::And(ts.iter().map(|t| t.substitute(subs)).collect::<Vec<Type>>()),
         Type::Tuple(ts) => Type::Tuple(ts.iter().map(|t| t.substitute(subs)).collect::<Vec<Type>>()),
         Type::Product(ts) => Type::Product(ts.iter().map(|t| t.substitute(subs)).collect::<Vec<Type>>()),
         Type::Constant(v,c) => Type::Constant(*v,*c)
      }
   }
   pub fn is_concrete(&self) -> bool {
      match self {
         Type::Any => false,
         Type::Arrow(p,b) => p.is_concrete() && b.is_concrete(),
         Type::Ratio(p,b) => p.is_concrete() && b.is_concrete(),
         Type::Named(_tn,ts) => ts.iter().all(|tc| tc.is_concrete()),
         Type::And(ts) => ts.iter().all(|tc| tc.is_concrete()), //bottom Typee is also concrete
         Type::Tuple(ts) => ts.iter().all(|tc| tc.is_concrete()),
         Type::Product(ts) => ts.iter().all(|tc| tc.is_concrete()),
         Type::Constant(_,_) => true,
      }
   }
   pub fn kind(&self, kinds: &HashMap<Type,Kind>) -> Kind {
      if let Some(k) = kinds.get(&self) {
         return k.clone();
      }
      match self {
         Type::Constant(_,_) => Kind::Named("Constant".to_string(),Vec::new()),
         Type::And(ats) => {
            let mut aks = Vec::new();
            for at in ats.iter() {
              aks.push(at.kind(kinds));
            }
            Kind::and(aks)
         },
         _ => Kind::Nil,
      }
   }
   pub fn narrow(&self, kinds: &HashMap<Type,Kind>, k: &Kind) -> Type {
      if !self.kind(kinds).has(k) { return Type::And(Vec::new()); } //nothing here to take
      let tt = match self {
         Type::And(ts) => {
            let mut tcs = Vec::new();
            for tc in ts.iter() {
               match tc.narrow(kinds,k) {
                  Type::And(acs) => {
                     tcs.append(&mut acs.clone());
                  }, ac => {
                     tcs.push(ac.clone());
                  }
               }
            }
            if tcs.len()==1 { tcs[0].clone() }
            else { Type::And(tcs) }
         }
         tt => tt.clone(),
      };
      tt
   }
   pub fn is_bottom(&self) -> bool {
      match self {
         Type::And(ts) if ts.len()==0 => { true },
         _ => false
      }
   }
   pub fn implies(tlc: &mut TLC, kinds: &HashMap<Type,Kind>, lt: &Type, rt: &Type) -> Type {
      let mut lt = tlc.extend_implied(lt);
      let mut rt = tlc.extend_implied(rt);
      lt.implication_unifier(&rt)
      /*
      //lt => rt
      let mut subs = HashMap::new();
      self.reduce_type(&subs, &mut lt, span); //reduce constant expressions in dependent types
      lt = lt.normalize();
      let mut rt = rt.clone();
      self.reduce_type(&subs, &mut rt, span);
      rt = rt.normalize();
      if let Ok(ref mut tt) = lt.unify_impl_par(kinds, &mut subs, &rt, par) {
         self.reduce_type(&subs, tt, span);
         let tt = tt.normalize();
         Ok(tt)
      } else { return Err(Error {
         kind: "Type Error".to_string(),
         rule: format!("failed unification {} (x) {}", self.print_type(kinds,&lt), self.print_type(kinds,&rt)),
         span: span.clone(),
      }) }
      */
   }
   pub fn implication_unifier(&self, other: &Type) -> Type {
      let mut subs = Vec::new();
      let nt = self._implication_unifier(other, &mut subs);
      let mut msubs: HashMap<Type,Type> = HashMap::new();
      for (lt,mut rt) in subs.clone().into_iter() {
         if let Some(vt) = msubs.get(&lt) {
            rt = vt.most_general_unifier(&rt);
            if rt.is_bottom() { return rt.clone(); }
         }
         msubs.insert(lt, rt);
      }
      nt.substitute(&msubs)
   }
   fn _implication_unifier(&self, other: &Type, subs: &mut Vec<(Type,Type)>) -> Type {
      //if the two types don't unify
      //then the mgu will be the bottom type
      match (self,other) {
         //wildcard failure
         (Type::And(lts),_) if lts.len()==0 => { Type::And(vec![]) },
         (_,Type::And(rts)) if rts.len()==0 => { Type::And(vec![]) },

         //wildcard match
         (lt,Type::Any) => { lt.clone() },
         (Type::Named(lv,_lps),rt) if lv.chars().all(char::is_uppercase) => {
            subs.push((self.clone(), rt.clone()));
            self.clone()
         },
         (lt,Type::Named(rv,_rps)) if rv.chars().all(char::is_uppercase) => {
            subs.push((other.clone(), lt.clone()));
            other.clone()
         },

         //conjunctive normal form takes precedence
         (Type::And(_lts),Type::And(rts)) => {
            let mut mts = Vec::new();
            for rt in rts.iter() {
               match self._implication_unifier(rt,subs) {
                  Type::And(tts) if tts.len()==0 => { return Type::And(vec![]); },
                  Type::And(mut tts) => { mts.append(&mut tts); },
                  tt => { mts.push(tt); },
               }
            }
            mts.sort(); mts.dedup();
            if mts.len()==1 { mts[0].clone() }
            else { Type::And(mts) }
         },
         (Type::And(lts),rt) => {
            let mut mts = Vec::new();
            for ltt in lts.iter() {
               match ltt._implication_unifier(rt,subs) {
                  Type::And(mut tts) => { mts.append(&mut tts); },
                  tt => { mts.push(tt); },
               }
            }
            mts.sort(); mts.dedup();
            if mts.len()==1 { mts[0].clone() }
            else { Type::And(mts) }
         },
         (lt,Type::And(rts)) => {
            let mut mts = Vec::new();
            for rt in rts.iter() {
               match lt._implication_unifier(rt,subs) {
                  Type::And(tts) if tts.len()==0 => { return Type::And(vec![]); },
                  Type::And(mut tts) => { mts.append(&mut tts); },
                  tt => { mts.push(tt); },
               }
            }
            mts.sort(); mts.dedup();
            if mts.len()==1 { mts[0].clone() }
            else { Type::And(mts) }
         }

         //ratio Typees have next precedence
         (Type::Ratio(pl,bl),Type::Ratio(pr,br)) => {
            let pt = pl._implication_unifier(pr,subs);
            if pt.is_bottom() { return pt.clone(); }
            let bt = bl._implication_unifier(br,subs);
            if bt.is_bottom() { return bt.clone(); }
            Type::Ratio(Box::new(pt),Box::new(bt))
         },
         (lt,Type::Ratio(pr,br)) => {
            //assert Nil divisor on rhs
            match **br {
               Type::Tuple(ref bs) if bs.len()==0 => {
                  lt._implication_unifier(pr,subs)
               }, _ => { Type::And(vec![]) }
            }
         },
         (Type::Ratio(pl,bl),rt) => {
            //assert Nil divisor on rhs
            match **bl {
               Type::Tuple(ref bs) if bs.len()==0 => {
                  pl._implication_unifier(rt,subs)
               }, _ => { Type::And(vec![]) }
            }
         },

         //everything else is a mixed bag
         (Type::Named(lv,lps),Type::Named(rv,rps))
         if lv==rv && lps.len()==rps.len() => {
            let mut tps = Vec::new();
            for (lp,rp) in std::iter::zip(lps,rps) {
               let nt = lp._implication_unifier(rp,subs);
               if nt.is_bottom() { return nt.clone(); }
               tps.push(lp._implication_unifier(rp,subs));
            }
            Type::Named(lv.clone(),tps)
         }
         (Type::Arrow(pl,bl),Type::Arrow(pr,br)) => {
            let pt = pr._implication_unifier(pl,subs); //contravariant
            if pt.is_bottom() { return pt.clone(); }
            let bt = bl._implication_unifier(br,subs);
            if bt.is_bottom() { return bt.clone(); }
            Type::Arrow(Box::new(pt),Box::new(bt))
         },
         (Type::Product(la),Type::Product(ra)) if la.len()==ra.len() => {
            let mut ts = Vec::new();
            for (lt,rt) in std::iter::zip(la,ra) {
               let nt = lt._implication_unifier(rt,subs);
               if nt.is_bottom() { return nt.clone(); }
               ts.push(nt.clone());
            }
            Type::Product(ts)
         },
         (Type::Tuple(la),Type::Tuple(ra)) if la.len()==ra.len() => {
            let mut ts = Vec::new();
            for (lt,rt) in std::iter::zip(la,ra) {
               let nt = lt._implication_unifier(rt,subs);
               if nt.is_bottom() { return nt.clone(); }
               ts.push(nt.clone());
            }
            Type::Tuple(ts)
         },

         (Type::Constant(lv,lc),Type::Constant(rv,rc)) => {
            if lc.id == rc.id {
               Type::Constant(*lv || *rv, *lc)
            } else {
               Type::And(vec![])
            }
         },
         _ => Type::And(vec![]),
      }
   }
   pub fn most_general_unifier(&self, other: &Type) -> Type {
      //if the two types don't unify
      //then the mgu will be the bottom type
      match (self,other) {
         //wildcard failure
         (Type::And(lts),_) if lts.len()==0 => { Type::And(vec![]) },
         (_,Type::And(rts)) if rts.len()==0 => { Type::And(vec![]) },

         //wildcard match
         (Type::Any,Type::Any) => { self.clone() },
         (Type::Named(lv,_lps),Type::Named(rv,_rps)) if lv.chars().all(char::is_uppercase) && lv==rv => {
            self.clone()
         },

         //conjunctive normal form takes precedence
         (Type::And(_lts),Type::And(rts)) => {
            let mut mts = Vec::new();
            for rt in rts.iter() {
               match self.most_general_unifier(rt) {
                  Type::And(mut tts) => { mts.append(&mut tts); },
                  tt => { mts.push(tt); },
               }
            }
            mts.sort(); mts.dedup();
            if mts.len()==1 { mts[0].clone() }
            else { Type::And(mts) }
         },
         (Type::And(lts),rt) => {
            let mut mts = Vec::new();
            for ltt in lts.iter() {
               match ltt.most_general_unifier(rt) {
                  Type::And(mut tts) => { mts.append(&mut tts); },
                  tt => { mts.push(tt); },
               }
            }
            mts.sort(); mts.dedup();
            if mts.len()==1 { mts[0].clone() }
            else { Type::And(mts) }
         },
         (lt,Type::And(rts)) => {
            let mut mts = Vec::new();
            for rt in rts.iter() {
               match lt.most_general_unifier(rt) {
                  Type::And(mut tts) => { mts.append(&mut tts); },
                  tt => { mts.push(tt); },
               }
            }
            mts.sort(); mts.dedup();
            if mts.len()==1 { mts[0].clone() }
            else { Type::And(mts) }
         }

         //ratio Typees have next precedence
         (Type::Ratio(pl,bl),Type::Ratio(pr,br)) => {
            let pt = pl.most_general_unifier(pr);
            if pt.is_bottom() { return pt.clone(); }
            let bt = bl.most_general_unifier(br);
            if bt.is_bottom() { return bt.clone(); }
            Type::Ratio(Box::new(pt),Box::new(bt))
         },
         (lt,Type::Ratio(pr,br)) => {
            //assert Nil divisor on rhs
            match **br {
               Type::Tuple(ref bs) if bs.len()==0 => {
                  lt.most_general_unifier(pr)
               }, _ => { Type::And(vec![]) }
            }
         },
         (Type::Ratio(pl,bl),rt) => {
            //assert Nil divisor on rhs
            match **bl {
               Type::Tuple(ref bs) if bs.len()==0 => {
                  pl.most_general_unifier(rt)
               }, _ => { Type::And(vec![]) }
            }
         },

         //everything else is a mixed bag
         (Type::Named(lv,lps),Type::Named(rv,rps))
         if lv==rv && lps.len()==rps.len() => {
            let mut tps = Vec::new();
            for (lp,rp) in std::iter::zip(lps,rps) {
               let nt = lp.most_general_unifier(rp);
               if nt.is_bottom() { return nt.clone(); }
               tps.push(nt);
            }
            Type::Named(lv.clone(),tps)
         }
         (Type::Arrow(pl,bl),Type::Arrow(pr,br)) => {
            let pt = if pl == pr { (**pl).clone() } else { Type::And(vec![]) };
            if pt.is_bottom() { return pt.clone(); }
            let bt = bl.most_general_unifier(br);
            if bt.is_bottom() { return bt.clone(); }
            Type::Arrow(Box::new(pt),Box::new(bt))
         },
         (Type::Product(la),Type::Product(ra)) if la.len()==ra.len() => {
            let mut ts = Vec::new();
            for (lt,rt) in std::iter::zip(la,ra) {
               let nt = lt.most_general_unifier(rt);
               if nt.is_bottom() { return nt.clone(); }
               ts.push(nt.clone());
            }
            Type::Product(ts)
         },
         (Type::Tuple(la),Type::Tuple(ra)) if la.len()==ra.len() => {
            let mut ts = Vec::new();
            for (lt,rt) in std::iter::zip(la,ra) {
               let nt = lt.most_general_unifier(rt);
               if nt.is_bottom() { return nt.clone(); }
               ts.push(nt.clone());
            }
            Type::Tuple(ts)
         },

         (Type::Constant(lv,lc),Type::Constant(rv,rc)) => {
            if lc.id == rc.id {
               Type::Constant(*lv || *rv, *lc)
            } else {
               Type::And(vec![])
            }
         },
         _ => Type::And(vec![]),
      }
   }
}

impl std::fmt::Debug for Type {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
           Type::Any => write!(f, "?"),
           Type::Named(t,ts) => {
              if ts.len()==0 { write!(f, "{}", t) }
              else { write!(f, "{}<{}>", t, ts.iter().map(|t|format!("{:?}",t)).collect::<Vec<String>>().join(",") ) }
           }
           Type::And(ts) => write!(f, "{{{}}}", ts.iter().map(|t|format!("{:?}",t)).collect::<Vec<String>>().join("+") ),
           Type::Tuple(ts) => write!(f, "({})", ts.iter().map(|t|format!("{:?}",t)).collect::<Vec<String>>().join(",") ),
           Type::Product(ts) => write!(f, "({})", ts.iter().map(|t|format!("{:?}",t)).collect::<Vec<String>>().join("*") ),
           Type::Arrow(p,b) => write!(f, "({:?})=>({:?})", p, b),
           Type::Ratio(n,d) => write!(f, "({:?})/({:?})", n, d),
           Type::Constant(v,c) => write!(f, "[{}term#{}]", if *v {"'"} else {""}, c.id),
        }
    }
}

