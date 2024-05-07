use log::error;

pub mod color {
    #[allow(dead_code)]
    pub fn red(text: &str) -> String {
        format!("\x1b[31m{}\x1b[0m", text)
    }
    #[allow(dead_code)]
    pub fn green(text: &str) -> String {
        format!("\x1b[32m{}\x1b[0m", text)
    }
    #[allow(dead_code)]
    pub fn yellow(text: &str) -> String {
        format!("\x1b[33m{}\x1b[0m", text)
    }
    #[allow(dead_code)]
    pub fn blue(text: &str) -> String {
        format!("\x1b[34m{}\x1b[0m", text)
    }
    #[allow(dead_code)]
    pub fn magenta(text: &str) -> String {
        format!("\x1b[35m{}\x1b[0m", text)
    }
    #[allow(dead_code)]
    pub fn cyan(text: &str) -> String {
        format!("\x1b[36m{}\x1b[0m", text)
    }
    #[allow(dead_code)]
    pub fn gray(text: &str) -> String {
        format!("\x1b[37m{}\x1b[0m", text)
    }
    #[allow(dead_code)]
    pub fn black(text: &str) -> String {
        format!("\x1b[30m{}\x1b[0m", text)
    }
}

pub fn tidy_usage(c: &str, d: &str) -> String {
    format!("  {}{}{}", c, " ".repeat(23 - c.len()), d)
}

pub fn print_error(msg: &str, e: anyhow::Error) {
    let mut error_list = vec![];

    error_list.push(msg.to_string());
    error_list.push(e.to_string());

    let mut err = match e.source() {
        Some(e) => e,
        None => {
            for (i, e) in error_list.iter().rev().enumerate() {
                if i == 0 {
                    error!("{}", e);
                } else {
                    error!(" -> {}", e);
                }
            }
            return;
        }
    };

    loop {
        error_list.push(err.to_string());
        err = match err.source() {
            Some(e) => e,
            None => break,
        }
    }

    for (i, e) in error_list.iter().rev().enumerate() {
        if i == 0 {
            error!("{}", e);
        } else {
            error!("{} -> {}", " ".repeat(i), e);
        }
    }
}
