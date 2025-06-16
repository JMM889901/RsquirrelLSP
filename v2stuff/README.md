This is split into various components

Core:
>compiler_state: Representations of the squirrel compiler state and methods for comparing states
>mod_loader: Object representing currently identified mods, resulting files can then be pulled

sqparse_errec:
>Fork of unibois preprocessor fork of sqparse to add some limited error recovery to the LEXER, we do not use the parser

preprocessor:
    
>preprocessor: Main module, struct representation of preprocessed files with tools for updating new or modified files   

>parser: Takes the lexed files from sqparse and identifies branches within the file

>identifier identifies relevant terms for each file, produces the map of known terms so comparisons can be optimized as usize/arc instead of direct string comparisons 

>predictor: outputs variants from the set of branches and predicted states