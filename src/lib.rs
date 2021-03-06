extern crate nom_sql;

use nom_sql::{
    InsertStatement,
    Literal,
    parser,
    parser::SqlQuery,
    SqlQuery::Insert,
    SqlQuery::CreateTable,
    Table,
};
use std::io::BufRead;
use std::io::Error;
use std::iter::empty;
use ::ExtractedSql::InsertData;
use nom_sql::CreateTableStatement;
use nom_sql::ColumnSpecification;
use nom_sql::Column;

type TableName = String;
type TableColumns = Vec<String>;
type InsertData = Vec<Vec<Literal>>;

struct TableSchema {
    name: TableName,
    columns: TableColumns
}

enum ExtractedSql {
    InsertData(Vec<Vec<Literal>>),
    CreateTableData(TableSchema),
    Error(String),
}

fn extract_data(query: SqlQuery) -> ExtractedSql {
    match query {
        Insert(InsertStatement {
                   table: Table { name, .. },
                   data, ..
               }) => {
            if name == TARGET_COLUMN.table {
                InsertData(data)
            } else {
                ExtractedSql::Error(format!("Wrong table: '{}'", name))
            }
        },
        CreateTable(CreateTableStatement {
                        table: Table { name, .. },
                        fields,
                        ..
                    }) => {
            let columns = fields.into_iter()
                .map(|ColumnSpecification { column: Column { name, .. }, .. }| name)
                .collect();
            CreateTableData(TableSchema {name: name, columns: columns})
        }
        parsed => {
            ExtractedSql::Error(format!("Not an import statement: {:?}", parsed))
        }
    }
}

fn extract_target_string(mut values: Vec<Literal>, schema: TableSchema) -> Result<String, String> {
    if values.len() != schema.len {
        Err(format!("Bad number of inserted values: {:?}", values))
    } else {
        // TODO
        "TODO"
    }
}

fn single_error_iterator<T: 'static>(s: String) -> Box<Iterator<Item=Result<T, String>>> {
    Box::new(std::iter::once(Err(s)))
}

fn extract_urls_from_insert_data(
    data: Vec<Vec<Literal>>,
    target_index: usize,
) -> impl Iterator<Item=Result<String, String>> {
    data.into_iter()
        .map(move |v| extract_target_string(v, target_index))
}

fn is_comment(line_bytes: &Vec<u8>) -> bool {
    line_bytes.starts_with(b"--") ||
        line_bytes.starts_with(b"/*") ||
        line_bytes.is_empty()
}

fn is_complete_statement(statement: &Vec<u8>) -> bool {
    statement.ends_with(b";")
}

#[derive(Debug)]
struct ScanState {
    current_statement: Vec<u8>,
    schema: HashMap<TableName, TableColumns>,
}

enum ScanLineAction {
    Pass,
    ReportError(String),
    ExtractFrom(InsertData, TableSchema),
}

impl ScanState {
    fn new() -> ScanState {
        ScanState {
            current_statement: Vec::with_capacity(1_000_000),
            schema: HashMap::new(),
        }
    }

    fn add_line(&mut self, line_bytes: &mut Vec<u8>) -> ScanLineAction {
        if is_comment(line_bytes) {
            ScanLineAction::Pass
        } else {
            self.current_statement.append(line_bytes);
            if is_complete_statement(&self.current_statement) {
                let scan_result = self.scan_result();
                self.current_statement.clear();
                scan_result
            } else { ScanLineAction::Pass }
        }
    }

    fn scan_result(&mut self) -> ScanLineAction {
        let parsed_sql = parser::parse_query_bytes(&self.current_statement);
        match parsed_sql {
            Ok(sql) => match extract_data(sql) {
                InsertData(data) => {
                    ScanLineAction::ExtractFrom(data, &self.schema)
                },
                ExtractedSql::CreateTableData(index) => {
                    self.target_field = Some(index);
                    ScanLineAction::Pass
                },
                ExtractedSql::Error(err) => ScanLineAction::ReportError(err),
            },
            Err(s) => ScanLineAction::ReportError(format!("Unable to parse as SQL: '{}' ({})", 
                    std::str::from_utf8(&self.current_statement).unwrap_or("invalid utf8"), s))
        }
    }
}

fn scan_binary_lines(
    scan_state: &mut ScanState,
    mut line_result: Result<Vec<u8>, Error>,
) -> Option<Box<Iterator<Item=Result<String, String>>>> {
    match line_result {
        Ok(ref mut line_bytes) => {
            match scan_state.add_line(line_bytes) {
                ScanLineAction::ExtractFrom(data, i) => Some(Box::new(extract_urls_from_insert_data(data, i))),
                ScanLineAction::ReportError(s) => Some(single_error_iterator(s)),
                ScanLineAction::Pass => Some(Box::new(empty()))
            }
        }
        Err(err) => Some(single_error_iterator(format!("Unable to read line: {}", err)))
    }
}

pub fn iter_string_urls<T: BufRead>(input: T) -> impl Iterator<Item=Result<String, String>> {
    input.split(b'\n')
        .scan(ScanState::new(), scan_binary_lines)
        .flat_map(|urls| urls)
}
