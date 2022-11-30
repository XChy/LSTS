use crate::tlc::TLC;

#[derive(Clone,Eq,PartialEq,Ord,PartialOrd,Hash)]
pub enum Constant {
   NaN,
   Boolean(bool),
   Integer(i64),
   Op(String),
   Tuple(Vec<Constant>),
}

impl std::fmt::Debug for Constant {
   fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      match self {
        Constant::NaN => write!(f, "NaN"),
        Constant::Boolean(c) => write!(f, "{}", if *c {"True"} else {"False"}),
        Constant::Integer(i) => write!(f, "{}", i),
        Constant::Op(op) => write!(f, "{}", op),
        Constant::Tuple(ts) => write!(f, "({})", ts.iter()
           .map(|t|format!("{:?}",t)).collect::<Vec<String>>()
           .join(",") ),
      }
   }
}

impl Constant {
   pub fn parse(tlc: &TLC, v: &str) -> Option<Constant> {
      if      v=="NaN" { Some(Constant::NaN) }
      else if v=="True" { Some(Constant::Boolean(true)) }
      else if v=="False" { Some(Constant::Boolean(false)) }
      else if let Ok(vi) = v.parse::<i64>() { Some(Constant::Integer(vi)) }
      else { Some(Constant::Op(v.to_string())) }
   }
}
