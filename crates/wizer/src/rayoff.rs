use std::ops::Range;

pub trait ParallelIterator: Iterator {}

impl<T: Iterator> ParallelIterator for T {}

pub trait IntoParallelIterator {
    type Item;
    type Iter: Iterator<Item = Self::Item> + ParallelIterator;

    fn into_par_iter(self) -> Self::Iter;
}

impl IntoParallelIterator for Range<usize> {
    type Item = usize;
    type Iter = Range<usize>;

    fn into_par_iter(self) -> Self::Iter {
        self
    }
}

pub trait ParallelExtend<T> {
    fn par_extend<I>(&mut self, par_iter: I)
    where
        I: IntoIterator<Item = T>;
}

impl<T> ParallelExtend<T> for Vec<T> {
    fn par_extend<I>(&mut self, par_iter: I)
    where
        I: IntoIterator<Item = T>,
    {
        self.extend(par_iter);
    }
}
