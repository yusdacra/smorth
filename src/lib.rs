#![no_std]

extern crate no_std_compat as std;

use hashbrown::HashMap;
use smartstring::{Compact, SmartString};
use std::{
    fmt::{self, Display, Formatter},
    io::Read,
    prelude::v1::*,
};

type Stack = Vec<i64>;
pub type Word = SmartString<Compact>;

pub const FALSE: i64 = 0;
pub const TRUE: i64 = -1;

#[derive(Debug)]
pub enum ExecutionError {
    Code(i32),
    StackUnderflow,
    NoSuchWord(Word),
    IoError(std::io::Error),
}

impl Display for ExecutionError {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            ExecutionError::Code(code) => write!(f, "exited with code {}", code),
            ExecutionError::StackUnderflow => write!(f, "stack underflow"),
            ExecutionError::NoSuchWord(word) => write!(f, "no such word ({})", word),
            ExecutionError::IoError(err) => write!(f, "io error occured: {}", err),
        }
    }
}

impl From<std::io::Error> for ExecutionError {
    fn from(err: std::io::Error) -> Self {
        ExecutionError::IoError(err)
    }
}

type ExecutionResult<T> = Result<T, ExecutionError>;

#[derive(Debug, Clone, Default)]
pub struct State {
    pub stack: Stack,
    pub dict: HashMap<Word, Vec<Word>>,
    read_buf: String,
    temp_inst_buf: Vec<Word>,
}

pub fn do_word<R: Read>(
    words: &mut Vec<Word>,
    state: &mut State,
    out_buf: &mut dyn std::io::Write,
    in_buf: &mut R,
) -> ExecutionResult<()> {
    let word = match words.pop() {
        Some(w) => w,
        None => return Ok(()),
    };
    match word.as_str() {
        "." => write!(out_buf, "{} ", su(state.stack.pop())?)?,
        "emit" => write!(
            out_buf,
            "{}",
            std::char::from_u32(su(state.stack.pop())? as u32)
                .unwrap_or(std::char::REPLACEMENT_CHARACTER)
        )?,
        "read" => {
            in_buf.read_to_string(&mut state.read_buf)?;
            for c in state.read_buf.chars() {
                state.stack.push(c as i64);
            }
            state.read_buf.clear();
        }
        "cr" => writeln!(out_buf)?,
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
                write!(out_buf, "{} ", word)?;
            } else {
                break;
            }
        },
        "if" => {
            if su(state.stack.pop())? == TRUE {
                state.temp_inst_buf.append(words);
                let has_else;
                loop {
                    let word = su(state.temp_inst_buf.pop())?;
                    if word == "then" {
                        has_else = false;
                        break;
                    } else if word == "else" {
                        has_else = true;
                        break;
                    } else {
                        words.push(word);
                    }
                }
                if has_else {
                    loop {
                        let word = su(state.temp_inst_buf.pop())?;
                        if word == "then" {
                            break;
                        }
                    }
                }
                words.reverse();
                do_word(words, state, out_buf, in_buf)?;
                words.append(&mut state.temp_inst_buf);
            } else {
                state.temp_inst_buf.append(words);
                let has_else;
                loop {
                    let word = su(state.temp_inst_buf.pop())?;
                    if word == "then" {
                        has_else = false;
                        break;
                    } else if word == "else" {
                        has_else = true;
                        break;
                    }
                }
                if has_else {
                    loop {
                        let word = su(state.temp_inst_buf.pop())?;
                        if word == "then" {
                            break;
                        } else {
                            words.push(word);
                        }
                    }
                    words.reverse();
                    do_word(words, state, out_buf, in_buf)?;
                    words.append(&mut state.temp_inst_buf);
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
                do_word(&mut instructions, state, out_buf, in_buf)?;
            }
        },
    }
    do_word(words, state, out_buf, in_buf)
}

fn do_op(stack: &mut Stack, op: fn(i64, i64) -> i64) -> ExecutionResult<()> {
    let v = op(su(stack.pop())?, su(stack.pop())?);
    stack.push(v);
    Ok(())
}

fn su<T>(val: Option<T>) -> ExecutionResult<T> {
    val.ok_or(ExecutionError::StackUnderflow)
}
