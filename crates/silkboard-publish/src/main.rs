//! Thin executable adapter; credentials and content never enter diagnostics.

fn main() {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    match silkboard_publish::run(&args, std::io::stdin().lock()) {
        Ok(message) => println!("{message}"),
        Err(error) => {
            eprintln!("{error}");
            std::process::exit(1);
        }
    }
}
