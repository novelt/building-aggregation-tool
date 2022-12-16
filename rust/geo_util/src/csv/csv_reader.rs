/*
This file is part of the Building Aggregration Tool
Copyright (C) 2022 Novel-T

The Building Aggregration Tool is free software: you can redistribute it and/or modify
it under the terms of the GNU General Public License as published by
the Free Software Foundation, either version 3 of the License, or
(at your option) any later version.

This program is distributed in the hope that it will be useful,
but WITHOUT ANY WARRANTY; without even the implied warranty of
MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
GNU General Public License for more details.

You should have received a copy of the GNU General Public License
along with this program.  If not, see <http://www.gnu.org/licenses/>.
*/
use std::io::{BufReader, SeekFrom, Seek, Read};
use std::fs::File;
use std::path::Path;
use csv_core::{ReadFieldResult, Reader, ReaderBuilder};
use anyhow::Result;


const INPUT_BUFFER_SIZE: usize = 1024;

/// A csv reader that can read a line starting from a byte offset
pub struct CsvReader {
    csv_reader: Reader,
    input_buffer: [u8; INPUT_BUFFER_SIZE],
    output_buffer: [u8; INPUT_BUFFER_SIZE],
    csv_file_reader: BufReader<File>,
}

impl CsvReader {
    pub fn new(csv_path: &Path) -> Self {
        let csv_file_reader = BufReader::new(File::open(csv_path.to_str().unwrap().to_string()).unwrap());
        let csv_reader = ReaderBuilder::new().build();
        let input_buffer = [0; INPUT_BUFFER_SIZE];
        let output_buffer = [0; INPUT_BUFFER_SIZE];

        CsvReader {
            csv_reader,
            input_buffer,
            output_buffer,
            csv_file_reader,
        }
    }

    pub fn read_fields_for_line(&mut self, csv_offset: u64) -> Result<Vec<String>> {
        self.csv_file_reader.seek(SeekFrom::Start(csv_offset))?;
        let _bytes_read = self.csv_file_reader.read(&mut self.input_buffer)?;

        //println!("Read {} bytes from offset {}", bytes_read, csv_offset);

        let mut fields = Vec::new();
        //let mut count_records = 0;
        let mut total_input_bytes_read = 0;
        loop {
            // We skip handling the output since we don't need it for counting.
            let (result, nin, nout) =
                self.csv_reader.read_field(&self.input_buffer[total_input_bytes_read..],
                                           &mut self.output_buffer);
            //println!("Read result {:?} input bytes read {} output bytes written {}", result, nin, nout);

            total_input_bytes_read += nin;
            match result {
                ReadFieldResult::InputEmpty => panic!("input empty"),
                ReadFieldResult::OutputFull => panic!("field too large"),
                ReadFieldResult::Field { record_end } => {
                    let field_str = std::str::from_utf8(&self.output_buffer[..nout])?;
                    //println!("Field {}: {}", fields.len(), field_str);
                    fields.push(field_str.to_string());

                    if record_end {
                        //count_records += 1;
                        break;
                    }
                }
                ReadFieldResult::End => break,
            }
        }

        Ok(fields)
    }
}