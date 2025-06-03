use std::{sync::Arc, vec};

use crate::ast::{Element, AST};


impl AST{

    pub fn visit<A, R>(step: &Element<AST>, context: &A, 
            leaf_func: &impl Fn(&A, &Vec<R>, &Element<AST>) -> Vec<R>, 
                leaf_func_post: &Option<impl Fn(&A, &Vec<R>, &Element<AST>) -> Vec<R>>) -> Vec<R>
    {
        let mut vec = Vec::new();
        let mut result = leaf_func(context, &vec, step);


        match step.value.as_ref(){
            AST::Declaration { name, vartype, value } =>{
                if let Some(value) = value{
                    result.extend(AST::visit(value, context, leaf_func, leaf_func_post));
                }
            }
            AST::Assignment { var, value } => {
                result.extend(AST::visit(var, context, leaf_func, leaf_func_post));
                result.extend(AST::visit(value, context, leaf_func, leaf_func_post));
            }
            AST::If { condition, actions } => {
                if let Some(condition) = condition{//if without a condition is shorthand for else, this is dumb but it is what most closely 
                    result.extend(AST::visit(condition, context, leaf_func, leaf_func_post)); //resembles the squirrel functionality
                }
                for action in actions{
                    result.extend(AST::visit(action, context, leaf_func, leaf_func_post));
                }
            }
            AST::While { condition, actions } => {
                if let Some(condition) = condition{
                    result.extend(AST::visit(condition, context, leaf_func, leaf_func_post));
                }
                for action in actions{
                    result.extend(AST::visit(action, context, leaf_func, leaf_func_post));
                }
            }
            AST::ForEach { iterators, iterable, actions } => {
                for iterator in iterators{
                    result.extend(AST::visit(iterator, context, leaf_func, leaf_func_post));
                }
                result.extend(AST::visit(iterable, context, leaf_func, leaf_func_post));
                for action in actions{
                    result.extend(AST::visit(action, context, leaf_func, leaf_func_post));
                }
            }
            AST::For { init, condition, increment, actions} => {
                for init in init{
                    result.extend(AST::visit(init, context, leaf_func, leaf_func_post));
                }
                if let Some(condition) = condition{
                    result.extend(AST::visit(condition, context, leaf_func, leaf_func_post));
                }
                for increment in increment{
                    result.extend(AST::visit(increment, context, leaf_func, leaf_func_post));
                }
                for action in actions{
                    result.extend(AST::visit(action, context, leaf_func, leaf_func_post));
                }
            }
            AST::Switch { condition, cases, default } => {
                result.extend(AST::visit(condition, context, leaf_func, leaf_func_post));
                let mut start = step.range.0;
                for (case) in cases{
                    result.extend(AST::visit(case, context, leaf_func, leaf_func_post));
                }
                if let Some(default) = default{
                    result.extend(AST::visit(default, context, leaf_func, leaf_func_post));
                }
            }
            AST::Case { condition, actions } => {
                for condition in condition{
                    result.extend(AST::visit(condition, context, leaf_func, leaf_func_post));
                }
                for action in actions{
                    result.extend(AST::visit(action, context, leaf_func, leaf_func_post));
                }
            }
            AST::Return(value) => {
                if let Some(value) = value{
                    result.extend(AST::visit(value, context, leaf_func, leaf_func_post));
                }
            }
            AST::Function { name, args, returns, actions } => {
                for arg in args{
                    result.extend(AST::visit(arg, context, leaf_func, leaf_func_post));
                }
                for action in actions{
                    result.extend(AST::visit(action, context, leaf_func, leaf_func_post));
                }
            }
            AST::AnonymousFunction(args, included_vars , actions) => {
                for arg in args{
                    result.extend(AST::visit(arg, context, leaf_func, leaf_func_post));
                }
                for action in actions{
                    result.extend(AST::visit(action, context, leaf_func, leaf_func_post));
                }
            }
            AST::AnonymousScope(actions) => {
                for action in actions{
                    result.extend(AST::visit(action, context, leaf_func, leaf_func_post));
                }
            }
            AST::Try { actions, catch } => {
                result.extend(AST::visit(actions, context, leaf_func, leaf_func_post));
                result.extend(AST::visit(catch, context, leaf_func, leaf_func_post));
            }
            AST::Catch { exception, actions } => {
                if let Some(exception) = exception{
                    result.extend(AST::visit(exception, context, leaf_func, leaf_func_post));
                }
                for action in actions{
                    result.extend(AST::visit(action, context, leaf_func, leaf_func_post));
                }
            }
            AST::Thread(action) => {
                result.extend(AST::visit(action, context, leaf_func, leaf_func_post));
            }
            AST::Member(left, right ) => {
                result.extend(AST::visit(left, context, leaf_func, leaf_func_post));
            }
            AST::Clone(value) | AST::Neg(value) | AST::Increment(value) | AST::Decrement(value) | AST::Not(value) => {
                result.extend(AST::visit(value, context, leaf_func, leaf_func_post));
            }
            AST::Expect(_, _) | AST::Cast(_, _) => {
                //We dont care about types yet
            }
            AST::Add(left, right) | AST::Sub(left, right) | AST::Mul(left, right) | AST::Div(left, right) | AST::Mod(left, right) | AST::Pow(left, right) |
            AST::Gt(left, right) | AST::Gte(left, right) | AST::Eq(left, right) | AST::Neq(left, right) | AST::Gt(left, right) | AST::Lt(left, right) | AST::Lte(left, right) |
            AST::And(left, right) | AST::Or(left, right) | AST::Xor(left, right)=> {
                result.extend(AST::visit(left, context, leaf_func, leaf_func_post));
                result.extend(AST::visit(right, context, leaf_func, leaf_func_post));
            }
            AST::Index(left, right) => {
                result.extend(AST::visit(left, context, leaf_func, leaf_func_post));
                result.extend(AST::visit(right, context, leaf_func, leaf_func_post));
            }
            AST::FunctionCall { function, args } => {
                result.extend(AST::visit(function, context, leaf_func, leaf_func_post));
                for arg in args{
                    result.extend(AST::visit(arg, context, leaf_func, leaf_func_post));
                }
            }
            AST::Array(values) => {
                for value in values{
                    result.extend(AST::visit(value, context, leaf_func, leaf_func_post));
                }
            }
            AST::Table(keyvalues) => {
                for (key, value) in keyvalues{
                    result.extend(AST::visit(key, context, leaf_func, leaf_func_post));
                    result.extend(AST::visit(value, context, leaf_func, leaf_func_post));
                }
            }
            AST::KeyValues(keyvalues) => {
                for (key, value) in keyvalues{
                    result.extend(AST::visit(value, context, leaf_func, leaf_func_post));
                }
            }
            AST::In(a, b) => {
                result.extend(AST::visit(a, context, leaf_func, leaf_func_post));
                result.extend(AST::visit(b, context, leaf_func, leaf_func_post));
            }
            AST::Ternary(a, b, c) => {
                result.extend(AST::visit(a, context, leaf_func, leaf_func_post));
                result.extend(AST::visit(b, context, leaf_func, leaf_func_post));
                result.extend(AST::visit(c, context, leaf_func, leaf_func_post));
            }
            AST::ConstDeclaration { global, name, vartype, value } => {
                result.extend(AST::visit(value, context, leaf_func, leaf_func_post));
            }
            AST::StructDeclaration { global, name, attributes } => {
                for attribute in attributes{
                    result.extend(AST::visit(attribute, context, leaf_func, leaf_func_post));
                }
            }
            AST::Throw(value) | AST::Delete(value) => {
                result.extend(AST::visit(value, context, leaf_func, leaf_func_post));
            }
            _ => {}
        }


        if let Some(leaf_func_post) = leaf_func_post{
            result.extend(leaf_func_post(context, &result, step));
        }
        
        return result;
    }

}

