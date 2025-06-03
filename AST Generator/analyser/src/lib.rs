use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt::Debug;
use std::io::Write;
use std::sync::{Arc, RwLock};

use ASTParser::error::Error;
use ConfigAnalyser::get_file_varaints;
use ASTParser::ast::{Element, Type, AST};
use ASTParser::grammar::squirrel_ast;
use ASTParser::SquirrelParse;

pub struct Scope{
    pub range: (usize, usize),
    pub vars: RwLock<Vec<Arc<Variable>>>,//TODO: Hashmap these
    pub types: RwLock<Vec<Arc<TypeDef>>>,
    pub children: RwLock<Vec<Arc<Scope>>>,
    pub errors: RwLock<Vec<Element<LogicError>>>,
    pub parent: Option<Arc<Scope>>,//also used to refer to previous file in load order
    pub global_names: RwLock<HashMap<String, bool>>,
    pub has_return: RwLock<bool>,
}
impl Debug for Scope{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Scope")
            .field("range", &self.range)
            .field("vars", &self.vars)
            .field("types", &self.types)
            .field("children", &self.children)
            .field("errors", &self.errors)
            .field("global_names", &self.global_names)
            .field("has_return", &self.has_return)
            .finish()
    }
}
impl Scope{
    pub fn new(range: (usize, usize)) -> Arc<Self>{
        Arc::new(Scope{
            range,
            vars: RwLock::new(Vec::new()),
            types: RwLock::new(Vec::new()),
            children: RwLock::new(Vec::new()),
            errors: RwLock::new(Vec::new()),
            parent: None,
            global_names: RwLock::new(HashMap::new()),
            has_return: RwLock::new(false),
        })
    }
    pub fn new_parent(range: (usize, usize), parent: Arc<Scope>) -> Arc<Self>{
        Arc::new(Scope{
            range,
            vars: RwLock::new(Vec::new()),
            types: RwLock::new(Vec::new()),
            children: RwLock::new(Vec::new()),
            errors: RwLock::new(Vec::new()),
            parent: Some(parent),
            global_names: RwLock::new(HashMap::new()),
            has_return: RwLock::new(false),
        })
    }
    pub fn add_child(parent: Arc<Self>, range: (usize, usize)) -> Arc<Self>{
        let child = Scope::new_parent(range, parent.clone());
        let mut children = parent.children.write().unwrap();        
        children.push(child.clone());
        return child;
    }
}

#[derive(Debug, Clone)]
pub enum LogicError{
    UndefinedVariableError(String),
    DoesNotReturnError,
    SyntaxError(Error),
}

#[derive(Debug)]
pub struct Variable{//These types are mostly here for later additional implementations, for now they are just ASTs
    pub ast: Element<AST>,//Bad and scary deep copy
    //
    //More probably needed
}

#[derive(Debug)]
pub struct TypeDef{
    pub ast: Element<AST>,//Bad and scary deep copy
    pub name: String,//Not handling types properly yet, just a "does it exist"
}



pub fn generate_asts(text: &String, run_on: &String, filename: &String) -> Vec<(HashMap<String, bool>, Vec<Element<AST>>)> {
    let variants = get_file_varaints(text, run_on, filename);
    let mut asts: Vec<(HashMap<String, bool>, Vec<Element<AST>>)> = Vec::new();
    for variant in variants{
        let parse_data = SquirrelParse::empty();
        let offset = RefCell::new(0);
        let parse = squirrel_ast::file_scope(&variant, &offset, &parse_data);
        asts.push((variant.state.0, parse.unwrap())); //BAD NOT GOOD

    }
    return asts;
}

