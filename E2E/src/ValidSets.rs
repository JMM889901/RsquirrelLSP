use super::*;

#[cfg(test)]
mod tests{
    use std::path;

    use super::*;
    #[test]
    fn test_valid_context_8() {
        let path = "../TestSets/ValidStructure/8Global";
        let result = run_paths(&path.to_string(), None);
        //let mut variants = vec![];
        //for run in &result{
        //    let variants_run = run.file.variants.get_direct().iter().map(|x| x.globalinfo.primitive.context.clone()).collect::<Vec<_>>();
        //    variants.extend(variants_run);
        //}
        //println!("Variants: {:?}", variants);
        //let server = CompiledState::from(
        //    HashMap::from([("SERVER".to_string(), true), ("CLIENT".to_string(), false), ("UI".to_string(), false)]),
        //);
        //println!("Cancel = {:?}", server.get_problematic_keys(&variants));
        for run in result {
            for variant in &run.outputs{
                let err = collect_errs(variant.clone());
                assert!(err.len() == 0, "Error in run: {:?} with error: {:?}", variant, err);
            }
        }
    }

    #[test]
    fn test_valid_many_variants_1(){
        let path = "../TestSets/ValidStructure/ManyVariants";
        let result = run_paths(&path.to_string(), None);
        for run in result {
            for variant in &run.outputs{
                let err = collect_errs(variant.clone());
                assert!(err.len() == 0, "Error in run: {:?} with error: {:?}", variant, err);
            }
        }
    }
    #[test]
    fn test_invalid_many_variants_1(){
        let path = "../TestSets/InvalidStructure/ManyVariants";
        let result = run_paths(&path.to_string(), None);
        let mut count = 0;
        for run in result {
            for variant in &run.outputs{
                let err = collect_errs(variant.clone());
                count += err.len();
            }
        }
        assert!(count == 1, "Expected one error but got none");
    }
}