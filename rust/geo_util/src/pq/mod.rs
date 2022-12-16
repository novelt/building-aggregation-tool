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
use pq_sys::*;
use std::ffi::{CString, CStr};
use anyhow::{bail, Result};
use std::ptr::null;
use chrono::{NaiveDate, DateTime, FixedOffset, TimeZone};
use log::debug;
use bitvec::prelude::*;

///Copies corrected settlements to database
/// Uses libpq directly and copy in binary format

pub struct PgConnection {
    c_handle: *mut PGconn,
    postgres_epoch_date: NaiveDate,
    postgres_epoch_datetime: DateTime<FixedOffset>
}

pub struct PgResult {
    result: *mut PGresult
}

const HEADER_MAGIC: &'static [u8] = b"PGCOPY\n\xff\r\n\0";

impl PgResult
{
    pub fn new_result(pg_conn: &PgConnection) -> Self
    {
        unsafe {
            Self {
                result: PQgetResult(pg_conn.c_handle)
            }
        }
    }
    pub fn status(&self) -> ExecStatusType {
        unsafe {
            PQresultStatus(self.result)
        }
    }
    pub fn status_str(&self) -> &str {
        unsafe {
            let status = PQresStatus(self.status());
            let c_str = CStr::from_ptr(status);
            c_str.to_str().unwrap()
        }
    }
    pub fn error_message(&self) -> &str {
        unsafe {
            let err_msg = PQresultErrorMessage(self.result);
            let c_str = CStr::from_ptr(err_msg);
            c_str.to_str().unwrap()
        }
    }

    pub fn is_ok(&self) -> Result<()>
    {
         if self.status() != ExecStatusType::PGRES_COMMAND_OK && self.status() != ExecStatusType::PGRES_TUPLES_OK {
             bail!("Statement is not OK -- Error: {} Status: {}", self.error_message(), self.status_str());
         }

        Ok(())
    }
}

impl Drop for PgResult
{
    fn drop(&mut self) {
        unsafe {
            PQclear(self.result);
        }
    }
}

impl PgConnection {
    pub fn new(conn_str: &str) -> Result<Self> {
        unsafe {
            let c_str = CString::new(conn_str)?;
            let c_handle = PQconnectdb(c_str.as_ptr());
            let stat = PQstatus(c_handle);
            match stat {
                _bindgen_ty_2::CONNECTION_OK => {}
                _bindgen_ty_2::CONNECTION_BAD => {
                    bail!("Connection bad");
                }
                _bindgen_ty_2::CONNECTION_STARTED => {}
                _bindgen_ty_2::CONNECTION_MADE => {}
                _bindgen_ty_2::CONNECTION_AWAITING_RESPONSE => {}
                _bindgen_ty_2::CONNECTION_AUTH_OK => {}
                _bindgen_ty_2::CONNECTION_SETENV => {}
                _bindgen_ty_2::CONNECTION_SSL_STARTUP => {}
                _bindgen_ty_2::CONNECTION_NEEDED => {}
            }

            let server_version = PQserverVersion(c_handle);

            debug!("Connection status: {:?} Server Version: {}", stat, server_version);

            Ok(Self {
                c_handle,
                postgres_epoch_date: NaiveDate::from_ymd(2000, 1, 1),
                postgres_epoch_datetime: FixedOffset::east(0).ymd(2000, 1, 1)
                    .and_hms(0,0,0)
            })
        }
    }

    pub fn status(&self) -> ConnStatusType {
        unsafe {
            let stat = PQstatus(self.c_handle);
            stat
        }
    }

    pub fn last_error(&self) -> &str {
        unsafe {
            let err_msg = PQerrorMessage(self.c_handle);
            let cstr = CStr::from_ptr(err_msg);
            cstr.to_str().unwrap()
        }
    }

    pub fn execute(&self, query: &str) -> PgResult
    {
        unsafe {
            let query_cstr = CString::new(query).unwrap();
            let result = PQexec(self.c_handle, query_cstr.as_ptr());

            PgResult {
                result
            }
        }
    }

    pub fn copy_start(&self, copy_sql: &str) -> Result<()>
    {
        let copy_result = self.execute(&copy_sql);

        if copy_result.status() != ExecStatusType::PGRES_COPY_IN {
            bail!("Error in copy: {}", self.last_error());
        }

        self.copy_data(HEADER_MAGIC)?;
        self.copy_data(&0i32.to_be_bytes())?;
        self.copy_data(&0i32.to_be_bytes())

    }

