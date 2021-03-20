use smorth::{tokenize, ExecutionError, State};

use std::{
    env, fs,
    io::{stdin, stdout},
    process::exit,
};

const HIST_FILE: &str = "/tmp/.smorth_history";

fn main() {
    let mut state = State::default();
    match env::args().nth(1) {
        Some(path) => {
            let code = fs::read_to_string(path).expect("couldnt read file");
            let mut words = tokenize(&code);
            match state.do_word(&mut words, &mut stdout(), &mut stdin()) {
                Err(ExecutionError::Code(code)) => exit(code),
                Err(err) => eprintln!("{}", err),
                _ => {}
            }
        }
        None => {
            let mut rl = rustyline::Editor::<()>::new();
            if rl.load_history(HIST_FILE).is_err() {
                println!("No previous history.");
            }
            while let Ok(line) = rl.readline(&construct_prefix(state.stack.as_slice())) {
                rl.add_history_entry(line.as_str());
                let mut words = tokenize(&line);
                match state.do_word(&mut words, &mut stdout(), &mut stdin()) {
                    Err(ExecutionError::Code(code)) => {
                        rl.save_history(HIST_FILE).unwrap();
                        exit(code);
                    }
                    Err(err) => eprintln!("{}", err),
                    _ => {}
                }
            }
            rl.save_history(HIST_FILE).unwrap();
        }
    }
}

fn construct_prefix(stack: &[i64]) -> String {
    let mut prefix = String::with_capacity(stack.len() * 5 + 2);
    for item in stack {
        use std::fmt::Write;
        write!(&mut prefix, "{} ", item).unwrap();
    }
    prefix.push_str("#> ");
    prefix
}
