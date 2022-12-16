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
use rstar::{AABB, PointDistance, Envelope, RTreeObject};

pub type Coord  = f64;

#[derive(Eq, PartialEq, Hash, Clone, Copy)]
pub struct FeatureKey {
    pub fid: i64,
    pub layer_idx: u8
}

//#[derive(Deserialize, Serialize, Clone)]
#[derive(Clone)]
pub struct RTreeIndexObject {
    pub feature_key: FeatureKey,
    pub envelope: AABB<[Coord; 2]>,
}

/// Implement this to support nearest neighbor calculations
impl PointDistance for RTreeIndexObject {
    /// For speed, use the distance of the center of the envelope to the point
    fn distance_2(
        &self,
        rhs: &[Coord; 2]) -> Coord {
        let center = self.envelope.center();

        // Vector distance in lat/lon
        return center.distance_2(rhs);
    }

    // This implementation is not required but more efficient since it
    // omits the calculation of a square root
    fn contains_point(&self, point: &[Coord; 2]) -> bool
    {
        self.envelope.contains_point(point)
    }
}

/// Rstar requires this implementation to know how to index it
impl RTreeObject for RTreeIndexObject {
    type Envelope = AABB<[Coord; 2]>;

    fn envelope(&self) -> Self::Envelope {
        self.envelope
    }
}

impl PartialEq for RTreeIndexObject {
  fn eq(&self, other: &Self) -> bool {
      self.feature_key == other.feature_key
  }
}
impl Eq for RTreeIndexObject {}