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
use crate::utils::_string;
use gdal_sys;
use std::ffi::CString;

pub fn version_info(key: &str) -> String {
    let c_key = CString::new(key.as_bytes()).unwrap();
    _string(unsafe { gdal_sys::GDALVersionInfo(c_key.as_ptr()) })
}

#[cfg(test)]
mod tests {
    use super::version_info;

    #[test]
    fn test_version_info() {
        let release_date = version_info("RELEASE_DATE");
        let release_name = version_info("RELEASE_NAME");
        let version_text = version_info("--version");

        let mut date_iter = release_date.chars();

        let expected_text: String = format!(
            "GDAL {}, released {}/{}/{}",
            release_name,
            date_iter.by_ref().take(4).collect::<String>(),
            date_iter.by_ref().take(2).collect::<String>(),
            date_iter.by_ref().take(2).collect::<String>(),
        );

        assert_eq!(version_text, expected_text);
    }
}
