use clap::{Parser, ValueEnum};
use object::{Object, ObjectSymbol, SymbolKind};
use rustc_demangle::demangle;

#[derive(Debug, Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    /// Input file
    #[arg(long)]
    input: String,

    /// Symbol sort mode
    #[arg(long, value_enum, default_value_t = Sort::Name)]
    sort: Sort,

    /// Function only mode
    #[arg(long)]
    function_only: bool,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
enum Sort {
    /// Symbol name
    Name,
    /// Symbol size
    Size,
}

fn main() {
    let cli = Cli::parse();

    let data = std::fs::read(&cli.input).expect("reading input file");
    let object_file = object::File::parse(&*data).expect("parsing file as elf binary");

    let mut symbols = vec![];
    for symbol in object_file.symbols() {
        if symbol.size() > 0 {
            if cli.function_only {
                if symbol.kind() != SymbolKind::Text {
                    continue;
                }
            }

            symbols.push((demangle(symbol.name().expect("extracting symbol name")).to_string(), symbol.size()));
        }
    }

    match cli.sort {
        Sort::Name => symbols.sort_by(|(a, _), (b, _)| a.cmp(b)),
        Sort::Size => symbols.sort_by(|(_, a), (_, b)| b.cmp(a)),
    }

    for (symbol, size) in symbols {
        println!("Symbol: {}, size: {}", symbol, size);
    }
}
