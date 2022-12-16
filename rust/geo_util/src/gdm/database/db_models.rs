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
pub mod data_work {


    table! {
        data_work.neighborhood_types (neighborhood_type_id) {
            name -> Nullable<Varchar>,
            neighborhood_type_id -> Int4,
            avg_closest_bldg_m -> Nullable<Float4>,
            avg_building_area_m2 -> Nullable<Float4>,
        }
    }

    allow_tables_to_appear_in_same_query!(
        neighborhood_types,
    );

    #[derive(Queryable, Debug, Identifiable, AsChangeset, Default)]
    #[primary_key(neighborhood_type_id)]
    #[table_name = "neighborhood_types"]
    pub struct NeighborhoodTypes {
        pub name: Option<String>,
        pub neighborhood_type_id: i32,
        pub avg_closest_bldg_m: Option<f32>,
        pub avg_building_area_m2: Option<f32>,
    }
}