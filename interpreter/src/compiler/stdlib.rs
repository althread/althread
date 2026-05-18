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
            _ => {}
        }

        self.interfaces
            .borrow_mut()
            .insert(dtype.clone(), new_interfaces.clone());
        new_interfaces
    }
}

pub fn resolve_interface_method(
    stdlib: &Stdlib,
    receiver_type: &DataType,
    name: &str,
) -> Result<Interface, String> {
    let interfaces = stdlib.interfaces(receiver_type);
    if interfaces.is_empty() {
        return Err(format!("Type {} has no available methods", receiver_type));
    }

    interfaces
        .into_iter()
        .find(|interface| interface.name == name)
        .ok_or_else(|| {
            format!(
                "No method {} found on variable of type {}",
                name, receiver_type
            )
        })
}

pub fn validate_interface_call(
    interface: &Interface,
    provided_arg_types: &[DataType],
) -> Result<(), String> {
    if interface.args.len() != provided_arg_types.len() {
        return Err(format!(
            "Method '{}' expects {} arguments, got {}",
            interface.name,
            interface.args.len(),
            provided_arg_types.len()
        ));
    }

    for (expected, provided) in interface.args.iter().zip(provided_arg_types.iter()) {
        if expected != provided {
            return Err(format!(
                "Method '{}' expects argument of type {}, got {}",
                interface.name, expected, provided
            ));
        }
    }

    Ok(())
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