//pub fn analyse_scope(scope: Arc<Scope>, steps: &Vec<Element<AST>>, untyped: bool){
//    for step in steps{
//        analyse_step(scope.clone(), step, untyped);
//    }
//}
//
//pub fn analyse_step(scope: Arc<Scope>, step: &Element<AST>, untyped: bool){
//    match step.value.as_ref(){
//        AST::Global(name) => {
//            let mut global_names = scope.global_names.write().unwrap();
//            global_names.insert(name.clone(), true);
//        }
//        AST::Typedef { name, sqtype } => {
//            let mut types = scope.types.write().unwrap();
//            let name = *name.value.clone();
//            types.push(Arc::new(TypeDef{ast: step.clone(), name }));
//        }
//        
//        AST::Declaration { name, vartype, value } => {
//            if let Some(value) = value{
//                analyse_step(scope.clone(), &value, untyped);
//            }
//            let mut vars = scope.vars.write().unwrap();
//            vars.push(Arc::new(Variable{ast: step.clone()}));
//        }
//        AST::ConstDeclaration { name, vartype, value } => {
//            let mut vars = scope.vars.write().unwrap();
//            //Todo: raise error if given a none-constant value
//            //TODO: This should be handled earlier than here
//            vars.push(Arc::new(Variable{ast: step.clone()}));
//        }
//        AST::EnumDeclaration { global,  name } => {
//            let mut types = scope.types.write().unwrap();
//            let name = *name.value.clone();
//            types.push(Arc::new(TypeDef{ast: step.clone(), name }));
//        }
//        AST::StructDeclaration { global, name, attributes } => {
//            let mut types = scope.types.write().unwrap();
//            let name = *name.value.clone();
//            types.push(Arc::new(TypeDef{ast: step.clone(), name }));
//        }
//        AST::Assignment { var, value } => {
//            let vars = scope.vars.read().unwrap();
//            analyse_step(scope.clone(), &var, untyped);
//            analyse_step(scope.clone(), &value, untyped);
//            //Todo: check if var is mutable
//        }
//        AST::If { condition, actions } => {
//            if let Some(condition) = condition{//if without a condition is shorthand for else, this is dumb but it is what most closely 
//                analyse_step(scope.clone(), &condition, untyped); //resembles the squirrel functionality
//            }
//            let scope = Scope::add_child(scope, step.range);
//            analyse_scope(scope, actions, untyped);
//        }
//        AST::While { condition, actions } => {
//            let scope = Scope::add_child(scope, step.range);
//            if let Some(condition) = condition{
//                analyse_step(scope.clone(), &condition, untyped);
//            }
//            analyse_scope(scope, actions, untyped);
//        }
//        AST::ForEach { iterators, iterable, actions } => {
//            let scope = Scope::add_child(scope, step.range);
//            for iterator in iterators{
//                analyse_step(scope.clone(), &iterator, untyped);
//            }
//            analyse_step(scope.clone(), &iterable, untyped);
//            analyse_scope(scope, actions, untyped);
//        }
//        AST::For { init, condition, increment, actions} => {
//            let scope = Scope::add_child(scope, step.range);
//            if let Some(init) = init{
//                analyse_step(scope.clone(), &init, untyped);
//            }
//            if let Some(condition) = condition{
//                analyse_step(scope.clone(), &condition, untyped);
//            }
//            if let Some(increment) = increment{
//                analyse_step(scope.clone(), &increment, untyped);
//            }
//            analyse_scope(scope, actions, untyped);
//        }
//        AST::Switch { condition, cases, default } => {
//            let scope = Scope::add_child(scope, step.range);
//            analyse_step(scope.clone(), &condition, untyped);
//            let mut start = step.range.0;
//            for (case, actions) in cases{
//                for condition in case{
//                    analyse_step(scope.clone(), &condition, untyped);
//                }
//                let range = (actions.first().map(|a| a.range.0).unwrap_or(0),
//                    actions.last().map(|a| a.range.0).unwrap_or(0));
//                start = range.1;
//                let scope = Scope::add_child(scope.clone(), range);
//                analyse_scope(scope, actions, untyped);
//            }
//            if let Some(default) = default{
//                let scope = Scope::add_child(scope.clone(), default.last().map(|x| x.range).unwrap_or((start, step.range.1)));
//                analyse_scope(scope, default, untyped);
//            }
//        }
//        AST::Return(value) => {
//            if let Some(value) = value{
//                analyse_step(scope.clone(), &value, untyped);
//            }
//            let mut has_return = scope.has_return.write().unwrap();
//            *has_return = true;
//        }
//        AST::Break => {
//            
//        }
//        AST::Continue => {
//            
//        }
//        AST::Unreachable => {
//            let mut has_return = scope.has_return.write().unwrap();
//            *has_return = true;
//        }
//        AST::Function { name, args, returns, actions } => {
//            //let mut vars = scope.vars.write().unwrap();
//            //vars.push(Arc::new(Variable{ast: step.clone()}));//Should functions be in the vars list? they basically are
//            //drop(vars);
//            let scope = Scope::add_child(scope.clone(), step.range);
//            for arg in args{
//                analyse_step(scope.clone(), &arg, untyped);
//            }
//            analyse_scope(scope.clone(), actions, untyped);
//
//            if let Some(returns) = returns{
//                //analyse_step(scope.clone(), step, untyped);//TODO: type checking
//                if returns.value.as_ref() != &Type::Void &&  returns.value.as_ref() != &Type::Var{
//                    let has_return = scope.has_return.read().unwrap();
//                    if !*has_return{
//                        let mut errors = scope.errors.write().unwrap();
//                        errors.push(Element::new(LogicError::DoesNotReturnError, returns.range));
//                    }
//                }
//            }
//
//
//        }
//        AST::AnonymousFunction(args, included_vars , actions) => {
//            let scope = Scope::add_child(scope, step.range);
//            analyse_scope(scope.clone(), actions, untyped);
//            for arg in args{
//                analyse_step(scope.clone(), &arg, untyped);
//            }
//            for var in included_vars{
//                let name = *var.value.clone();
//                let variable = get_variable(scope.clone(), &name);
//                if variable.is_none(){
//                    let mut errors = scope.errors.write().unwrap();
//                    errors.push(Element::new(LogicError::UndefinedVariableError(name), var.range));
//                }
//            }
//        }
//        AST::AnonymousScope(actions) => {
//            let scope = Scope::add_child(scope, step.range);
//            analyse_scope(scope.clone(), actions, untyped);
//        }
//        AST::Thread(action) => {
//            analyse_step(scope, action, untyped);
//        }
//        AST::Wait(_) => {
//            //Todo, either shout if this is not a thread, or mark this as needing to be threaded
//            //And complain when the function is called
//        }
//        AST::Member(left, right ) => {
//            analyse_step(scope.clone(), &left, untyped);
//            //Todo: Actually test if this is even a struct
//        }
//        AST::Clone(value) | AST::Neg(value) | AST::Increment(value) | AST::Decrement(value) | AST::Not(value) => {
//            analyse_step(scope.clone(), &value, untyped);
//        }
//        AST::Expect(_, _) | AST::Cast(_, _) => {
//            //We dont care about types yet
//        }
//        AST::Add(left, right) | AST::Sub(left, right) | AST::Mul(left, right) | AST::Div(left, right) | AST::Mod(left, right) | AST::Pow(left, right) |
//        AST::Gt(left, right) | AST::Gte(left, right) | AST::Eq(left, right) | AST::Neq(left, right) | AST::Gt(left, right) | AST::Lt(left, right) | AST::Lte(left, right) |
//        AST::And(left, right) | AST::Or(left, right) | AST::Xor(left, right)=> {
//            analyse_step(scope.clone(), &left, untyped);
//            analyse_step(scope.clone(), &right, untyped);
//            //Todo: this doesnt test if these operations are valid
//        }
//        AST::Index(left, right) => {
//            analyse_step(scope.clone(), &left, untyped);
//            analyse_step(scope.clone(), &right, untyped);
//            //todo: this doesnt test if these operations are valid
//        }
//        AST::FunctionCall { function, args } => {
//            analyse_step(scope.clone(), &function, untyped);
//            for arg in args{
//                analyse_step(scope.clone(), &arg, untyped);
//            }
//            //Todo: test for thread stuff
//        }
//        AST::Variable(name) => {
//            let var = get_variable(scope.clone(), name.value.as_ref());
//            if var.is_none(){
//                let mut errors = scope.errors.write().unwrap();
//                errors.push(Element::new(LogicError::UndefinedVariableError(*name.value.clone()), name.range));
//            }
//        }
//        AST::Error(err) => {
//            let mut errors = scope.errors.write().unwrap();
//            errors.push(Element::new(LogicError::SyntaxError(err.clone()), step.range));
//        }
//        AST::Comment(_) => {
//            
//        }
//        AST::Literal(sqtype) => {
//            
//        }
//        AST::Array(values) => {
//            for value in values{
//                analyse_step(scope.clone(), &value, untyped);
//            }
//        }
//        AST::Table(keyvalues) => {
//            for (key, value) in keyvalues{
//                analyse_step(scope.clone(), &key, untyped);
//                analyse_step(scope.clone(), &value, untyped);
//                //TODO: This doesnt check types and such
//            }
//        }
//        AST::KeyValues(keyvalues) => {
//            for (key, value) in keyvalues{
//                analyse_step(scope.clone(), &value, untyped);
//            
//            }
//        }
//        AST::In(a, b) => {
//            analyse_step(scope.clone(), a, untyped);
//            analyse_step(scope, b, untyped);
//        }
//        AST::Ternary(a, b, c) => {
//            analyse_step(scope.clone(), a, untyped);
//            analyse_step(scope.clone(), b, untyped);
//            analyse_step(scope.clone(), c, untyped);
//        }
//    }
//}
