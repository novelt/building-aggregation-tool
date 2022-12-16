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
use crate::util::{ChunkPairIterator, ChunkIteratorConstraits};

pub struct RasterChunkIterator<I>
where I: ChunkIteratorConstraits
{
    num_steps: usize,

    x_pair_it: ChunkPairIterator<I>,
    y_pair_it: ChunkPairIterator<I>,
    num_col_chunks: usize,
    cur_step: usize,

    current_y_pair: Option< (I, I) >
}

#[derive(Debug)]
pub struct RasterChunkIteratorItem<I>
where I: ChunkIteratorConstraits
{
    //X, Y  or column, row order
    pub window_size: (I, I),

    pub window_offset: (I, I),

    pub x_range_inclusive: (I, I),
    pub y_range_inclusive: (I, I),

    pub current_step: usize,
    pub num_steps: usize
}

impl <I> RasterChunkIterator<I>
//No implied bounds, could use a parent trait to DRY this
where I: ChunkIteratorConstraits
{
    pub fn new(n_rows: I, n_cols: I, n_chunks: I) -> Self {

        assert!(n_rows > I::zero());
        assert!(n_cols > I::zero());
        assert!(n_chunks > I::zero());

        let y_it: ChunkPairIterator<I> = ChunkPairIterator::new(I::zero(), n_rows - I::one(),
                                                                              n_rows.div_ceil(&n_chunks));
        let x_it = ChunkPairIterator::new(I::zero(), n_cols - I::one(),
                                                                                  n_cols.div_ceil(&n_chunks));
        let x_it_len = x_it.len();

        Self {
            num_steps: y_it.len() * x_it.len(),

            x_pair_it: x_it,
            y_pair_it: y_it,
            num_col_chunks: x_it_len,

            cur_step: 0,

            current_y_pair: None
        }
    }
}

impl <I> Iterator for RasterChunkIterator<I>
where I: ChunkIteratorConstraits
{
    type Item = RasterChunkIteratorItem<I>;

    fn next(&mut self) -> Option<Self::Item> {

        if self.cur_step >= self.num_steps {
            return None;
        }

        let chunk_col = self.cur_step % self.num_col_chunks;
        //let chunk_row = self.cur_step / self.num_col_chunks;

        if chunk_col == 0 {
            self.current_y_pair = self.y_pair_it.next()
        }

        //println!("AAA {} {} {} {}", self.cur_step, self.num_steps,self.num_col_chunks, chunk_col);

        let y_val = self.current_y_pair.unwrap();
        let x_val = self.x_pair_it.next().unwrap();

        if chunk_col == self.num_col_chunks - 1 {
            self.x_pair_it.reset();
        }

        //println!("BBB {:?}  {:?}", x_val, y_val);

        let window_size = (I::one() + x_val.1 - x_val.0, I::one() + y_val.1 - y_val.0);
        let window_offset = (x_val.0,
                             y_val.0);

        let r = Some( RasterChunkIteratorItem {
            window_size,
            window_offset,
            x_range_inclusive: x_val,
            y_range_inclusive: y_val,
            current_step: self.cur_step,
            num_steps: self.num_steps
        });

        self.cur_step += 1;

        r
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let l = self.num_steps - self.cur_step;
        (l, Some(l))
    }
}


impl <I> ExactSizeIterator for RasterChunkIterator<I>
where I: ChunkIteratorConstraits
{

}


#[cfg(test)]
mod raster_window_iterator_tests {
    use super::*;

    #[test]
    fn test_window_iter_4chunks() {
        let mut it = RasterChunkIterator::<u16>::new(5, 5, 2);
        assert_eq!(4, it.len());

        let item = it.next().unwrap();

        assert_eq!( (3,3), item.window_size);
        assert_eq!( (0,0), item.window_offset);

        let item = it.next().unwrap();

        assert_eq!( (2,3), item.window_size);
        assert_eq!( (3,0), item.window_offset);

        let item = it.next().unwrap();

        assert_eq!( (3,2), item.window_size);
        assert_eq!( (0,3), item.window_offset);

        let item = it.next().unwrap();

        assert_eq!( (2,2), item.window_size);
        assert_eq!( (3,3), item.window_offset);

        assert!(it.next().is_none());
    }

    #[test]
    fn test_window_iter_1chunks() {
        let mut it = RasterChunkIterator::<u16>::new(52, 15, 1);
        assert_eq!(1, it.len());

        let item = it.next().unwrap();

        assert_eq!( (15,52), item.window_size);
        assert_eq!( (0,0), item.window_offset);

        assert!(it.next().is_none());
    }

    #[test]
    fn test_window_iter_perfect_rectange() {
        let mut it = RasterChunkIterator::<u16>::new(8, 4, 2);
        assert_eq!(4, it.len());

        let item = it.next().unwrap();

        assert_eq!( (2,4), item.window_size);
        assert_eq!( (0,0), item.window_offset);

        let item = it.next().unwrap();

        assert_eq!( (2,4), item.window_size);
        assert_eq!( (2,0), item.window_offset);

        let item = it.next().unwrap();

        assert_eq!( (2,4), item.window_size);
        assert_eq!( (0,4), item.window_offset);

        let item = it.next().unwrap();

        assert_eq!( (2,4), item.window_size);
        assert_eq!( (2,4), item.window_offset);


        assert!(it.next().is_none());
    }

    #[test]
    fn test_window_iter_many_chunks() {
        let mut it = RasterChunkIterator::<u16>::new(103, 112, 10);
        assert_eq!(100, it.len());

        let item = it.next().unwrap();

        assert_eq!( (12,11), item.window_size);
        assert_eq!( (0,0), item.window_offset);


    }
}