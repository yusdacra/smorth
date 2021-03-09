const HIST_FILE: &str = "/tmp/.smorth_history";

fn main() {
    let mut state = Default::default();
    let mut output = String::default();
    match std::env::args().nth(1) {
        Some(path) => {
            let code = std::fs::read_to_string(path).expect("couldnt read file");
            if let Some(code) = run_words(&mut output, tokenize(&code).as_slice(), &mut state) {
                std::process::exit(code);
            }
        }
        None => {
            let mut rl = rustyline::Editor::<()>::new();
            if rl.load_history(HIST_FILE).is_err() {
                println!("No previous history.");
            }
            while let Ok(line) = rl.readline(&construct_prefix(state.stack.as_slice())) {
                rl.add_history_entry(line.as_str());
                if let Some(code) = run_words(&mut output, tokenize(&line).as_slice(), &mut state) {
                    rl.save_history(HIST_FILE).unwrap();
                    std::process::exit(code);
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

fn run_words(output: &mut String, words: &[&str], state: &mut smorth::State) -> Option<i32> {
    for (index, word) in words.iter().enumerate() {
        let data = smorth::Data { words, word, index };
        let exit_code = smorth::do_word(data, state, output);
        if exit_code.is_some() {
            return exit_code;
        }
        if !output.is_empty() {
            print!("{}", output);
        }
        output.clear();
    }
    None
}

fn tokenize(code: &str) -> Vec<&str> {
    code.split(|c| c == ' ' || c == '\n')
        .filter_map(|s| {
            let new = s.trim();
            if new.is_empty() {
                None
            } else {
                Some(new)
            }
        })
        .collect::<Vec<_>>()
}
