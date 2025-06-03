use std::{collections::HashMap, sync::Arc};
use std::fmt::Display;
use crate::error::Error;
use crate::external_resources::ExternalResourceEntry;



#[derive(Debug, Clone, PartialEq)]
pub struct Element<A> {
    pub range: (usize, usize),
    pub value: Box<A>
}
impl<A> Element<A> {
    pub fn new(thing: A, pos: (usize, usize)) -> Element<A>{
        Element{
            range: pos,
            value: Box::new(thing)
        }
    }
    pub fn newb(pos: (usize, usize), thing: A) -> Element<A>{//Because borrow chicanery
        Element{
            range: pos,
            value: Box::new(thing)
        }
    }
}
impl<A: Clone> Element<A> {
    pub fn unwrap_v(self) -> A {
        let value = self.value;
        return *value;
        //match Arc::try_unwrap(value) {
        //    Ok(a) => a,
        //    Err(value) => A::clone(&value)
        //}
        
    }
    pub fn unwrap(&self) -> (A, (usize, usize)) {
        let value = self.value.clone();
        return (*value, self.range);
        //match Arc::try_unwrap(value) {
        //    Ok(a) => (a, self.range),
        //    Err(value) => (A::clone(&value), self.range)
        //}
        
    }
}

impl Display for Element<String> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.value)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum AST { //This is an unresolved/validated AST
    Comment(String),//TODO: Should probably add a distinct docstring ast
    //External
    Global(String),
    GlobalizeAllFunctions,
    //Statement
    Declaration{
        name: Element<String>,
        vartype: Element<Type>,
        value: Option<Element<AST>>
    },
    ConstDeclaration{
        global: bool,
        name: Element<String>,
        vartype: Option<Element<Type>>,
        value: Element<AST>
    },
    EnumDeclaration{
        global: bool,
        name: Element<String>,
        //global: bool,
    },
    StructDeclaration{
        global: bool,
        name: Element<String>,
        attributes: Vec<(Element<AST>)>
    },
    Assignment{
        var: Element<AST>,
        value: Element<AST>
    },
    Typedef{
        global: bool,
        name: Element<String>,
        sqtype: Element<Type>,
    },
    // Functions
    Function{
        name: Element<String>,
        args: Vec<Element<AST>>, //Value in this case is a default value
        returns: Option<Element<Type>>, //Cannot just assume void, untyped functions are a thing
        actions: Vec<Element<AST>>
    },
    Try{
        actions: Element<AST>,//AnonymousScope
        catch: Element<AST>
    },//As far as i know RSquirrel try/catch only has one output
    Catch{
        exception: Option<Element<AST>>,//Declaration
        actions: Vec<Element<AST>>,
    },
    Return(Option<Element<AST>>),
    Unreachable,//This is functionally return(None), typically only used when returning with an IF/Else but squirrel complains anyway
    Break,
    Continue,
    FunctionCall{
        function: Element<AST>,
        args: Vec<Element<AST>>,
    },
    //Control flow
    AnonymousScope(Vec<Element<AST>>),
    Thread(Element<AST>),//This shouldnt just accept expression, but its easier than making a load of new rules
    Wait(String), //this only really keeps the string to be non-destructive, it doesnt really need to store anything
    If{
        condition: Option<Element<AST>>, //None indicates catchall such as else
        actions: Vec<Element<AST>>,//Thankfully, squirrel actually handles elifs and ifs exactly like ifs
    },//IE if all returns exist in an if (or even an else) it will complain
    While{
        condition: Option<Element<AST>>,
        actions: Vec<Element<AST>>,
    },
    ForEach{
        iterators: Vec<Element<AST>>,
        iterable: Element<AST>,
        actions: Vec<Element<AST>>,
    },
    For{
        init: Vec<Element<AST>>,
        condition: Option<Element<AST>>,
        increment: Vec<Element<AST>>,
        actions: Vec<Element<AST>>,
    },
    Switch{
        condition: Element<AST>,
        cases: Vec<Element<AST>>,//The ONLY ast that should ever be here is Case
        default: Option<Element<AST>>,//Case with no conditions
    },
    Case{
        condition: Vec<Element<AST>>,
        actions: Vec<Element<AST>>,
    },
    Empty, //This is used for empty statements, mostly for default conditions

