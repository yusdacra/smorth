#![no_std]

extern crate no_std_compat as std;

use hashbrown::HashMap;
use std::{fmt::Write, prelude::v1::*};
use tinystr::TinyStrAuto;

#[derive(Debug, Clone, Copy)]
enum Action {
    Defining(usize),
    Saying,
    None,
}

impl Default for Action {
    fn default() -> Self {
        Action::None
    }
}

type Stack = Vec<i64>;
type Word = TinyStrAuto;

#[derive(Debug, Default)]
pub struct State {
    pub stack: Stack,
    dict: HashMap<Word, Vec<Word>>,
    act: Action,
}

pub struct Data<'a> {
    pub words: &'a [&'a str],
    pub word: &'a str,
    pub index: usize,
}

pub fn do_word<'a>(data: Data<'a>, state: &mut State, out_buf: &mut String) -> Option<i32> {
    let stack = &mut state.stack;
    match state.act {
        Action::Saying => {
            if data.word != "\"" {
                out_buf.push_str(data.word);
                out_buf.push(' ');
            } else {
                state.act = Action::None;
            }
        }
        Action::Defining(def_index) => {
            let def_word: Word = data
                .words
                .get(def_index + 1)
                .expect("no def word name")
                .parse()
                .expect("should not happen");
            if data.index != def_index + 1 {
                if data.word == ";" {
                    state.act = Action::None;
                } else {
                    state
                        .dict
                        .get_mut(&def_word)
                        .expect("cant happen")
                        .push(data.word.parse().expect("should not happen"));
                }
            } else {
                state.dict.insert(def_word, vec![]);
            }
        }
        Action::None => match data.word {
            "." => write!(out_buf, "{} ", su(stack.pop()))
                .unwrap_or_else(|_| out_buf.push(std::char::REPLACEMENT_CHARACTER)),
            "emit" => out_buf.push(
                std::char::from_u32(su(stack.pop()) as u32)
                    .unwrap_or(std::char::REPLACEMENT_CHARACTER),
            ),
            "cr" => out_buf.push('\n'),
            "+" => do_op(stack, |f, s| s + f),
            "-" => do_op(stack, |f, s| s - f),
            "*" => do_op(stack, |f, s| s * f),
            "/" => do_op(stack, |f, s| s / f),
            "<" => do_op(stack, |f, s| if s < f { -1 } else { 0 }),
            ">" => do_op(stack, |f, s| if s > f { -1 } else { 0 }),
            "=" => do_op(stack, |f, s| if f == s { -1 } else { 0 }),
            "and" => do_op(stack, |f, s| s & f),
            "or" => do_op(stack, |f, s| s | f),
            "invert" => {
                let val = !su(stack.pop());
                stack.push(val);
            }
            "dup" => stack.push(*su(stack.last())),
            "drop" => {
                su(stack.pop());
            }
            "swap" => {
                if stack.len() > 1 {
                    let mut v = stack.split_off(stack.len() - 2);
                    v.reverse();
                    stack.append(&mut v)
                } else {
                    panic!("stack underflow")
                }
            }
            "over" => stack.push(*su(stack.get(stack.len() - 2))),
            "rot" => {
                stack.push(*su(stack.get(stack.len() - 3)));
                stack.remove(stack.len() - 4);
            }
            "exit" => return Some(su(stack.pop()) as i32),
            ":" => state.act = Action::Defining(data.index),
            ".\"" => state.act = Action::Saying,
            _ => match data.word.parse::<i64>() {
                Ok(num) => stack.push(num),
                Err(_) => {
                    for w in state
                        .dict
                        .iter()
                        .find_map(|(k, v)| {
                            if data.word.as_bytes() == k.as_bytes() {
                                Some(v)
                            } else {
                                None
                            }
                        })
                        .expect("no such word")
                        .clone()
                    {
                        let maybe_exit_code = do_word(Data { word: &w, ..data }, state, out_buf);
                        if maybe_exit_code.is_some() {
                            return maybe_exit_code;
                        }
                    }
                }
            },
        },
    }
    None
}

fn do_op(stack: &mut Stack, op: fn(i64, i64) -> i64) {
    let v = op(su(stack.pop()), su(stack.pop()));
    stack.push(v)
}

fn su<T>(val: Option<T>) -> T {
    val.expect("stack underflow")
}
