extern crate nom_sql;
extern crate encoding;

use std::io::{self, BufRead};
use nom_sql::{
    parser::parse_query,
    SqlQuery::Insert,
    InsertStatement,
    Table,
    Literal
};

use encoding::{Encoding, DecoderTrap};
use encoding::all::UTF_8;

fn extract_data(input: &str) -> Option<Vec<Vec<Literal>>> {
    match parse_query(input) {
        Ok(Insert(InsertStatement {
                table: Table {name, ..},
                data, ..
        })) => if name == "externallinks" {Some(data)} else {None},
        parsed => {
            eprintln!("Not a valid import statement: {} ({:?})", input, parsed);
            None
        }
    }
}

fn process_line(input: &str) -> Vec<String> {
    extract_data(input).iter()
        .flat_map(|data| data.iter())
        .flat_map(|v| match v.get(2) {
            Some(Literal::String(s)) => Some(s.clone()),
            _ => None
        })
        .collect()
}

fn main() {
    let stdin = io::stdin();
    for line_result in stdin.lock().split(b'\n') {
        match line_result {
            Ok(ref line_bytes) => {
                if let Ok(line_str) = UTF_8.decode(line_bytes, DecoderTrap::Replace) {
                    for url in process_line(&line_str) {
                        println!("{}", url);
                    }
                } else {
                    eprintln!("Unable to decode the line (should never happen).");
                }
            },
            Err(err) => {
                eprintln!("Unable to read line: {}", err);
            }
        }
    }
}