    Clone(Element<AST>),
    //Expression
    Add(Element<AST>, Element<AST>),
    Sub(Element<AST>, Element<AST>),
    Mul(Element<AST>, Element<AST>),
    Div(Element<AST>, Element<AST>),

    Neg(Element<AST>),

    And(Element<AST>, Element<AST>),
    Or(Element<AST>, Element<AST>),
    Not(Element<AST>),
    Xor(Element<AST>, Element<AST>),

    Eq(Element<AST>, Element<AST>),
    Neq(Element<AST>, Element<AST>),
    Gt(Element<AST>, Element<AST>),
    Lt(Element<AST>, Element<AST>),
    Gte(Element<AST>, Element<AST>),
    Lte(Element<AST>, Element<AST>),

    Mod(Element<AST>, Element<AST>),
    Pow(Element<AST>, Element<AST>),

    Increment(Element<AST>),
    Decrement(Element<AST>),

    Index(Element<AST>, Element<AST>),
    Member(Element<AST>, String),
    In(Element<AST>, Element<AST>),

    Expect(Element<Type>, Element<AST>),
    Cast(Element<AST>, Element<Type>),//Identical for all we care about, in practice Expect assumes the type already is something, only used for var

    Ternary(Element<AST>, Element<AST>, Element<AST>),

    AnonymousFunction(Vec<Element<AST>>, Vec<Element<String>>, Vec<Element<AST>>),
    KeyValues(Vec<(Element<String>, Element<AST>)>),

    Delete(Element<AST>),
    Throw(Element<AST>),

    Error(Error),
    Literal(Type),//Guh
    Array(Vec<Element<AST>>),//Refers to the expression of an array, not the type array, IE refers to [1,2] and not array<int> x =
    Table(Vec<(Element<AST>, Element<AST>)>),
    //ContextWrapper(ContextWrapper<AST>),
    Variable(Element<String>),
   // FunctionRef(ArcLockElement<ContextWrapper<Function>>), Functions are now variables, im sure this will have no downsides

    ExternalReference(ExternalResourceEntry) //Used for nativefuncs.json and the likes, Its a little weird but this is probably the right way
}
impl AST{
    pub fn fallback_literal() -> Self{
        AST::Literal(Type::Var)
    }
    pub fn get_name(&self) -> String{
        match self {
            //AST::Function { name, .. } => *name.value.clone(),
            //AST::Declaration { name, .. } => *name.value.clone(),
            //AST::Variable(name) => *name.value.clone(),
            //AST::ConstDeclaration { name, .. } => *name.value.clone(),
            //AST::EnumDeclaration { name, .. } => *name.value.clone(),
            //AST::StructDeclaration { name, .. } => *name.value.clone(),
            //AST::Global(name) => name.clone(),
            _ => panic!("Not a function or variable")
        }
    }
}


impl Element<AST> {
    pub fn text_none_rec(&self) -> String {
        match self.value.as_ref(){
            AST::Function { name, args, returns, actions } => {
                let mut text = String::new();
                text.push_str(&name.value);
                text.push_str("(");
                for arg in args {
                    text.push_str(&arg.text_none_rec());
                    text.push_str(", ");
                }
                if !args.is_empty() {
                    text.pop();//Bit skethcy
                    text.pop();//Remove the last comma and space
                }
                text.push_str(") ");
                text.push_str(&format!("{:?}", name.range));
                return text
            },
            AST::Declaration { name, vartype, value } => {
                let mut text = String::new();
                text.push_str(&format!("{:?} ", vartype.value));
                text.push_str(&name.value);

                if let Some(value) = value {
                    text.push_str(" = ");
                    text.push_str(&value.text_none_rec());
                }
                return text
            },
            AST::ConstDeclaration { global, name, vartype, value } => {
                let mut text = String::new();
                if *global {
                    text.push_str("global ");
                }
                text.push_str("const ");
                if let Some(vartype) = vartype {
                    text.push_str(&format!("{:?} ", vartype.value));
                }
                text.push_str(&name.value);

                text.push_str(" = ");
                text.push_str(&value.text_none_rec());
                return text
            },
            AST::EnumDeclaration { global, name } => {
                let mut text = String::new();
                if *global {
                    text.push_str("global ");
                }
                text.push_str(&name.value);
                return text
            },
            AST::StructDeclaration { global, name, attributes } => {
                let mut text = String::new();
                if *global {
                    text.push_str("global ");
                }
                text.push_str(&name.value);
                text.push_str("{ ");
                for attr in attributes {
                    text.push_str(&attr.text_none_rec());
                    text.push_str(", ");
                }
                if !attributes.is_empty() {
                    text.pop();//Bit skethcy
                    text.pop();//Remove the last comma and space
                }
                text.push_str("}");
                return text
            },
            AST::Variable(wuh) => {
                return format!("{}", wuh.value)
            },
            AST::Global(wuh) => {
                return format!("global function {}", wuh)
            },
            _ => return format!("{:?}", self)
        }
    }
}


