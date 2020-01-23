use getopts::{Matches, Options};
use std::env;

fn main() {
    let args: Vec<String> = env::args().collect();
    let program = args[0].clone();

    let mut opts = Options::new();
    opts.reqopt("t", "", "set app token", "APP_TOKEN");
    opts.optflag("h", "help", "print this help");
    match opts.parse(&args[1..]) {
        Ok(m) if m.opt_present("h") => print_usage(&program, opts),
        // TOOD: print failure reason
        Err(f) => print_usage(&program, opts),
        Ok(m) => start(m),
    }
}

fn start(matches: Matches) {
    if let Some(token) = matches.opt_str("t") {
        println!("starting schedules runner with token: {:?}", token);
    }
}

fn print_usage(program: &str, opts: Options) {
    let brief = format!("Usage: {} [options] -t APP_TOKEN", program);
    print!("{}", opts.usage(&brief));
}
