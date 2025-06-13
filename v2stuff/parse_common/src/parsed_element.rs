use std::{cmp::max, collections::HashMap, default, fmt::{Debug, Display}, iter::Map};

use chumsky::span::SimpleSpan;
use sqparse::token::TerminalToken;

use crate::errors::{expected::Expected, substitute::Substituted, DefaultErrorRank, ErrorRank, ParseError};

pub trait StringToken {
    fn as_string(&self) -> &str;
}
impl StringToken for TerminalToken {
    fn as_string(&self) -> &str {
        self.as_str()
    }
}

pub trait CascadePollution {
    fn pollution(&self) -> usize;
    fn child_pollution(&self) -> usize {
        0
    }
    fn calculate_pollution(&self) -> usize;
}
pub trait Visitable {
    fn visit_children(
        &self,
        function: &mut impl FnMut(&Self),
        pre_visit: &mut impl FnMut(&Self) -> bool,
    );
    fn visit(&self, function: &mut impl FnMut(&Self), pre_visit: &mut impl FnMut(&Self) -> bool) {
        if(pre_visit(self)) {
            self.visit_children(function, pre_visit);
            function(self);
        }
    }
}
pub trait ParseVisitable : Sized{
    fn visit_parse_children(
        &self,
        function: &mut impl FnMut(&Parsed<Self>),
        pre_visit: &mut impl FnMut(&Parsed<Self>) -> bool,
    );
    fn visit_parse(this: &Parsed<Self>, function: &mut impl FnMut(&Parsed<Self>), pre_visit: &mut impl FnMut(&Parsed<Self>) -> bool) {
        if(pre_visit(this)) {
            this.item.visit_parse_children(function, pre_visit);
            function(this);
        }
    }
    fn get_errors(this: &Parsed<Self>) -> Vec<Box<dyn ParseError>> {
        this.errors.clone()
    }
}


#[derive(Debug, PartialEq)]
pub struct Parsed<T>{
    pub range: SimpleSpan,
    pub item: T,
    pub polluted: usize, // Worst match that has occured (0 is perfect)
    pub errors: Vec<Box<dyn ParseError>>,
}


impl<T> Parsed<T> where T: ParseVisitable {
    pub fn find_errors(&self) -> Vec<Box<dyn ParseError>> {
        let mut errs: Vec<Box<dyn ParseError + 'static>> = vec![];
        self.visit_parse_children(
            &mut |e| errs.extend(e.find_errors()),
            &mut |_| true,//This should not need to be mutable probably
        );
        errs.extend(self.get_errors());
        errs
    }
    pub fn get_errors(&self) -> Vec<Box<dyn ParseError>> {
        T::get_errors(self)
    }
}
impl<T: ParseVisitable> Parsed<T> {
    pub fn visit_parse_children(
        &self,
        function: &mut impl FnMut(&Parsed<T>),
        pre_visit: &mut impl FnMut(&Parsed<T>) -> bool,
    ) {
        self.item.visit_parse_children(function, pre_visit);
    }
    pub fn visit_parse(
        &self,
        function: &mut impl FnMut(&Parsed<T>),
        pre_visit: &mut impl FnMut(&Parsed<T>) -> bool,
    ) {
        T::visit_parse(self, function, pre_visit);
    }
}

impl<T> Parsed<T> {
    pub fn new(range: SimpleSpan, item: T) -> Self {
        Parsed {
            range,
            item,
            polluted: 0,
            errors: vec![],
        }
    }
    pub fn pollute_from(&mut self, from: &[&dyn CascadePollution]) {
        self.polluted = from.iter().map(|e| e.pollution()).max().unwrap_or(self.polluted);
    }
    pub fn add_error<E: ParseError>(&mut self, error: E) {
        self.polluted = max(self.polluted, error.rank());
        self.errors.push(Box::new(error));
    }
    pub fn map_item<V, F: FnOnce(T) -> V>(self, f: F) -> Parsed<V> {
        Parsed {
            range: self.range,
            item: f(self.item),
            polluted: self.polluted,
            errors: self.errors,
        }
    }
}
impl<T: Expectable + Clone> Parsed<T> {
    pub fn expect(&mut self, expected: Vec<T>) {
        if !expected.iter().any(|e| e.matches(&self.item)) {
            let error = Expected{
                got: self.item.clone(),
                expected,
                range: self.range.clone(),
            };
            self.add_error(error);
        }
    }
}

pub trait Expectable : StringToken + Debug + Send + Sync + 'static{
    fn matches(&self, item: &Self) -> bool;
}
impl Expectable for TerminalToken {
    fn matches(&self, item: &Self) -> bool {
        self == item
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Substitute<T: Expectable + Clone>(T, Vec<T>);
impl<T: Expectable + Clone> Substitute<T> {
    pub fn map(&self, thing: &T) -> Result<(), Option<T>> {
        if self.0.matches(thing) {
            Ok(())
        } else if self.1.iter().any(|e| e.matches(thing)) {
            Err(Some(self.0.clone()))
        } else {
            Err(None) // No substitute, just the thing
        }
    }
    pub fn one(expect: T, substitute: T) -> Self {
        Self(expect, vec![substitute])
    }
    pub fn many(expect: T, substitutes: Vec<T>) -> Self {
        Self(expect, substitutes)
    }
    pub fn take_expect(self) -> T {
        self.0
    }
}

impl<T: Expectable + Clone> Parsed<T> {
    pub fn substitute(
        &mut self,
        map: Vec<Substitute<T>>,
    ){
        let mut expected = vec![];
        let mut out = None;
        for sub in map {//Technically this breaks if you substitute a valid result
            //Literally skill issue, don't do that
            let map_result = sub.map(&self.item);
            if let Ok(()) = map_result{
                return;//All good
            }
            let map_result = map_result.unwrap_err();
            if map_result.is_some(){
                out = map_result;
            }
            expected.push(sub.take_expect());
        }
        if let Some(new) = out {
            let error = Substituted {
                gave: self.item.clone(),
                got: new.clone(),
                expected,
                range: self.range,
            };
            self.add_error(error);
            self.item = new;
            self.polluted = max(self.polluted, DefaultErrorRank::substitute());
        } else {
            let error = Expected {
                got: self.item.clone(),
                expected,
                range: self.range,
            };
            self.add_error(error);
            self.polluted = max(self.polluted, DefaultErrorRank::expected());
        }
    }
}

pub fn default_parsed<T>(
    option: Option<Parsed<T>>,
    default: T,
    range: SimpleSpan,
) -> Parsed<T> {
    option.unwrap_or_else(|| Parsed {
        range,
        item: default,
        polluted: 0,
        errors: vec![],
    })
}