#[derive(Debug, Clone, PartialEq)]
pub enum ExternalType{
    Func,
    Var,
    Struct,
    //Enum
}



#[derive(PartialEq, Clone, Debug)]
pub enum Type { //I should look into splitting this to types as in the declaration(hence why resolvable from string) and types as in the functionality (the result of a given expression)
    Int,
    Float,
    Bool,
    String,
    Asset,
    Vector,
    SimpleArray,
    SimpleTable,
    FixedArray(Box<Type>, usize),
    Array(Box<Type>),
    Table(Box<Type>, Box<Type>),
    Functionref(Box<Type>, Vec<Argument>),
    Ref(Box<Type>),
    Entity,
    Struct(Arc<Struct>),
    KeyValues(Vec<(Element<String>, Element<AST>)>), //Essentially an anonymous struct
    Empty, //Only used for tables and such
    Void,
    Any,
    Local,
    Var,
    OrNull(Box<Type>), //This would imply other types cant be null, they can, i dont know why this even exists honestly
    Named(String),//Thankfully squirrel does not have complex structs with type variants
    NotResolvable, //Very bad and not good, this hopefully wont ever happen
}
impl Display for AST{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self{
            _ => write!(f, "{:?}", self),
            //AST::Try { actions, catch } => write!(f, "Try: {:#?} Catch: {:#?}", actions, catch),
            //AST::Catch { exception, actions } => write!(f, "Catch: {:?} {:#?}", exception, actions),
            //AST::Delete(a) => write!(f, "Delete: {}", a.value),
            //AST::Throw(a) => write!(f, "Throw: {}", a.value),
            //AST::GlobalizeAllFunctions => write!(f, "GlobalizeAllFunctions"),
            //AST::Comment(c) => write!(f, "Comment: {}", c),
            //AST::Global(g) => write!(f, "Global: {}", g),
            //AST::Declaration{name, vartype, value} => write!(f, "Declaration: {} {:?} = {:?}", name.value, vartype.value, value),
            //AST::ConstDeclaration{global, name, vartype, value} => write!(f, "ConstDeclaration: {} {:?} = {:?}", name.value, vartype.as_ref().map(|x| x.value.clone()).unwrap_or_else(|| Box::new(Type::Named("".to_string()))), value),
            //AST::EnumDeclaration{global, name} => write!(f, "EnumDeclaration: {}", name.value),
            //AST::StructDeclaration{global, name, attributes} => write!(f, "StructDeclaration: {} {} {:#?}", if *global {"global"} else {""}, name.value, attributes),
            //AST::Assignment{var, value} => write!(f, "Assignment: {} = {}", var.value, value.value),
            //AST::Typedef{global, name, sqtype} => write!(f, "Typedef: {} {:?}", name.value, sqtype.value),
            //AST::Function{name, args, returns, actions} => write!(f, "Function: {} {:?} -> {:?} \n {:#?}", name.value, args, returns, actions),
            //AST::Return(value) => write!(f, "Return: {:?}", value),
            //AST::FunctionCall{function, args} => write!(f, "FunctionCall: {} {:?}", function.value, args),
            //AST::Add(a, b) => write!(f, "Add: {} + {}", a.value, b.value),
            //AST::Sub(a, b) => write!(f, "Sub: {} - {}", a.value, b.value),
            //AST::Mul(a, b) => write!(f, "Mul: {} * {}", a.value, b.value),
            //AST::Div(a, b) => write!(f, "Div: {} / {}", a.value, b.value),
            //AST::Neg(a) => write!(f, "Neg: -{}", a.value),
            //AST::And(a, b) => write!(f, "And: {} && {}", a.value, b.value),
            //AST::Or(a, b) => write!(f, "Or: {} || {}", a.value, b.value),
            //AST::Not(a) => write!(f, "Not: !{}", a.value),
            //AST::Xor(a, b) => write!(f, "Xor: {} ^ {}", a.value, b.value),
            //AST::Eq(a, b) => write!(f, "Eq: {} == {}", a.value, b.value),
            //AST::Neq(a, b) => write!(f, "Neq: {} != {}", a.value, b.value),
            //AST::Gt(a, b) => write!(f, "Gt: {} > {}", a.value, b.value),
            //AST::Lt(a, b) => write!(f, "Lt: {} < {}", a.value, b.value),
            //AST::Gte(a, b) => write!(f, "Gte: {} >= {}", a.value, b.value),
            //AST::Lte(a, b) => write!(f, "Lte: {} <= {}", a.value, b.value),
            //AST::Mod(a, b) => write!(f, "Mod: {} % {}", a.value, b.value),
            //AST::Pow(a, b) => write!(f, "Pow: {} ^ {}", a.value, b.value),
            //AST::Increment(a) => write!(f, "Increment: {}++", a.value),
            //AST::Decrement(a) => write!(f, "Decrement: {}--", a.value),
            //AST::Index(a, b) => write!(f, "Index: {}[{}]", a.value, b.value),
            //AST::Member(a, b) => write!(f, "Member: {}.{}", a.value, b),
            //AST::Expect(a, b) => write!(f, "Expect: {:?} as {}", a.value, b.value),
            //AST::KeyValues(kv) => write!(f, "KeyValues: {:?}", kv),
            //AST::Error(e) => write!(f, "Error: {:?}", e),
            //AST::Literal(t) => write!(f, "Literal: {:?}", t),
            //AST::Array(a) => write!(f, "Array: {:?}", a),
            //AST::Table(t) => write!(f, "Table: {:?}", t),
            //AST::Variable(v) => write!(f, "Variable: {}", v.value),
            //AST::Unreachable => write!(f, "Unreachable"),
            //AST::Break => write!(f, "Break"),
            //AST::Continue => write!(f, "Continue"),
            //AST::If{condition, actions} => write!(f, "If: {:?} {:#?}", condition, actions),
            //AST::While{condition, actions} => write!(f, "While: {:?} {:#?}", condition, actions),
            //AST::ForEach{iterators, iterable, actions} => write!(f, "ForEach: {:?} {} {:#?}", iterators, iterable.value, actions),
            //AST::For{init, condition, increment, actions} => write!(f, "For: {:?} {:?} {:?} {:#?}", init, condition, increment, actions),
            //AST::Cast(a, b) => write!(f, "Cast: {} as {:#?}", a.value, b.value),
            //AST::AnonymousScope(actions) => write!(f, "AnonymousScope: {:#?}", actions),
            //AST::AnonymousFunction(args, returns, actions) => write!(f, "AnonymousFunction: {:?} -> {:?} {:#?}", args, returns, actions),
            //AST::Thread(a) => write!(f, "Thread: {:#?}", a.value),
            //AST::Clone(a) => write!(f, "Clone: {:#?}", a.value),
            //AST::Wait(a) => write!(f, "Wait: {}", a),
            //AST::Switch{condition, cases, default} => write!(f, "Switch: {} {:#?} {:?}", condition.value, cases, default),
            //AST::Case{condition, actions} => write!(f, "Case: {:?} {:#?}", condition, actions),
            //AST::Ternary(a, b, c) => write!(f, "Ternary: if {} then {} else {}", a.value, b.value, c.value),
            //AST::In(a, b) => write!(f, "{} is in {}", a.value, b.value),
            //AST::ExternalReference(e) => write!(f, "ExternalReference: {:?}", e),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Argument{
    pub arg_type: Type,
    pub default: bool,
}

#[derive(Debug, PartialEq, Clone)]
pub struct Struct {
    pub declaration: Arc<AST>,
    pub source: String,
    pub name: String,
    pub attributes: Arc<HashMap<String, AST>>,
    pub singleton: bool,
    pub global: bool,
}