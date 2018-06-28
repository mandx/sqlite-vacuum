use atty;
use console::Term;

pub struct Display {
    istty: bool,
    term: Term,
}

impl Display {
    pub fn new() -> Self {
        Self {
            istty: atty::is(atty::Stream::Stdout) && atty::is(atty::Stream::Stderr),
            term: Term::stdout(),
        }
    }

    pub fn progress(&self, msg: &str) {
        if self.istty {
            self.term.clear_line().unwrap();
            self.term.write_str(msg).unwrap();
        } else {
            println!("{}", msg);
        }
    }

    pub fn error(&self, msg: &str) {
        if self.istty {
            self.term.write_line(msg).unwrap();
        } else {
            eprintln!("{}", msg);
        }
    }

    pub fn write_line(&self, msg: &str) {
        if self.istty {
            self.term.clear_line().unwrap();
            self.term.write_line(msg).unwrap();
        } else {
            println!("{}", msg);
        }
    }
}
