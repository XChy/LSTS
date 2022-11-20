use lsts::tlc::TLC;

#[test]
fn check_alpha_conversion() {
   let mut tlc = TLC::new();
   tlc.strict();
   let alg = tlc.import_file(None, "preludes/algebra.tlc").unwrap();

   tlc.check(Some(alg), "1 + 2 * 3 == 1 + 2 * 3 @reflexive : [True]").unwrap();
}
