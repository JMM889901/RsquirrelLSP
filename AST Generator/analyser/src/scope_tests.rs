use super::*;


#[cfg(test)]
#[test]
fn test_child_ref(){
    let parent = Scope::new((0, 70));
    let child = Scope::add_child(parent.clone(), (5, 15));
    let child2 = Scope::add_child(parent.clone(), (10, 20));
    let child_child = Scope::add_child(child.clone(), (6, 7));
    let children = parent.all_children_rec();
    assert!(children.len() == 3);
    assert!(children[0].range == (5, 15));
    assert!(children[1].range == (6, 7));
    assert!(children[2].range == (10, 20));//Ideally should be by order of start point


}

#[cfg(test)]
#[test]
fn memtest_children_dropped(){
    let parent = Scope::new((0, 70));
    let child = Scope::add_child(parent.clone(), (5, 15));
    let child2 = Scope::add_child(parent.clone(), (10, 20));
    let child_child = Scope::add_child(child.clone(), (6, 7));
    //let children = parent.all_children_rec();
    //assert!(children.len() == 3);
    let weak_child = Arc::downgrade(&child_child);
    drop(child_child); //Drop the child
    let weak = Arc::downgrade(&child);
    drop(child); 
    let children = parent.all_children_rec();
    println!("{:?}", children.len());
    //Children should not be dropped while hard referenced
    assert!(children.len() == 3);
    drop(children);//Counts as a hard reference

    let child = weak.upgrade();
    assert!(child.is_some());
    let child = child.unwrap();
    child.children.clear();
    let children = parent.all_children_rec();
    println!("{:?}", children.len());
    assert!(children.len() == 2);
    assert!(weak_child.upgrade().is_none());
}



#[cfg(test)]
#[test]
fn test_reference_weak(){
    //let variable = AST::Declaration { name: , vartype: (), value: () };

    use analysis_common::variable::VariableInternal;
    let name = Element::new("test".to_string(), (0, 0));
    let vartype = Element::new(Type::Any, (0, 0));
    let var_ast = AST::Declaration { name: name, vartype: vartype, value: None };
    let name2 = Element::new("test".to_string(), (7, 11));
    let reference = AST::Variable(name2);
    let scope = Scope::new((0, 70));
    let child = Scope::add_child(scope.clone(), (5, 15));
    //
    //let variable = Variable::Variable(VariableInternal{
    //    ast: var_ast,
//
    //})
}