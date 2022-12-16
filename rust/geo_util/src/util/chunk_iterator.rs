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


use gdal::raster::types::IntAlias;

// See the tests for examples

pub trait ChunkIteratorConstraits:  IntAlias
{
}

impl<T: IntAlias> ChunkIteratorConstraits for T {}

pub struct ChunkIterator<I>
where I: ChunkIteratorConstraits
{

    step_size: I,
    num_steps: usize,
    cur_step: usize,
    start: I,
    end: I,
}

impl <I> ChunkIterator<I>
where I: ChunkIteratorConstraits
{
    ///
    /// Inclusive [start, end]
    pub fn new(start: I, end: I, step_size: I) -> Self {

        assert!(end >= start);
        assert!(step_size >= I::one());

        let range_len = end-start;
        let mut num_steps = I::one() + range_len / step_size;
        if range_len % step_size > I::zero() {
            num_steps = num_steps + I::one()
        }

        ChunkIterator {
            step_size,
            start,
            end,
            num_steps: num_steps.to_usize().unwrap(),
            cur_step: 0
        }
    }

    fn current(&self) -> I {

        if self.cur_step >= self.num_steps - 1 {
            return self.end;
        }

        self.start + self.step_size * I::from_usize(self.cur_step).unwrap()
    }

    fn go_back(&mut self) {
        if self.cur_step > 0 {
            self.cur_step -= 1;
        }
    }

    pub fn reset(&mut self) {
        self.cur_step = 0;
    }
}

impl <I> Iterator for ChunkIterator<I>
where I: ChunkIteratorConstraits
{
    type Item = I;

    fn next(&mut self) -> Option<Self::Item> {



        if self.cur_step >= self.num_steps {
            return None;
        }

        let r = self.current();

        self.cur_step = self.cur_step + 1;

        Some(r)
    }



    fn size_hint(&self) -> (usize, Option<usize>) {
        let l = self.num_steps - self.cur_step;
        (l, Some(l))
    }

    fn count(self) -> usize {
        panic!("Should use len()")
    }
}


impl <I> ExactSizeIterator for ChunkIterator<I>
where I: ChunkIteratorConstraits
{

}

pub struct ChunkPairIterator<I>
where I: ChunkIteratorConstraits
{
    chunk_iter: ChunkIterator<I>,
}

impl <I> ChunkPairIterator<I>
where I: ChunkIteratorConstraits
{
    ///
    /// Start and end are inclusive
    pub fn new(start: I, end: I, step_size: I) -> Self {

        let chunk_iter = ChunkIterator::new(start, end+I::one(), step_size);

        //should always have at least 2 elements, because we added 1 to the end
        assert!(chunk_iter.size_hint().0 > 1);

        ChunkPairIterator {
            chunk_iter,
        }
    }

    /*
    fn peek(&mut self) -> Option<(I,I)> {
        if let Some(lower_bound_inc) = self.chunk_iter.next() {
            if let Some(upper_bound_inc) = self.chunk_iter.next() {
                self.chunk_iter.go_back();
                self.chunk_iter.go_back();
                return Some( (lower_bound_inc, upper_bound_inc - I::one()) );
            }
        }

        None
    }*/

    pub fn reset(&mut self) {
        self.chunk_iter.reset();
    }
}


impl <I> Iterator for ChunkPairIterator<I>
where I: ChunkIteratorConstraits
{
    type Item = (I, I);

    fn next(&mut self) -> Option<Self::Item> {

        if let Some(lower_bound_inc) = self.chunk_iter.next() {

            if let Some(upper_bound_inc) = self.chunk_iter.next() {
                self.chunk_iter.go_back();
                return Some( (lower_bound_inc, upper_bound_inc - I::one()) )
            }
        }

        None
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let sh = self.chunk_iter.size_hint();

        if sh.0 <= 1 {
            (0, Some(0))
        } else {
            (sh.0 - 1, Some(sh.0 - 1))
        }
    }
}

impl <I> ExactSizeIterator for ChunkPairIterator<I>
where I: ChunkIteratorConstraits
{

}

#[cfg(test)]
mod chunk_iterator_tests {

    use super::*;

    #[test]
    fn test_chunk_iter() {
        let v : Vec<_> = ChunkIterator::new(0, 10, 3).collect();
        assert_eq!(v, vec![0, 3, 6, 9, 10]);

        let v : Vec<_> = ChunkIterator::new(2, 11, 3).collect();
        assert_eq!(v, vec![2, 5, 8, 11]);

        //always includes the end
        let v : Vec<_> = ChunkIterator::new(4, 5, 30).collect();
        assert_eq!(v, vec![4, 5]);

        let v : Vec<_> = ChunkIterator::new(4, 4, 30).collect();
        assert_eq!(v, vec![4]);

        let v : Vec<_> = ChunkIterator::new(4, 4, 1).collect();
        assert_eq!(v, vec![4]);

        let v : Vec<_> = ChunkIterator::new(0, 54, 10).collect();
        assert_eq!(v, vec![0, 10, 20, 30, 40, 50, 54]);
    }

    #[test]
    fn test_chunk_iter_size() {
        for (start, stop, step_size) in vec![
            (0,10,3),
            (2,11,3),
            (4,5,30),
            (4,4,30),
            (4,4,1),
            (0, 54,10),
        ].into_iter() {
            let len = ChunkIterator::new(start,stop, step_size).len();

            let mut it = ChunkIterator::new(start,stop, step_size);
            for i in 0..len {
                assert_eq!(len -i, it.size_hint().0);
                assert_eq!(Some(len -i), it.size_hint().1);

                it.next();
            }

            assert_eq!(None, it.next());
        }

    }

    #[test]
    fn test_pair_chunk_iter() {
        let v : Vec<_> = ChunkPairIterator::new(0, 10, 3).collect();
        assert_eq!(v, vec![(0, 2), (3, 5), (6, 8), (9, 10)]);

        let v : Vec<_> = ChunkPairIterator::new(2, 11, 3).collect();
        assert_eq!(v, vec![(2, 4), (5, 7), (8, 10), (11, 11)]);

        let v : Vec<_> = ChunkPairIterator::new(2, 10, 3).collect();
        assert_eq!(v, vec![(2, 4), (5, 7), (8, 10)]);

        let v : Vec<_> = ChunkPairIterator::new(3, 5, 30).collect();
        assert_eq!(v, vec![(3, 5)]);

        let v : Vec<_> = ChunkPairIterator::new(3, 5, 1).collect();
        assert_eq!(v, vec![(3, 3), (4, 4), (5, 5)]);

        let v : Vec<_> = ChunkPairIterator::new(3, 5, 2).collect();
        assert_eq!(v, vec![(3, 4), (5, 5)]);

        let v : Vec<_> = ChunkPairIterator::new(4, 4, 30).collect();
        assert_eq!(v, vec![(4, 4)]);

        let v : Vec<_> = ChunkPairIterator::new(4, 4, 1).collect();
        assert_eq!(v, vec![(4, 4)]);

        let v : Vec<_> = ChunkPairIterator::new(0, 54, 10).collect();
        assert_eq!(v, vec![(0, 9), (10, 19), (20, 29), (30, 39), (40, 49), (50, 54)]);
    }
}