pub fn get_variable(scope: Arc<Scope>, name: &String) -> Option<Arc<Variable>>{
    let vars = scope.vars.read().unwrap();
    for var in vars.iter(){
        match var.ast.value.as_ref(){
            AST::Function { name: var_name, args, returns, actions } => {
                if var_name.value.as_ref() == name{
                    return Some(var.clone());
                }
            }
            AST:: Declaration { name: var_name, vartype, value } => {
                if var_name.value.as_ref() == name{
                    return Some(var.clone());
                }
            }
            AST::ConstDeclaration { name: var_name, vartype, value } => {
                if var_name.value.as_ref() == name{
                    return Some(var.clone());
                }
            }
            _ => {}
        }
        
    }
    if let Some(parent) = &scope.parent{
        return get_variable(parent.clone(), name);
    }
    return None;
}

pub fn get_type(scope: Arc<Scope>, name: &String) -> Option<Arc<TypeDef>>{
    let types = scope.types.read().unwrap();
    for type_def in types.iter(){
        if type_def.name == *name{
            return Some(type_def.clone());
        }
    }
    if let Some(parent) = &scope.parent{
        return get_type(parent.clone(), name);
    }
    return None;
}

pub fn find_funcs(scope: Arc<Scope>, steps: &Vec<Element<AST>>) {
    for step in steps{
        match step.value.as_ref(){
            AST::Function { name, args, returns, actions } => {
                let mut vars = scope.vars.write().unwrap();
                vars.push(Arc::new(Variable{ast: step.clone()}));//Should functions be in the vars list? they basically are
            },
            _ => {}
        }
    }
}

pub fn analyse_scope(scope: Arc<Scope>, steps: &Vec<Element<AST>>, untyped: bool){
    for step in steps{
        analyse_step(scope.clone(), step, untyped);
    }
}

