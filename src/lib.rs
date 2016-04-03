
use std::collections::{BTreeSet};
use std::collections::VecDeque;

pub struct Replace <'a, I, T: 'a + Ord > {
    iter: I,
    buffer_out: VecDeque<T>,
    buffer_in: VecDeque<T>,
    replace_from: &'a [T],
    replace_with: &'a [T],
    candidates: BTreeSet<usize>,
    index: usize,
    flushed_index: usize,
}

impl <'a, I, T> Replace <'a, I, T> where
    I: Iterator<Item = T>,
    T: Eq + Ord + Copy {
    pub fn adapt(iter: I, replace_from: &'a [T], replace_with: &'a [T]) -> Replace<'a, I, T> {
        Replace {
            iter: iter,
            buffer_out: VecDeque::new(),
            buffer_in: VecDeque::new(),
            replace_from: replace_from,
            replace_with: replace_with,
            candidates: BTreeSet::new(),
            index: 0,
            flushed_index: 0,
        }
    }

    fn fill_buffer(&mut self) {
        'consume: while let Some(item) = self.iter.next() {
            self.index += 1;
            // buffer all incoming items
            self.buffer_in.push_back(item);
            // Prune existing partial match candidates that don't match the next item
            let removes: Vec<_> = self.candidates.iter().cloned()
                .filter(|start_index| {
                    self.replace_from[self.index - *start_index] != item
                }).collect();
            for r in removes {
                self.candidates.remove(&r);
            }
            // Keep track of new partial match candidates
            if self.replace_from[0] == item {
                self.candidates.insert(self.index);
            }
            // if the length of the first match is the length of the replace sequence then it's a complete match
            match self.candidates.iter().cloned()
                .next()
                .into_iter()
                .find(|x| self.index - x + 1 == self.replace_from.len()) {
                    None => {
                        // We can flush the inbound buffer up to the first partial match
                        // (or the full buffer if there are no partial matches)
                        let flush_index = self.candidates.iter().next().map(|x| x - 1).unwrap_or(self.index);
                        if flush_index > self.flushed_index {
                            let mut flush: VecDeque<_> = self.buffer_in.drain(0 .. flush_index - self.flushed_index).collect();
                            self.buffer_out.append(&mut flush);
                            self.flushed_index = flush_index;
                            break 'consume;
                        }
                    },
                    Some(_) => {
                        // A match! So replace it and clear all the partial matches
                        self.candidates.clear();
                        for &x in self.replace_with.iter() {
                            self.buffer_out.push_back(x);
                        }
                        self.buffer_in.clear();
                        self.flushed_index = self.index;
                        break 'consume;
                    }
                }
        }
    }

}


pub trait ReplaceIter<'a, I, T> where
    I: Iterator<Item = T>,
    T: Ord {

    // fn replace_iter(self, map: BTreeMap<&'a [T], &'a [T]>) -> Replace<'a, I, T>;
    fn replace(self, from: &'a [T], to: &'a [T]) -> Replace<'a, I, T>;
}

impl <'a, I, T> ReplaceIter<'a, I, T> for I where
    I: Iterator<Item = T>,
    T: Eq + Ord + Copy {

    // fn replace_iter(self, map: BTreeMap<&'a [T], &'a [T]>) -> Replace<'a, I, T> {
    fn replace(self, from: &'a [T], to: &'a [T]) -> Replace<'a, I, T> {
        Replace::adapt(self, from, to)
    }
}


impl <'a, I, T> Iterator for Replace <'a, I, T> where
    I: Iterator<Item = T>,
    T: Eq + Ord + Copy {

    type Item = T;

    fn next(&mut self) -> Option<T> {
        if self.buffer_out.len() == 0 {
            self.fill_buffer();
        }
        self.buffer_out.pop_front()
    }

}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    pub fn test_replace_simple() {
        let v: Vec<u32> = vec![1,2,3].into_iter().replace(&[2], &[10]).collect();
        assert_eq!(v, vec![1,10,3]);
    }

    #[test]
    pub fn test_replace_longer() {
        let v: Vec<u32> = vec![3,4,5,6,7,8,9].into_iter().replace(&[4,5], &[100]).collect();
        assert_eq!(v, vec![3,100,6,7,8,9]);
    }

    #[test]
    pub fn test_replace_multi() {
        let v: Vec<u32> = vec![3,4,5,6,4,5,9].into_iter().replace(&[4,5], &[100,200,300]).collect();
        assert_eq!(v, vec![3,100,200,300,6,100,200,300,9]);
    }

    #[test]
    pub fn test_nearly_match() {
        let v: Vec<u32> = vec![3,4,5,6].into_iter().replace(&[4,5,1], &[100,200]).collect();
        assert_eq!(v, vec![3,4,5,6]);
    }

    #[test]
    pub fn test_replace_overlapping() {
        let v: Vec<u32> = vec![3,4,5,4,5,4,9].into_iter().replace(&[4,5,4,5], &[100]).collect();
        assert_eq!(v, vec![3,100,4,9]);
    }
}
