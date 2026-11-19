use std::{io::Read, str::FromStr};

use csv::{ReaderBuilder, StringRecord, Trim};

use crate::{
    compiler::{Error, Result},
    models::{Date, Time},
};

/// A row that lenient mode left out of the output.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkippedRow {
    pub file: &'static str,
    /// Why the row was left out, naming its line, column and value.
    pub message: String,
}

/// How many skipped rows are kept as text. The rest are only counted.
const MAX_REPORTED_SKIPS: usize = 100;

/// A csv file read one row at a time, with columns found by header name.
///
/// After every call, `skipped` holds the first rows that were left out and `skipped_total` counts
/// all of them, so the first is never longer than the second. Rows are only left out when the
/// table is lenient and the row had a bad value.
pub(super) struct Table<'a> {
    file: &'static str,
    lenient: bool,
    skipped: Vec<SkippedRow>,
    skipped_total: usize,
    reader: csv::Reader<Box<dyn Read + 'a>>,
    headers: Vec<String>,
    record: StringRecord,
}

impl<'a> Table<'a> {
    pub(super) fn new(
        file: &'static str,
        input: Box<dyn Read + 'a>,
        lenient: bool,
    ) -> Result<Self> {
        let mut reader = ReaderBuilder::new()
            .flexible(true)
            .trim(Trim::All)
            .from_reader(input);
        let headers = reader
            .headers()
            .map_err(|source| Error::Csv { file, source })?
            .iter()
            // Some publishers start the file with a byte order mark.
            .map(|header| {
                header
                    .trim_start_matches('\u{feff}')
                    .trim()
                    .to_ascii_lowercase()
            })
            .collect();
        Ok(Self {
            file,
            lenient,
            skipped: Vec::new(),
            skipped_total: 0,
            reader,
            headers,
            record: StringRecord::new(),
        })
    }

    /// Returns the position of an optional column.
    pub(super) fn column(&self, name: &str) -> Option<usize> {
        self.headers.iter().position(|header| header == name)
    }

    /// Returns the position of a column the file must have.
    pub(super) fn require(&self, name: &'static str) -> Result<usize> {
        self.column(name).ok_or(Error::MissingColumn {
            file: self.file,
            column: name,
        })
    }

    /// Passes on a parsed row, or in lenient mode drops one with a bad value and remembers why.
    ///
    /// Errors that aren't about one row, such as a missing column, are never dropped.
    pub(super) fn accept<T>(&mut self, parsed: Result<T>) -> Result<Option<T>> {
        match parsed {
            Ok(value) => Ok(Some(value)),
            Err(error @ Error::InvalidValue { .. }) if self.lenient => {
                self.skipped_total += 1;
                if self.skipped.len() < MAX_REPORTED_SKIPS {
                    self.skipped.push(SkippedRow {
                        file: self.file,
                        message: error.to_string(),
                    });
                }
                Ok(None)
            }
            Err(error) => Err(error),
        }
    }

    /// Returns the rows that were skipped, and how many there were in all.
    pub(super) fn take_skipped(&mut self) -> (Vec<SkippedRow>, usize) {
        (std::mem::take(&mut self.skipped), self.skipped_total)
    }

    /// Reads the next row, or returns `None` at the end of the file.
    pub(super) fn next_row(&mut self) -> Result<Option<Row<'_>>> {
        let file = self.file;
        let has_row = self
            .reader
            .read_record(&mut self.record)
            .map_err(|source| Error::Csv { file, source })?;
        if !has_row {
            return Ok(None);
        }
        let line = self.record.position().map_or(0, csv::Position::line);
        Ok(Some(Row {
            file,
            line,
            record: &self.record,
        }))
    }
}

/// One row of a `Table`.
pub(super) struct Row<'a> {
    file: &'static str,
    line: u64,
    record: &'a StringRecord,
}

impl<'a> Row<'a> {
    /// Returns the field, or `None` if the column is missing or the field is empty.
    pub(super) fn text(&self, column: Option<usize>) -> Option<&'a str> {
        let value = self.record.get(column?)?;
        (!value.is_empty()).then_some(value)
    }

    /// Returns a field that must not be empty.
    pub(super) fn required(&self, column: usize, name: &'static str) -> Result<&'a str> {
        self.text(Some(column))
            .ok_or_else(|| self.invalid(name, ""))
    }

    /// Parses an optional field. An empty field is `None`, and a field that doesn't parse is an
    /// error.
    pub(super) fn parse<T: FromStr>(
        &self,
        column: Option<usize>,
        name: &'static str,
    ) -> Result<Option<T>> {
        let Some(value) = self.text(column) else {
            return Ok(None);
        };
        value
            .parse()
            .map(Some)
            .map_err(|_| self.invalid(name, value))
    }

    /// Parses a field that must be there.
    pub(super) fn parse_required<T: FromStr>(
        &self,
        column: usize,
        name: &'static str,
    ) -> Result<T> {
        let value = self.required(column, name)?;
        value.parse().map_err(|_| self.invalid(name, value))
    }

    /// Parses an optional decimal number, which must be finite.
    pub(super) fn float(&self, column: Option<usize>, name: &'static str) -> Result<Option<f64>> {
        let Some(value) = self.parse::<f64>(column, name)? else {
            return Ok(None);
        };
        if value.is_finite() {
            Ok(Some(value))
        } else {
            Err(self.invalid(name, self.text(column).unwrap_or_default()))
        }
    }

    /// Parses a decimal number that must be there and be finite.
    pub(super) fn float_required(&self, column: usize, name: &'static str) -> Result<f64> {
        self.float(Some(column), name)?
            .ok_or_else(|| self.invalid(name, ""))
    }

    /// Parses a `0` or `1` flag.
    pub(super) fn flag(&self, column: usize, name: &'static str) -> Result<bool> {
        match self.required(column, name)? {
            "0" => Ok(false),
            "1" => Ok(true),
            other => Err(self.invalid(name, other)),
        }
    }

    pub(super) fn date(&self, column: usize, name: &'static str) -> Result<Date> {
        let value = self.required(column, name)?;
        Date::parse_gtfs(value).ok_or_else(|| self.invalid(name, value))
    }

    /// Parses an optional `H:MM:SS` time.
    pub(super) fn time(&self, column: Option<usize>, name: &'static str) -> Result<Option<Time>> {
        let Some(value) = self.text(column) else {
            return Ok(None);
        };
        Time::from_hms(value)
            .map(Some)
            .ok_or_else(|| self.invalid(name, value))
    }

    /// Builds the error for a bad value in this row.
    pub(super) fn invalid(&self, column: &'static str, value: &str) -> Error {
        Error::InvalidValue {
            file: self.file,
            line: self.line,
            column,
            value: value.to_string(),
        }
    }
}