pub fn analyse_step(scope: Arc<Scope>, step: &Element<AST>, untyped: bool){
    match step.value.as_ref(){
        AST::Global(name) => {
            let mut global_names = scope.global_names.write().unwrap();
            global_names.insert(name.clone(), true);
        }
        AST::Typedef { name, sqtype } => {
            let mut types = scope.types.write().unwrap();
            let name = *name.value.clone();
            types.push(Arc::new(TypeDef{ast: step.clone(), name }));
        }
        
        AST::Declaration { name, vartype, value } => {
            if let Some(value) = value{
                analyse_step(scope.clone(), &value, untyped);
            }
            let mut vars = scope.vars.write().unwrap();
            vars.push(Arc::new(Variable{ast: step.clone()}));
        }
        AST::ConstDeclaration { name, vartype, value } => {
            let mut vars = scope.vars.write().unwrap();
            //Todo: raise error if given a none-constant value
            //TODO: This should be handled earlier than here
            vars.push(Arc::new(Variable{ast: step.clone()}));
        }
        AST::EnumDeclaration { global,  name } => {
            let mut types = scope.types.write().unwrap();
            let name = *name.value.clone();
            types.push(Arc::new(TypeDef{ast: step.clone(), name }));
        }
        AST::StructDeclaration { global, name, attributes } => {
            let mut types = scope.types.write().unwrap();
            let name = *name.value.clone();
            types.push(Arc::new(TypeDef{ast: step.clone(), name }));
        }
        AST::Assignment { var, value } => {
            let vars = scope.vars.read().unwrap();
            analyse_step(scope.clone(), &var, untyped);
            analyse_step(scope.clone(), &value, untyped);
            //Todo: check if var is mutable
        }
        AST::If { condition, actions } => {
            if let Some(condition) = condition{//if without a condition is shorthand for else, this is dumb but it is what most closely 
                analyse_step(scope.clone(), &condition, untyped); //resembles the squirrel functionality
            }
            let scope = Scope::add_child(scope, step.range);
            analyse_scope(scope, actions, untyped);
        }
        AST::While { condition, actions } => {
            let scope = Scope::add_child(scope, step.range);
            if let Some(condition) = condition{
                analyse_step(scope.clone(), &condition, untyped);
            }
            analyse_scope(scope, actions, untyped);
        }
        AST::ForEach { iterators, iterable, actions } => {
            let scope = Scope::add_child(scope, step.range);
            for iterator in iterators{
                analyse_step(scope.clone(), &iterator, untyped);
            }
            analyse_step(scope.clone(), &iterable, untyped);
            analyse_scope(scope, actions, untyped);
        }
        AST::For { init, condition, increment, actions} => {
            let scope = Scope::add_child(scope, step.range);
            if let Some(init) = init{
                analyse_step(scope.clone(), &init, untyped);
            }
            if let Some(condition) = condition{
                analyse_step(scope.clone(), &condition, untyped);
            }
            if let Some(increment) = increment{
                analyse_step(scope.clone(), &increment, untyped);
            }
            analyse_scope(scope, actions, untyped);
        }
        AST::Switch { condition, cases, default } => {
            let scope = Scope::add_child(scope, step.range);
            analyse_step(scope.clone(), &condition, untyped);
            let mut start = step.range.0;
            for (case, actions) in cases{
                for condition in case{
                    analyse_step(scope.clone(), &condition, untyped);
                }
                let range = (actions.first().map(|a| a.range.0).unwrap_or(0),
                    actions.last().map(|a| a.range.0).unwrap_or(0));
                start = range.1;
                let scope = Scope::add_child(scope.clone(), range);
                analyse_scope(scope, actions, untyped);
            }
            if let Some(default) = default{
                let scope = Scope::add_child(scope.clone(), default.last().map(|x| x.range).unwrap_or((start, step.range.1)));
                analyse_scope(scope, default, untyped);
            }
        }
        AST::Return(value) => {
            if let Some(value) = value{
                analyse_step(scope.clone(), &value, untyped);
            }
            let mut has_return = scope.has_return.write().unwrap();
            *has_return = true;
        }
        AST::Break => {
            
        }
        AST::Continue => {
            
        }
        AST::Unreachable => {
            let mut has_return = scope.has_return.write().unwrap();
            *has_return = true;
        }
        AST::Function { name, args, returns, actions } => {
            //let mut vars = scope.vars.write().unwrap();
            //vars.push(Arc::new(Variable{ast: step.clone()}));//Should functions be in the vars list? they basically are
            //drop(vars);
            let scope = Scope::add_child(scope.clone(), step.range);
            for arg in args{
                analyse_step(scope.clone(), &arg, untyped);
            }
            analyse_scope(scope.clone(), actions, untyped);

            if let Some(returns) = returns{
                //analyse_step(scope.clone(), step, untyped);//TODO: type checking
                if returns.value.as_ref() != &Type::Void &&  returns.value.as_ref() != &Type::Var{
                    let has_return = scope.has_return.read().unwrap();
                    if !*has_return{
                        let mut errors = scope.errors.write().unwrap();
                        errors.push(Element::new(LogicError::DoesNotReturnError, returns.range));
                    }
                }
            }


        }
        AST::AnonymousFunction(args, included_vars , actions) => {
            let scope = Scope::add_child(scope, step.range);
            analyse_scope(scope.clone(), actions, untyped);
            for arg in args{
                analyse_step(scope.clone(), &arg, untyped);
            }
            for var in included_vars{
                let name = *var.value.clone();
                let variable = get_variable(scope.clone(), &name);
                if variable.is_none(){
                    let mut errors = scope.errors.write().unwrap();
                    errors.push(Element::new(LogicError::UndefinedVariableError(name), var.range));
                }
            }
        }
        AST::AnonymousScope(actions) => {
            let scope = Scope::add_child(scope, step.range);
            analyse_scope(scope.clone(), actions, untyped);
        }
        AST::Thread(action) => {
            analyse_step(scope, action, untyped);
        }
        AST::Wait(_) => {
            //Todo, either shout if this is not a thread, or mark this as needing to be threaded
            //And complain when the function is called
        }
        AST::Member(left, right ) => {
            analyse_step(scope.clone(), &left, untyped);
            //Todo: Actually test if this is even a struct
        }
        AST::Clone(value) | AST::Neg(value) | AST::Increment(value) | AST::Decrement(value) | AST::Not(value) => {
            analyse_step(scope.clone(), &value, untyped);
        }
        AST::Expect(_, _) | AST::Cast(_, _) => {
            //We dont care about types yet
        }
        AST::Add(left, right) | AST::Sub(left, right) | AST::Mul(left, right) | AST::Div(left, right) | AST::Mod(left, right) | AST::Pow(left, right) |
        AST::Gt(left, right) | AST::Gte(left, right) | AST::Eq(left, right) | AST::Neq(left, right) | AST::Gt(left, right) | AST::Lt(left, right) | AST::Lte(left, right) |
        AST::And(left, right) | AST::Or(left, right) | AST::Xor(left, right)=> {
            analyse_step(scope.clone(), &left, untyped);
            analyse_step(scope.clone(), &right, untyped);
            //Todo: this doesnt test if these operations are valid
        }
        AST::Index(left, right) => {
            analyse_step(scope.clone(), &left, untyped);
            analyse_step(scope.clone(), &right, untyped);
            //todo: this doesnt test if these operations are valid
        }
        AST::FunctionCall { function, args } => {
            analyse_step(scope.clone(), &function, untyped);
            for arg in args{
                analyse_step(scope.clone(), &arg, untyped);
            }
            //Todo: test for thread stuff
        }
        AST::Variable(name) => {
            let var = get_variable(scope.clone(), name.value.as_ref());
            if var.is_none(){
                let mut errors = scope.errors.write().unwrap();
                errors.push(Element::new(LogicError::UndefinedVariableError(*name.value.clone()), name.range));
            }
        }
        AST::Error(err) => {
            let mut errors = scope.errors.write().unwrap();
            errors.push(Element::new(LogicError::SyntaxError(err.clone()), step.range));
        }
        AST::Comment(_) => {
            
        }
        AST::Literal(sqtype) => {
            
        }
        AST::Array(values) => {
            for value in values{
                analyse_step(scope.clone(), &value, untyped);
            }
        }
        AST::Table(keyvalues) => {
            for (key, value) in keyvalues{
                analyse_step(scope.clone(), &key, untyped);
                analyse_step(scope.clone(), &value, untyped);
                //TODO: This doesnt check types and such
            }
        }
        AST::KeyValues(keyvalues) => {
            for (key, value) in keyvalues{
                analyse_step(scope.clone(), &value, untyped);
            
            }
        }
        AST::In(a, b) => {
            analyse_step(scope.clone(), a, untyped);
            analyse_step(scope, b, untyped);
        }
        AST::Ternary(a, b, c) => {
            analyse_step(scope.clone(), a, untyped);
            analyse_step(scope.clone(), b, untyped);
            analyse_step(scope.clone(), c, untyped);
        }
    }
}

