use lazy_static::lazy_static;
use serde::de::value;
use std::{
    cell::{Ref, RefCell},
    collections::HashMap,
    fmt::{self, Debug},
    rc::Rc,
};

use crate::ast::token::{datatype::DataType, literal::Literal};

use std::sync::Mutex;

pub struct Interface {
    pub name: String,
    pub args: Vec<DataType>,
    pub ret: DataType,
    //pub f: Mutex<Box<dyn Fn(&mut Literal, &mut Literal) -> Literal + Send + Sync>>,
    pub f: Box<dyn Fn(&mut Literal, &mut Literal) -> Literal>,
}

#[derive(Debug)]
pub struct Stdlib {
    pub interfaces: HashMap<DataType, Vec<Interface>>,
}

impl Debug for Interface {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Interface: {} -> {:?} -> {:?}",
            self.name, self.args, self.ret
        )
    }
}

impl Stdlib {
    pub fn new() -> Self {
        Self {
            interfaces: HashMap::new(),
        }
    }
    pub fn get_interfaces(&self, dtype: &DataType) -> Option<&Vec<Interface>> {
        self.interfaces.get(dtype)
    }
    pub fn interfaces(&mut self, dtype: &DataType) -> &Vec<Interface> {
        if self.interfaces.contains_key(&dtype) {
            return self.interfaces.get(&dtype).unwrap();
        }

        match dtype.clone() {
            DataType::List(t) => {
                let mut interfaces = vec![];
                interfaces.push(Interface {
                    name: "len".to_string(),
                    args: vec![],
                    ret: DataType::Integer,
                    f: Box::new(|list, _| match list {
                        Literal::List(_, v) => Literal::Int(v.len() as i64),
                        _ => panic!("Expected List"),
                    }),
                });
                interfaces.push(Interface {
                    name: "push".to_string(),
                    args: vec![t.as_ref().clone()],
                    ret: DataType::Void,
                    f: Box::new(|list, v| {
                        let v = v.to_tuple().unwrap();
                        if let Literal::List(dtype, list) = list {
                            if v.len() != 1 {
                                panic!("Expected Tuple with one element");
                            }
                            if dtype != &v[0].get_datatype() {
                                panic!("List of type {} can only accept values of the same type ({} given)", dtype, v[0].get_datatype());
                            }
                            list.push(v[0].clone());
                        }
                        else {
                            panic!("Expected List") 
                        }
                        Literal::Null
                    }),
                });
                interfaces.push(Interface {
                    name: "remove".to_string(),
                    args: vec![DataType::Integer],
                    ret: t.as_ref().clone(),
                    f: Box::new(|list, v| {
                        let v = v.to_integer().unwrap();
                        if let Literal::List(dtype, list) = list {
                            if v < 0 || v as usize >= list.len() {
                                panic!("Index out of bounds");
                            }
                            return list.remove(v as usize);
                        } else {
                            panic!("Expected List")
                        }
                    }),
                });
                interfaces.push(Interface {
                    name: "set".to_string(),
                    args: vec![DataType::Integer, t.as_ref().clone()],
                    ret: DataType::Void,
                    f: Box::new(|list, v| {
                        let v = v.to_tuple().unwrap();
                        if v.len() != 2 {
                            panic!("Expected Tuple with two elements");
                        }
                        let idx = v[0].to_integer().unwrap() as usize;
                        if let Literal::List(dtype, list) = list {
                            if idx >= list.len() {
                                panic!("Index out of bounds");
                            }
                            if dtype != &v[1].get_datatype() {
                                panic!("List of type {:?} can only accept values of the same type ({} given)", dtype, v[1].get_datatype());
                            }
                            list[idx] = v[1].clone();
                        }
                        else {
                            panic!("Expected List")
                        }
                        Literal::Null
                    }),
                });
                interfaces.push(Interface {
                    name: "at".to_string(),
                    args: vec![DataType::Integer],
                    ret: t.as_ref().clone(),
                    f: Box::new(|list, v| {
                        let v = v.to_tuple().unwrap();
                        let v = v.first().unwrap().to_integer().unwrap();
                        if let Literal::List(dtype, list) = list {
                            if v < 0 || v as usize >= list.len() {
                                panic!("Index out of bounds: {}", v);
                            }
                            return list[v as usize].clone();
                        } else {
                            panic!("Expected List")
                        }
                    }),
                });
                self.interfaces.insert(dtype.clone(), interfaces);
            }
            _ => {
                self.interfaces.insert(dtype.clone(), vec![]);
            }
        }
        return self.interfaces.get(&dtype).unwrap();
    }
}
