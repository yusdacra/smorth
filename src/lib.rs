#![no_std]

extern crate alloc;

use alloc::vec::Vec;
use core::fmt::{self, Display, Formatter};
use core2::io::{Error as IoError, Read, Write};
use hashbrown::HashMap;
use smartstring::{Compact, SmartString};
use tinyvec::{tiny_vec, TinyVec};

const STACK_SIZE: usize = 256;
const READ_BUF_SIZE: usize = 256;

type ReadBuf = TinyVec<[u8; READ_BUF_SIZE]>;
type Words = Vec<Word>;
type Stack = TinyVec<[i64; STACK_SIZE]>;
pub type Word = SmartString<Compact>;

pub const FALSE: i64 = 0;
pub const TRUE: i64 = -1;

#[derive(Debug)]
pub enum ExecutionError {
    Code(i32),
    StackUnderflow,
    NoSuchWord(Word),
    IoError(IoError),
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

impl From<IoError> for ExecutionError {
    fn from(err: IoError) -> Self {
        ExecutionError::IoError(err)
    }
}

type ExecutionResult<T> = Result<T, ExecutionError>;

#[derive(Debug, Clone)]
pub struct State {
    pub stack: Stack,
    pub dict: HashMap<Word, Words>,
    read_buf: ReadBuf,
    temp_inst_buf: Words,
}

impl Default for State {
    fn default() -> Self {
        Self {
            stack: tiny_vec!([i64; STACK_SIZE]),
            dict: HashMap::with_capacity(16),
            read_buf: tiny_vec!([u8; READ_BUF_SIZE]),
            temp_inst_buf: Vec::with_capacity(256),
        }
    }
}

impl State {
    pub fn do_word<R: Read, W: Write>(
        &mut self,
        words: &mut Words,
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
                core::char::from_u32(su(self.stack.pop())? as u32)
                    .unwrap_or(core::char::REPLACEMENT_CHARACTER)
            )?,
            "key" => {
                let mut ch = [0; 1];
                if let Err(err) = in_buf.read_exact(&mut ch) {
                    match err.kind() {
                        core2::io::ErrorKind::UnexpectedEof => {}
                        _ => return Err(err.into()),
                    }
                }
                self.stack.push(ch[0] as i64);
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
                self.dict.insert(def_word.clone(), Vec::with_capacity(256));
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

pub fn tokenize(code: &str) -> Words {
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
        .collect()
}