pub fn is_expression_mutable(scope: Arc<Scope>, expression: &AST) -> bool{
    todo!()
    //Pain
}

pub fn parse_file(text: &String, run_on: &String, filename: &String, untyped: bool){
    let asts = generate_asts(text, run_on, filename);
    for (state, steps) in asts{
        println!("\n\n\n{:?}\n\n\n\n", state);
        let scope = Scope::new((0, text.len()));
        find_funcs(scope.clone(), &steps);
        analyse_scope(scope.clone(), &steps, untyped);
        for err in collect_errs(scope.clone()){
            if let LogicError::UndefinedVariableError(_) = err.value.as_ref(){
                continue
            }
            println!("{:?} at \n {} \n", err, text[err.range.0 .. err.range.1].to_string());
        }
    }
}

pub fn collect_errs(scope: Arc<Scope>) -> Vec<Element<LogicError>>{
    let mut errors = scope.errors.read().unwrap().clone();
    for child in scope.children.read().unwrap().iter(){
        errors.append(&mut collect_errs(child.clone()));
    }
    return errors;
}

#[cfg(test)]
#[test]
fn run_testfile(){
    use std::fs::read_to_string;
    let text = read_to_string("squirrelFile.nut").unwrap();
    parse_file(&text, &"MP || UI".to_string(), &"name".to_string(), false);

}