    pub fn copy_data(&self, data: &[u8]) -> Result<()>
    {
        unsafe {
            let status = PQputCopyData(self.c_handle, data.as_ptr() as * const i8, data.len() as i32);

            if status < 0 {
                bail!("Error in copy: {}", self.last_error());
            }
            if status == 0 {
                bail!("wait for write ready, try again");
            }
            //println!("Status in copy is {}", status);

            Ok(())
        }
    }

    pub fn copy_field_count(&self, field_count: u16) -> Result<()>
    {
        self.copy_data(&field_count.to_be_bytes())
    }

    pub fn copy_smallint(&self, data: i16) -> Result<()>
    {
        self.copy_data(&2i32.to_be_bytes())?;

        self.copy_data(&data.to_be_bytes())
    }

    pub fn copy_int(&self, data: i32) -> Result<()>
    {
        self.copy_data(&4i32.to_be_bytes())?;

        self.copy_data(&data.to_be_bytes())
    }

    pub fn copy_opt_int(&self, data: Option<i32>) -> Result<()>
    {
        if let Some(d) = data {
            self.copy_int(d)
        } else {
            self.copy_null()
        }
    }


    pub fn copy_bigint(&self, data: i64) -> Result<()>
    {
        self.copy_data(&8i32.to_be_bytes())?;

        self.copy_data(&data.to_be_bytes())
    }

    pub fn copy_boolean(&self, data: bool) -> Result<()>
    {
        self.copy_data(&1i32.to_be_bytes())?;

        let a : u8 = if data {1} else {0};
        self.copy_data(&a.to_be_bytes())
    }

    pub fn copy_f32(&self, data: f32) -> Result<()>
    {
        self.copy_data(&4i32.to_be_bytes())?;

        self.copy_data(&data.to_be_bytes())
    }

    pub fn copy_f64(&self, data: f64) -> Result<()>
    {
        self.copy_data(&8i32.to_be_bytes())?;

        self.copy_data(&data.to_be_bytes())
    }

    pub fn copy_date(&self, date: NaiveDate) -> Result<()>
    {
        self.copy_data(&4i32.to_be_bytes())?;

        let days_since_epoch = date - self.postgres_epoch_date;

        //println!("Days since epoch between {} and {} is {}", date, self.postgres_epoch_date, days_since_epoch.num_days());

        self.copy_data(&(days_since_epoch.num_days() as i32).to_be_bytes() )
    }

    pub fn copy_timestamp(&self, date: DateTime<FixedOffset>) -> Result<()>
    {
        self.copy_data(&8i32.to_be_bytes())?;

        let secs_since_epoch_duration = date - self.postgres_epoch_datetime;

        let micro_secs_since_epoch = secs_since_epoch_duration.num_microseconds().unwrap();

        //println!("Secs since epoch {} and {}", 494760779.9999999, micro_secs_since_epoch);

        self.copy_data(&micro_secs_since_epoch.to_be_bytes() )
    }

    pub fn copy_null(&self) -> Result<()> {
        let neg_one: i32 = -1;
        self.copy_data(&neg_one.to_be_bytes())
    }

    pub fn copy_str(&self, data: &str) -> Result<()>
    {
        let str_bytes = data.as_bytes();
        self.copy_bytes(str_bytes)
    }

    pub fn copy_str_empty_null(&self, data: &str) -> Result<()>
    {
        if data.is_empty() {
            self.copy_null()
        } else {
            self.copy_str(data)
        }
    }

    pub fn copy_bytes(&self, data: &[u8]) -> Result<()>
    {
        self.copy_data(&(data.len() as i32).to_be_bytes())?;

        self.copy_data(data)
    }

    pub fn copy_bits(&self, data: &BitVec<u8, Msb0>) -> Result<()>
    {
        let slice: &[u8] = data.as_raw_slice();
        let total_bytes = 4 + slice.len();

        self.copy_data(&(total_bytes as i32).to_be_bytes())?;

        //being bit varying, we need the number of bits, not bytes
        self.copy_data(&(data.len() as i32).to_be_bytes())?;

        self.copy_data(slice)
    }

    //Copies as bit varying

    pub fn copy_end(&self) -> Result<PgResult> {
        unsafe {
            //signal the end of the records
            //file trailer
            self.copy_data(&(-1i16).to_be_bytes())?;

            PQputCopyEnd(self.c_handle, null());

            Ok(PgResult::new_result(self))
        }
    }
}

impl Drop for PgConnection {
    fn drop(&mut self) {
        debug!("Dropping connection");
        unsafe {
            PQfinish(self.c_handle);
        }
    }

}