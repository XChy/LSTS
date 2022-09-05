use lsts::typ::*;
use lsts::term::TermId;

#[test]
fn check_structural_equality() {
   let tany = Type::Any;
   let tn1  = Type::Named("Aa".to_string(),vec![]);
   let tn2  = Type::Named("Bb".to_string(),vec![]);
   let tn3  = Type::Named("Cc".to_string(),vec![tn1.clone(),tn2.clone()]);
   let td1  = Type::And(vec![]);
   let td2  = Type::And(vec![tn1.clone()]);
   let td3  = Type::And(vec![tn1.clone(),tn2.clone(),tn3.clone()]);
   let ta1  = Type::Arrow(Box::new(tn1.clone()), Box::new(tn2.clone()));
   let tt1  = Type::Tuple(vec![tn1.clone(),ta1.clone()]);
   let tp1  = Type::Product(vec![tn1.clone(),ta1.clone()]);
   let tr1  = Type::Ratio(Box::new(tt1.clone()),Box::new(tp1.clone()));
   let tc1  = Type::Constant(false,TermId{id:1});
   let tc2  = Type::Constant(false,TermId{id:2});
   assert_eq!(tany, tany);
   assert_eq!(tn1, tn1);
   assert_eq!(tn2, tn2);
   assert_eq!(tn3, tn3);
   assert_eq!(td1, td1);
   assert_eq!(td2, td2);
   assert_eq!(td3, td3);
   assert_eq!(ta1, ta1);
   assert_eq!(tt1, tt1);
   assert_eq!(tp1, tp1);
   assert_eq!(tr1, tr1);
   assert_eq!(tc1, tc1);
   assert_eq!(tc2, tc2);

   assert_ne!(tany, tn1);
   assert_ne!(tn1, tn2);
   assert_ne!(tn2, tn3);
   assert_ne!(tn3, td1);
   assert_ne!(td1, td2);
   assert_ne!(td2, td3);
   assert_ne!(td3, ta1);
   assert_ne!(ta1, tt1);
   assert_ne!(tt1, tp1);
   assert_ne!(tp1, tr1);
   assert_ne!(tr1, tc1);
   assert_ne!(tc1, tc2);
   assert_ne!(tc2, tany);
}

#[test]
fn check_self_unifies() {
   let tany = Type::Any;
   let tn1  = Type::Named("Aa".to_string(),vec![]);
   let tn2  = Type::Named("Bb".to_string(),vec![]);
   let tn3  = Type::Named("Cc".to_string(),vec![tn1.clone(),tn2.clone()]);
   let td1  = Type::And(vec![]);
   let td2  = Type::And(vec![tn1.clone()]);
   let td3  = Type::And(vec![tn1.clone(),tn2.clone(),tn3.clone()]);
   let ta1  = Type::Arrow(Box::new(tn1.clone()), Box::new(tn2.clone()));
   let tt1  = Type::Tuple(vec![tn1.clone(),ta1.clone()]);
   let tp1  = Type::Product(vec![tn1.clone(),ta1.clone()]);
   let tr1  = Type::Ratio(Box::new(tt1.clone()),Box::new(tp1.clone()));
   let tc1  = Type::Constant(false,TermId{id:1});
   let tc2  = Type::Constant(false,TermId{id:2});
   assert_eq!(tany, tany.most_general_unifier(&tany));
   assert_eq!(tn1, tn1.most_general_unifier(&tn1));
   assert_eq!(tn2, tn2.most_general_unifier(&tn2));
   assert_eq!(tn3, tn3.most_general_unifier(&tn3));
   assert_eq!(td1, td1.most_general_unifier(&td1));
   assert_eq!(tn1, td2.most_general_unifier(&td2));
   assert_eq!(td3, td3.most_general_unifier(&td3));
   assert_eq!(ta1, ta1.most_general_unifier(&ta1));
   assert_eq!(tt1, tt1.most_general_unifier(&tt1));
   assert_eq!(tp1, tp1.most_general_unifier(&tp1));
   assert_eq!(tr1, tr1.most_general_unifier(&tr1));
   assert_eq!(tc1, tc1.most_general_unifier(&tc1));
   assert_eq!(tc2, tc2.most_general_unifier(&tc2));
}

#[test]
fn check_plural_mgu() {
   let tany = Type::Any;
   let tn1  = Type::Named("Aa".to_string(),vec![]);
   let tn2  = Type::Named("Bb".to_string(),vec![]);
   let tn3  = Type::Named("Cc".to_string(),vec![]);
   let ta1  = Type::Arrow(Box::new(tn1.clone()), Box::new(tn2.clone()));
   let tt1  = Type::Tuple(vec![tn1.clone(), tn2.clone()]);
   let tp1  = Type::Product(vec![tn1.clone(), tn2.clone()]);
   let tr1  = Type::Ratio(Box::new(tn1.clone()), Box::new(tn2.clone()));
   let tc1  = Type::Constant(false, TermId{id:1});
   let tc2  = Type::Constant(false, TermId{id:2});
   assert_eq!(
      Type::And(vec![tany.clone(), tn1.clone()]).most_general_unifier(&tany), 
      tany.clone()
   );
   assert_eq!(
      Type::And(vec![tn1.clone(), tn2.clone()]).most_general_unifier(&tn1), 
      tn1.clone()
   );
   assert_eq!(
      Type::And(vec![ta1.clone(), tn3.clone()]).most_general_unifier(&ta1), 
      ta1.clone()
   );
   assert_eq!(
      Type::And(vec![tt1.clone(), tn3.clone()]).most_general_unifier(&tt1), 
      tt1.clone()
   );
   assert_eq!(
      Type::And(vec![tp1.clone(), tn3.clone()]).most_general_unifier(&tp1), 
      tp1.clone()
   );
   assert_eq!(
      Type::And(vec![tr1.clone(), tn3.clone()]).most_general_unifier(&tr1), 
      tr1.clone()
   );
   assert_eq!(
      Type::And(vec![tc1.clone(), tc2.clone()]).most_general_unifier(&tc1), 
      tc1.clone()
   );
}

#[test]
fn check_special_cases_mgu() {
   let tn1 = Type::Named("Aa".to_string(),vec![]);
   let tt1 = Type::Tuple(vec![]);
   let tr1 = Type::Ratio(Box::new(tn1.clone()), Box::new(tt1.clone()));
   assert_eq!(
      tr1.most_general_unifier(&tn1), 
      tn1.clone()
   );
}
