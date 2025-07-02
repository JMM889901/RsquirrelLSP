use std::collections::HashMap;

use compiler_state::state::CompilerState;
use preprocessor::VariantID;
use rayon::iter::{IndexedParallelIterator, IntoParallelRefIterator, ParallelIterator};

use crate::{analysis_error::AnalysisError, tree::VariantData, Analyser, PrebuildTrees};
/// Abstract prebuilt trees for a given state, this does not have data associated with it, its effectively a dependency tree for each compiler state
/// See [`crate::tree::VariantData`]
/// This is fine in northstar which only has like 10-20 states, though i'm not sure how well this will scale to vanilla
/// Its a lot of data to store, but i don't think its a *huge* problem

impl Analyser {
    
    pub fn prebuild_trees(&mut self) {
        let mut map = HashMap::new();
        let mut new_trees = HashMap::new();
        for variant in self.variants.iter() {
            match self.config.prebuild_trees {
                PrebuildTrees::Always => {
                    if self.prebuilt_trees.contains_key(&variant.state) {
                        continue; //Already built this
                    }
                    match self.prebuild_tree(variant) {
                        Ok(res) => {
                            new_trees.insert(variant.state.clone(), res);
                        },
                        Err(e) => {
                            self.handle_error(e);
                        }
                    }
                },
                PrebuildTrees::Never => continue,
                PrebuildTrees::After(count) => {
                    let so_far = map.entry(variant.state.clone()).or_insert(0);
                    *so_far += 1;//Does this do what i think it does?
                    if *so_far == count {//No point building multiple times
                        match self.prebuild_tree(variant) {
                            Ok(res) => {
                                new_trees.insert(variant.state.clone(), res);
                            },
                            Err(e) => {
                                self.handle_error(e);
                            }
                        }
                    }
                },
            }
        }
        self.prebuilt_trees = new_trees;
    }


    /// Builds a blank tree for a given state, without any actual data, can be mapped later, intended for caching
    fn prebuild_tree(&self, variant: &VariantID) -> Result<VariantData<()>, AnalysisError> {
        //As the name implies this should be run at the start, so it can be parrallelised
        let iter = self.variants.par_iter().enumerate();
        let res = VariantData::Multi(iter.filter_map(|(idx, v)| {
            if v.state.rejects_explicit(&variant.state) {
                return None; //This variant is not compatible with the target state, so skip it
            }
            Some((v.clone(), idx, ()))
        }).collect::<Vec<_>>());
        let res = res.resolve(variant);

        if res.is_some() {
            return Ok(res);
        }
        Err(AnalysisError::TreeBuildFailed {
            variant: variant.clone(),
            reason: "No matching variants found, even though self is in the tree".to_string(),
        })

        //TODO: I remember i needed to do something here but i forgor :(
        
    }
}