use smorth::ExecutionError;

const HIST_FILE: &str = "/tmp/.smorth_history";

fn main() {
    let mut state = smorth::State::default();
    match std::env::args().nth(1) {
        Some(path) => {
            let code = std::fs::read_to_string(path).expect("couldnt read file");
            let mut words = tokenize(&code);
            match smorth::do_word(&mut words, &mut state, &mut std::io::stdout()) {
                Err(ExecutionError::Code(code)) => std::process::exit(code),
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
                match smorth::do_word(&mut words, &mut state, &mut std::io::stdout()) {
                    Err(ExecutionError::Code(code)) => {
                        rl.save_history(HIST_FILE).unwrap();
                        std::process::exit(code);
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

fn tokenize(code: &str) -> Vec<smorth::Word> {
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
        .collect::<Vec<_>>()
}
