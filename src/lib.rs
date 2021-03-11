#![no_std]

extern crate no_std_compat as std;

use hashbrown::HashMap;
use smartstring::{Compact, SmartString};
use std::fmt::{self, Display, Formatter};
use std::prelude::v1::*;

type Stack = Vec<i64>;
pub type Word = SmartString<Compact>;

pub const FALSE: i64 = 0;
pub const TRUE: i64 = -1;

#[derive(Debug, Clone)]
pub enum ExecutionError {
    Code(i32),
    StackUnderflow,
    NoSuchWord(Word),
}

impl Display for ExecutionError {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            ExecutionError::Code(code) => write!(f, "exited with code {}", code),
            ExecutionError::StackUnderflow => write!(f, "stack underflow"),
            ExecutionError::NoSuchWord(word) => write!(f, "no such word ({})", word),
        }
    }
}

type ExecutionResult<T> = Result<T, ExecutionError>;

#[derive(Debug, Clone, Default)]
pub struct State {
    pub stack: Stack,
    pub dict: HashMap<Word, Vec<Word>>,
}

pub fn do_word(
    words: &mut Vec<Word>,
    state: &mut State,
    out_buf: &mut dyn std::io::Write,
) -> ExecutionResult<()> {
    let word = match words.pop() {
        Some(w) => w,
        None => return Ok(()),
    };
    match word.as_str() {
        "." => write!(out_buf, "{} ", su(state.stack.pop())?).unwrap(),
        "emit" => write!(
            out_buf,
            "{}",
            std::char::from_u32(su(state.stack.pop())? as u32)
                .unwrap_or(std::char::REPLACEMENT_CHARACTER)
        )
        .unwrap(),
        "cr" => writeln!(out_buf).unwrap(),
        "+" => do_op(&mut state.stack, |f, s| s + f)?,
        "-" => do_op(&mut state.stack, |f, s| s - f)?,
        "*" => do_op(&mut state.stack, |f, s| s * f)?,
        "/" => do_op(&mut state.stack, |f, s| s / f)?,
        "<" => do_op(&mut state.stack, |f, s| if s < f { TRUE } else { FALSE })?,
        ">" => do_op(&mut state.stack, |f, s| if s > f { TRUE } else { FALSE })?,
        "=" => do_op(&mut state.stack, |f, s| if f == s { TRUE } else { FALSE })?,
        "and" => do_op(&mut state.stack, |f, s| s & f)?,
        "or" => do_op(&mut state.stack, |f, s| s | f)?,
        "not" => {
            let val = !su(state.stack.pop())?;
            state.stack.push(val);
        }
        "dup" => state.stack.push(*su(state.stack.last())?),
        "drop" => {
            su(state.stack.pop())?;
        }
        "swap" => {
            if state.stack.len() > 1 {
                let mut v = state.stack.split_off(state.stack.len() - 2);
                v.reverse();
                state.stack.append(&mut v)
            } else {
                return Err(ExecutionError::StackUnderflow);
            }
        }
        "over" => state
            .stack
            .push(*su(state.stack.get(state.stack.len() - 2))?),
        "rot" => {
            state
                .stack
                .push(*su(state.stack.get(state.stack.len() - 3))?);
            state.stack.remove(state.stack.len() - 4);
        }
        "exit" => return Err(ExecutionError::Code(su(state.stack.pop())? as i32)),
        ":" => {
            let def_word = su(words.pop())?;
            state.dict.insert(def_word.clone(), vec![]);
            loop {
                let word = su(words.pop())?;
                if word == ";" {
                    break;
                } else {
                    state
                        .dict
                        .get_mut(&def_word)
                        .expect("cant happen")
                        .push(word);
                }
            }
        }
        ".\"" => loop {
            let word = su(words.pop())?;
            if word != "\"" {
                write!(out_buf, "{} ", word).unwrap();
            } else {
                break;
            }
        },
        "if" => {
            if su(state.stack.pop())? == TRUE {
                let has_else;
                let mut instructions = Vec::with_capacity(5);
                loop {
                    let word = su(words.pop())?;
                    if word == "then" {
                        has_else = false;
                        break;
                    } else if word == "else" {
                        has_else = true;
                        break;
                    } else {
                        instructions.push(word);
                    }
                }
                if has_else {
                    loop {
                        let word = su(words.pop())?;
                        if word == "then" {
                            break;
                        }
                    }
                }
                instructions.reverse();
                do_word(&mut instructions, state, out_buf)?;
            } else {
                let has_else;
                loop {
                    let word = su(words.pop())?;
                    if word == "then" {
                        has_else = false;
                        break;
                    } else if word == "else" {
                        has_else = true;
                        break;
                    }
                }
                if has_else {
                    let mut instructions = Vec::with_capacity(5);
                    loop {
                        let word = su(words.pop())?;
                        if word == "then" {
                            break;
                        } else {
                            instructions.push(word);
                        }
                    }
                    instructions.reverse();
                    do_word(&mut instructions, state, out_buf)?;
                }
            }
        }
        _ => match word.as_str().parse::<i64>() {
            Ok(num) => state.stack.push(num),
            Err(_) => {
                let mut instructions = state
                    .dict
                    .get(&word)
                    .ok_or(ExecutionError::NoSuchWord(word))?
                    .clone();
                instructions.reverse();
                do_word(&mut instructions, state, out_buf)?;
            }
        },
    }
    do_word(words, state, out_buf)
}

fn do_op(stack: &mut Stack, op: fn(i64, i64) -> i64) -> ExecutionResult<()> {
    let v = op(su(stack.pop())?, su(stack.pop())?);
    stack.push(v);
    Ok(())
}

fn su<T>(val: Option<T>) -> ExecutionResult<T> {
    val.ok_or(ExecutionError::StackUnderflow)
}
