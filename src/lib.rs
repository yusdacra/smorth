#![no_std]

extern crate no_std_compat as std;

use hashbrown::HashMap;
use smartstring::{Compact, SmartString};
use std::prelude::v1::*;

type Stack = Vec<i64>;
pub type Word = SmartString<Compact>;

#[derive(Debug, Clone, Copy)]
pub enum ProcessResult {
    Ok,
    Code(i32),
}

#[derive(Debug, Clone, Default)]
pub struct State {
    pub stack: Stack,
    pub dict: HashMap<Word, Vec<Word>>,
}

pub fn do_word(
    words: &mut Vec<Word>,
    state: &mut State,
    out_buf: &mut dyn std::io::Write,
) -> ProcessResult {
    let word = match words.pop() {
        Some(w) => w,
        None => return ProcessResult::Ok,
    };
    match word.as_str() {
        "." => write!(out_buf, "{} ", su(state.stack.pop())).unwrap(),
        "emit" => write!(
            out_buf,
            "{}",
            std::char::from_u32(su(state.stack.pop()) as u32)
                .unwrap_or(std::char::REPLACEMENT_CHARACTER)
        )
        .unwrap(),
        "cr" => writeln!(out_buf).unwrap(),
        "+" => do_op(&mut state.stack, |f, s| s + f),
        "-" => do_op(&mut state.stack, |f, s| s - f),
        "*" => do_op(&mut state.stack, |f, s| s * f),
        "/" => do_op(&mut state.stack, |f, s| s / f),
        "<" => do_op(&mut state.stack, |f, s| if s < f { -1 } else { 0 }),
        ">" => do_op(&mut state.stack, |f, s| if s > f { -1 } else { 0 }),
        "=" => do_op(&mut state.stack, |f, s| if f == s { -1 } else { 0 }),
        "and" => do_op(&mut state.stack, |f, s| s & f),
        "or" => do_op(&mut state.stack, |f, s| s | f),
        "invert" => {
            let val = !su(state.stack.pop());
            state.stack.push(val);
        }
        "dup" => state.stack.push(*su(state.stack.last())),
        "drop" => {
            su(state.stack.pop());
        }
        "swap" => {
            if state.stack.len() > 1 {
                let mut v = state.stack.split_off(state.stack.len() - 2);
                v.reverse();
                state.stack.append(&mut v)
            } else {
                panic!("stack underflow")
            }
        }
        "over" => state
            .stack
            .push(*su(state.stack.get(state.stack.len() - 2))),
        "rot" => {
            state
                .stack
                .push(*su(state.stack.get(state.stack.len() - 3)));
            state.stack.remove(state.stack.len() - 4);
        }
        "exit" => return ProcessResult::Code(su(state.stack.pop()) as i32),
        ":" => {
            let def_word = su(words.pop());
            state.dict.insert(def_word.clone(), vec![]);
            define_word(words, state, def_word.as_str());
        }
        ".\"" => say_smth(words, state, out_buf),
        _ => match word.as_str().parse::<i64>() {
            Ok(num) => state.stack.push(num),
            Err(_) => {
                let mut instructions = state.dict.get(&word).expect("no such word").clone();
                instructions.reverse();
                let exit_code = do_word(&mut instructions, state, out_buf);
                if matches!(exit_code, ProcessResult::Code(_)) {
                    return exit_code;
                }
            }
        },
    }
    do_word(words, state, out_buf)
}

fn say_smth(words: &mut Vec<Word>, state: &mut State, out_buf: &mut dyn std::io::Write) {
    let word = su(words.pop());
    if word != "\"" {
        write!(out_buf, "{} ", word).unwrap();
        say_smth(words, state, out_buf);
    }
}

fn define_word(words: &mut Vec<Word>, state: &mut State, def_word: &str) {
    let word = su(words.pop());
    if word == ";" {
        return;
    } else {
        state
            .dict
            .get_mut(def_word)
            .expect("cant happen")
            .push(word);
    }
    define_word(words, state, def_word);
}

fn do_op(stack: &mut Stack, op: fn(i64, i64) -> i64) {
    let v = op(su(stack.pop()), su(stack.pop()));
    stack.push(v)
}

fn su<T>(val: Option<T>) -> T {
    val.expect("stack underflow")
}
