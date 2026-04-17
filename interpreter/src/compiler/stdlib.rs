use std::{
    cell::RefCell,
    collections::HashMap,
    fmt::{self, Debug},
    rc::Rc,
};

use crate::{
    ast::token::{datatype::DataType, literal::Literal},
    error::{AlthreadError, ErrorType, Pos},
};

#[derive(Clone)]
pub struct Interface {
    pub name: String,
    pub args: Vec<DataType>,
    pub ret: DataType,
    pub mutates_receiver: bool,
    //pub f: Mutex<Box<dyn Fn(&mut Literal, &mut Literal) -> Literal + Send + Sync>>,
    pub f: Rc<dyn Fn(&mut Literal, &mut Literal, Option<Pos>) -> Result<Literal, AlthreadError>>,
}

#[derive(Debug)]
pub struct Stdlib {
    pub interfaces: RefCell<HashMap<DataType, Vec<Interface>>>,
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
            interfaces: RefCell::new(HashMap::new()),
        }
    }

    pub fn get_interfaces(&self, dtype: &DataType) -> Option<Vec<Interface>> {
        self.interfaces.borrow().get(dtype).cloned()
    }

    pub fn is_interface(&self, dtype: &DataType, name: &str) -> bool {
        if let Some(interfaces) = self.get_interfaces(dtype) {
            interfaces.iter().any(|i| i.name == name)
        } else {
            false
        }
    }

    pub fn interfaces(&self, dtype: &DataType) -> Vec<Interface> {
        if let Some(interfaces) = self.get_interfaces(dtype) {
            return interfaces;
        }

        let mut new_interfaces = vec![];

        match dtype.clone() {
            DataType::List(t) => {
                new_interfaces.push(Interface {
                    name: "len".to_string(),
                    args: vec![],
                    ret: DataType::Integer,
                    mutates_receiver: false,
                    f: Rc::new(|list, _v, pos| {
                        // let args = v.to_tuple().unwrap();
                        // TODO!: for in control uses len with args???
                        // if !args.is_empty() {
                        //     return Err(AlthreadError::new(
                        //         ErrorType::RuntimeError,
                        //         pos,
                        //         ".len() expects no arguments: l.len();".to_string()
                        //     ));
                        // }
                        match list {
                            Literal::List(_, v) => Ok(Literal::Int(v.len() as i64)),
                            _ => Err(AlthreadError::new(
                                ErrorType::RuntimeError,
                                pos,
                                "Expected List".to_string(),
                            )),
                        }
                    }),
                });
                new_interfaces.push(Interface {
                    name: "push".to_string(),
                    args: vec![t.as_ref().clone()],
                    ret: DataType::Void,
                    mutates_receiver: true,
                    f: Rc::new(|list, v, pos| {
                        let v = v.to_tuple().unwrap();
                        if let Literal::List(dtype, list) = list {
                            if v.len() != 1 {
                                return Err(AlthreadError::new(
                                    ErrorType::RuntimeError,
                                    pos,
                                    ".push() expects exactly one argument: l.push(value);".to_string()
                                ));
                            }
                            if dtype != &v[0].get_datatype() {
                                return Err(AlthreadError::new(
                                    ErrorType::RuntimeError,
                                    pos,
                                    format!("List of type {} can only accept values of the same type ({} given)", dtype, v[0].get_datatype())
                                ));
                            }
                            list.push(v[0].clone());
                        }
                        else {
                            Err(AlthreadError::new(
                                ErrorType::RuntimeError,
                                pos,
                                "Expected List".to_string()
                            ))?;
                        }
                        Ok(Literal::Null)
                    }),
                });
                new_interfaces.push(Interface {
                    name: "remove".to_string(),
                    args: vec![DataType::Integer],
                    ret: t.as_ref().clone(),
                    mutates_receiver: true,
                    f: Rc::new(|list, v, pos| {
                        let args = v.to_tuple().unwrap();
                        if args.len() != 1 {
                            return Err(AlthreadError::new(
                                ErrorType::RuntimeError,
                                pos,
                                ".remove() expects one argument: l.remove(index);".to_string(),
                            ));
                        }
                        let idx = args[0].to_integer().unwrap();
                        if let Literal::List(_dtype, list) = list {
                            if idx < 0 || idx as usize >= list.len() {
                                return Err(AlthreadError::new(
                                    ErrorType::RuntimeError,
                                    pos,
                                    format!("Index out of bounds: {}", idx),
                                ));
                            }
                            Ok(list.remove(idx as usize))
                        } else {
                            Err(AlthreadError::new(
                                ErrorType::RuntimeError,
                                pos,
                                "Expected List".to_string(),
                            ))
                        }
                    }),
                });
                new_interfaces.push(Interface {
                    name: "set".to_string(),
                    args: vec![DataType::Integer, t.as_ref().clone()],
                    ret: DataType::Void,
                    mutates_receiver: true,
                    f: Rc::new(|list, v, pos| {
                        let v = v.to_tuple().unwrap();
                        if v.len() != 2 {
                            return Err(AlthreadError::new(
                                ErrorType::RuntimeError,
                                pos,
                                ".set() expects two arguments: l.set(index, value);".to_string()
                            ));
                        }
                        let idx = v[0].to_integer().unwrap() as usize;
                        if let Literal::List(dtype, list) = list {
                            if idx >= list.len() {
                                return Err(AlthreadError::new(
                                    ErrorType::RuntimeError,
                                    pos,
                                    format!("Index out of bounds: {}", idx)
                                ));
                            }
                            if dtype != &v[1].get_datatype() {
                                return Err(AlthreadError::new(
                                    ErrorType::RuntimeError,
                                    pos,
                                    format!("List of type {:?} can only accept values of the same type ({} given)", dtype, v[1].get_datatype())
                                ));
                            }
                            list[idx] = v[1].clone();
                        }
                        else {
                            return Err(AlthreadError::new(
                                ErrorType::RuntimeError,
                                pos,
                                "Expected List".to_string()
                            ));
                        }
                        Ok(Literal::Null)
                    }),
                });
                new_interfaces.push(Interface {
                    name: "at".to_string(),
                    args: vec![DataType::Integer],
                    ret: t.as_ref().clone(),
                    mutates_receiver: false,
                    f: Rc::new(|list, v, pos| {
                        let v = v.to_tuple().unwrap();
                        let v = v.first().unwrap().to_integer().unwrap();
                        if let Literal::List(_dtype, list) = list {
                            if v < 0 || v as usize >= list.len() {
                                return Err(AlthreadError::new(
                                    ErrorType::RuntimeError,
                                    pos,
                                    format!("Index out of bounds: {}", v),
                                ));
                            }
                            Ok(list[v as usize].clone())
                        } else {
                            return Err(AlthreadError::new(
                                ErrorType::RuntimeError,
                                pos,
                                "Expected List".to_string(),
                            ));
                        }
                    }),
                });
            }
            DataType::Tuple(t) => {
                new_interfaces.push(Interface {
                    name: "len".to_string(),
                    args: vec![],
                    ret: DataType::Integer,
                    mutates_receiver: false,
                    f: Rc::new(|list, _v, pos| {
                        match list {
                            Literal::Tuple(v) => Ok(Literal::Int(v.len() as i64)),
                            _ => Err(AlthreadError::new(
                                ErrorType::RuntimeError,
                                pos,
                                "Expected tuple".to_string(),
                            )),
                        }
                    }),
                });
                for s in 0..t.len(){
                    let size = s.clone();
                    let func_name : String =String::from("set") + &size.to_string();
                    new_interfaces.push(Interface {
                        name: func_name.clone(),
                        args: vec![t[size].clone()],
                        ret: DataType::Void,
                        mutates_receiver: true,
                        f: Rc::new(move |tuple, v, pos| {
                            let v = v.to_tuple().unwrap();
                            if v.len() != 1 {
                                let message : String = func_name.clone() + &String::from("expects one arguments: l.")  + &func_name.clone() + &String::from("(value);");
                                return Err(AlthreadError::new(
                                    ErrorType::RuntimeError,
                                    pos,
                                    message
                                ));
                            }
                            if let Literal::Tuple(tuple) = tuple {
                                if tuple.len() <= 0 {
                                    return Err(AlthreadError::new(
                                        ErrorType::RuntimeError,
                                        pos,
                                        format!("Set on an empty tuple")
                                    ));
                                }
                                let value = v[0].clone();
                                let dtype_of_value = value.clone().get_datatype();
                                let dtype_of_element = tuple[size].get_datatype();
                                if tuple[size].get_datatype() != dtype_of_value {
                                    return Err(AlthreadError::new(
                                        ErrorType::RuntimeError,
                                        pos,
                                        format!("Element of tuple {:?} can only accept values of the same type ({} given, expect {})", tuple[size], dtype_of_value,dtype_of_element)
                                    ));
                                }
                                tuple[size] = value;
                            }
                            else {
                                return Err(AlthreadError::new(
                                    ErrorType::RuntimeError,
                                    pos,
                                    "Expected Tuple".to_string()
                                ));
                            }
                            Ok(Literal::Null)
                        }),
                    });
                }
                for s in 0..t.len()
                {
                    let size = s.clone();
                    new_interfaces.push(Interface {
                        name: size.to_string(),
                        args: vec![],
                        ret: t[size].clone(),
                        mutates_receiver: false,
                        f: Rc::new(move |tuple, _v, pos| {
                            if let Literal::Tuple(vec ) = tuple {
                                if vec.len() <=0  {
                                    return Err(AlthreadError::new(
                                        ErrorType::RuntimeError,
                                        pos,
                                        format!(" First call on a empty Tuple"),
                                    ));
                                }
                                Ok(vec[size].clone())
                            } else {
                                return Err(AlthreadError::new(
                                    ErrorType::RuntimeError,
                                    pos,
                                    "Expected tuple".to_string(),
                                ));
                            }
                        }),
                    });
                } 
            }
            _ => {}
        }

        self.interfaces
            .borrow_mut()
            .insert(dtype.clone(), new_interfaces.clone());
        new_interfaces
    }
}

pub fn invoke_interface_method(
    stdlib: &Stdlib,
    name: &str,
    receiver: &mut Literal,
    args: &mut Literal,
    pos: Option<Pos>,
) -> Result<(Literal, bool), AlthreadError> {
    let datatype = receiver.get_datatype();
    let interfaces = stdlib.interfaces(&datatype);

    if interfaces.is_empty() {
        return Err(AlthreadError::new(
            ErrorType::UndefinedFunction,
            pos.clone(),
            format!("Type {:?} has no interface available", datatype),
        ));
    }

    let interface = interfaces
        .iter()
        .find(|interface| interface.name == name)
        .ok_or_else(|| {
            AlthreadError::new(
                ErrorType::UndefinedFunction,
                pos.clone(),
                format!("undefined function {}", name),
            )
        })?;

    let ret = interface.f.as_ref()(receiver, args, pos)?;

    Ok((ret, interface.mutates_receiver))
}
