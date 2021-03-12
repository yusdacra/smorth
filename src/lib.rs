#![no_std]

extern crate no_std_compat as std;

use hashbrown::HashMap;
use smartstring::{Compact, SmartString};
use std::{
    fmt::{self, Display, Formatter},
    io::{Read, Write},
    prelude::v1::*,
};
use tinyvec::{tiny_vec, TinyVec};

const STACK_SIZE: usize = 256;

type Words<const N: usize> = TinyVec<[Word; N]>;
type Stack = TinyVec<[i64; STACK_SIZE]>;
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

#[derive(Debug, Clone)]
pub struct State<const N: usize> {
    pub stack: Stack,
    pub dict: HashMap<Word, Words<N>>,
    read_buf: String,
    temp_inst_buf: Words<N>,
}

impl<const N: usize> Default for State<N> {
    fn default() -> Self {
        Self {
            stack: tiny_vec!([i64; STACK_SIZE]),
            dict: HashMap::with_capacity(16),
            read_buf: String::with_capacity(128),
            temp_inst_buf: tiny_vec!([Word; N]),
        }
    }
}

impl<const N: usize> State<N> {
    pub fn do_word<R: Read, W: Write>(
        &mut self,
        words: &mut Words<N>,
        out_buf: &mut W,
        in_buf: &mut R,
    ) -> ExecutionResult<()> {
        let word = match words.pop() {
            Some(w) => w,
            None => return Ok(()),
        };
        match word.as_str() {
            "." => write!(out_buf, "{} ", su(self.stack.pop())?)?,
            "emit" => write!(
                out_buf,
                "{}",
                std::char::from_u32(su(self.stack.pop())? as u32)
                    .unwrap_or(std::char::REPLACEMENT_CHARACTER)
            )?,
            "read" => {
                in_buf.read_to_string(&mut self.read_buf)?;
                for c in self.read_buf.chars() {
                    self.stack.push(c as i64);
                }
                self.read_buf.clear();
            }
            "cr" => writeln!(out_buf)?,
            "+" => do_op(&mut self.stack, |f, s| s + f)?,
            "-" => do_op(&mut self.stack, |f, s| s - f)?,
            "*" => do_op(&mut self.stack, |f, s| s * f)?,
            "/" => do_op(&mut self.stack, |f, s| s / f)?,
            "<" => do_op(&mut self.stack, |f, s| if s < f { TRUE } else { FALSE })?,
            ">" => do_op(&mut self.stack, |f, s| if s > f { TRUE } else { FALSE })?,
            "=" => do_op(&mut self.stack, |f, s| if f == s { TRUE } else { FALSE })?,
            "and" => do_op(&mut self.stack, |f, s| s & f)?,
            "or" => do_op(&mut self.stack, |f, s| s | f)?,
            "not" => {
                let val = !su(self.stack.pop())?;
                self.stack.push(val);
            }
            "dup" => self.stack.push(*su(self.stack.last())?),
            "drop" => {
                su(self.stack.pop())?;
            }
            "swap" => {
                if self.stack.len() > 1 {
                    let mut v = self.stack.split_off(self.stack.len() - 2);
                    v.reverse();
                    self.stack.append(&mut v)
                } else {
                    return Err(ExecutionError::StackUnderflow);
                }
            }
            "over" => self.stack.push(*su(self.stack.get(self.stack.len() - 2))?),
            "rot" => {
                self.stack.push(*su(self.stack.get(self.stack.len() - 3))?);
                self.stack.remove(self.stack.len() - 4);
            }
            "exit" => return Err(ExecutionError::Code(su(self.stack.pop())? as i32)),
            ":" => {
                let def_word = su(words.pop())?;
                self.dict
                    .insert(def_word.clone(), tinyvec::tiny_vec!([Word; N]));
                loop {
                    let word = su(words.pop())?;
                    if word == ";" {
                        break;
                    } else {
                        self.dict
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
                let has_else;
                self.temp_inst_buf.append(words);
                if su(self.stack.pop())? == TRUE {
                    loop {
                        let word = su(self.temp_inst_buf.pop())?;
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
                            let word = su(self.temp_inst_buf.pop())?;
                            if word == "then" {
                                break;
                            }
                        }
                    }
                    words.reverse();
                    self.do_word(words, out_buf, in_buf)?;
                    words.append(&mut self.temp_inst_buf);
                } else {
                    loop {
                        let word = su(self.temp_inst_buf.pop())?;
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
                            let word = su(self.temp_inst_buf.pop())?;
                            if word == "then" {
                                break;
                            } else {
                                words.push(word);
                            }
                        }
                        words.reverse();
                        self.do_word(words, out_buf, in_buf)?;
                        words.append(&mut self.temp_inst_buf);
                    }
                }
            }
            _ => match word.as_str().parse::<i64>() {
                Ok(num) => self.stack.push(num),
                Err(_) => {
                    let mut instructions = self
                        .dict
                        .get(&word)
                        .ok_or(ExecutionError::NoSuchWord(word))?
                        .clone();
                    instructions.reverse();
                    self.do_word(&mut instructions, out_buf, in_buf)?;
                }
            },
        }
        self.do_word(words, out_buf, in_buf)
    }
}

fn do_op(stack: &mut Stack, op: fn(i64, i64) -> i64) -> ExecutionResult<()> {
    let v = op(su(stack.pop())?, su(stack.pop())?);
    stack.push(v);
    Ok(())
}

fn su<T>(val: Option<T>) -> ExecutionResult<T> {
    val.ok_or(ExecutionError::StackUnderflow)
}

pub fn tokenize<const N: usize>(code: &str) -> Words<N> {
    code.split(|c| c == ' ' || c == '\n')
        .filter_map(|s| {
            let new = s.trim();
            if new.is_empty() {
                None
            } else {
                Some(new.into())
            }
        })
        .rev()
        .collect::<TinyVec<_>>()
}
